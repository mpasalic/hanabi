use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, ScrollDirection},
};
use ratatui::{style::Stylize, widgets::WidgetRef, Terminal};

use std::{
    char::from_digit,
    collections::HashMap,
    error::Error,
    iter::{self},
};

use taffy::{
    style_helpers::{length, percent},
    JustifyContent, Overflow, Point,
};

use crate::{
    components::*,
    key_code::KeyCode,
    nodes::{
        GridStack, HStack, LayoutRect, LayoutSize, LayoutStyle, Node, NodeBuilder, Stack, VStack,
    },
};
use shared::client_logic::*;
use shared::model::*;
use shared::model::{ClientPlayerView, GameStateSnapshot};

type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone)]
pub enum HanabiClient {
    Connecting,
    Loaded(HanabiGame),
}

#[derive(Debug, Clone)]
pub struct HanabiApp {
    pub exit: bool,
    command: CommandState,
    pub client_state: HanabiClient,
    game_log_scroll_adjust: usize,
    game_state_selection: usize,
    hint_mode: HintMode,
    card_focus: Option<(PlayerIndex, usize)>,
}

pub enum EventHandlerResult {
    PlayerAction(PlayerAction),
    Quit,
    Continue,
    Start,
}

fn default_style() -> Style {
    Style::default().fg(NORMAL_TEXT).bg(BACKGROUND_COLOR)
}

fn root_tree_widget(area: Rect, child: Node<'static>) -> Node<'static> {
    use taffy::prelude::*;
    let mut tree = Node::new_flex(LayoutStyle {
        size: Size {
            width: length(area.width as f32),
            height: length(area.height as f32),
        },
        ..VStack::default_layout()
    })
    .debug("root")
    .child(child.append_layout(|l| LayoutStyle {
        size: Size {
            width: length(area.width),
            height: length(area.height),
        },
        max_size: Size {
            width: length(area.width),
            height: length(area.height),
        },
        ..l
    }));

    tree.compute_layout(Size {
        width: length(area.width),
        height: length(area.height),
    });
    // tree.print_tree();

    tree
}

#[derive(Debug, Clone)]
pub enum Binding<Action> {
    Keyboard {
        key_code: KeyCode,
        action: Action,
    },
    MouseClick {
        action: Action,
        click_rect: Rect,
    },
    Scroll {
        direction: ScrollDirection,
        action: Action,
        scroll_rect: Rect,
    },
}

impl HanabiApp {
    pub fn new(game_state: HanabiClient) -> Self {
        HanabiApp {
            exit: false,
            command: CommandState {
                current_player: match &game_state {
                    HanabiClient::Loaded(HanabiGame::Started { game_state, .. }) => {
                        game_state.this_client_player_index
                    }
                    _ => PlayerIndex(0),
                },
                current_command: CommandBuilder::Empty,
            },
            client_state: game_state,
            game_log_scroll_adjust: 0,
            game_state_selection: 0,
            hint_mode: HintMode::NotHints,
            card_focus: None,
        }
    }

    /// runs the application's main loop until the user quits
    pub fn draw<T>(&mut self, terminal: &mut Terminal<T>) -> BoxedResult<Vec<Binding<AppAction>>>
    where
        T: ratatui::backend::Backend,
    {
        // while !self.exit {

        let (legend_description, legend) = self.legend_for_command_state(&self.client_state);
        let mut ui = self.ui(legend_description, legend);

        terminal.draw(|frame| {
            // let tree = root_tree_widget(frame.size(), ui);
            ui.compute_layout(LayoutSize {
                width: length(frame.size().width as f32),
                height: length(frame.size().height as f32),
            });

            let area = frame.size();
            frame.buffer_mut().set_style(area, default_style());

            ui.render_ref(frame.size(), frame.buffer_mut());
        })?;

        let bindings: Vec<Binding<AppAction>> = ui.collect_bindings();

        Ok(bindings)
    }

    pub fn update(&mut self, state: HanabiClient) {
        
        self.command.current_player = match &state {
            HanabiClient::Loaded(HanabiGame::Started { game_state, .. }) => {
                game_state.this_client_player_index
            }
            _ => PlayerIndex(0),
        };
        self.client_state = state;
    }

    pub fn handle_action(&mut self, app_action: AppAction) -> BoxedResult<EventHandlerResult> {
        match app_action {
            AppAction::GameAction(game_action) => {
                let (builder, player_action) =
                    process_app_action(self.command.clone(), game_action);
                self.command = builder;
                match player_action {
                    Some(action) => {
                        return Ok(EventHandlerResult::PlayerAction(action));

                        // todo don't unwrap
                    }

                    _ => {}
                }
            }

            AppAction::Quit => {
                return Ok(EventHandlerResult::Quit);
            }
            AppAction::Start => {
                return Ok(EventHandlerResult::Start);
            }

            AppAction::ScrollGameLog(adjust) => {
                if adjust > 0 {
                    self.game_log_scroll_adjust = self.game_log_scroll_adjust.saturating_add(1);
                } else if adjust < 0 {
                    self.game_log_scroll_adjust = self.game_log_scroll_adjust.saturating_sub(1);
                }
            }
            AppAction::AdjustCurrentState(adjust) => {
                if adjust > 0 {
                    self.game_state_selection =
                        self.game_state_selection.saturating_add(1 as usize);
                } else if adjust < 0 {
                    self.game_state_selection =
                        self.game_state_selection.saturating_sub(1 as usize);
                }
            }
            AppAction::ChangeHintMode(hint_mode) => {
                self.hint_mode = hint_mode;
            }
            
            AppAction::FocusCard(player_index, slot_index) => {
                if self.card_focus == Some((player_index, slot_index)) {
                    self.card_focus = None;
                } else {
                    self.card_focus = Some((player_index, slot_index));
                }
                
            },
        }

        Ok(EventHandlerResult::Continue)
    }

    pub fn handle_event(&mut self, key: KeyCode) -> BoxedResult<EventHandlerResult> {
        use KeyCode::*;
        match key {
            Char('q') | Esc => {
                self.exit = true;
            }
            // Char(',') => {
            //     self.game_state_selection = self.game_state_selection.saturating_sub(1);
            // }
            // Char('.') => self.game_state_selection = self.game_state_selection.saturating_add(1),
            // Char('w') => {
            //     self.game_log_scroll_adjust = self.game_log_scroll_adjust.saturating_sub(1);
            //     // app.vertical_scroll_state = app.vertical_scroll_state.position(app.vertical_scroll);
            // }
            // Char('s') => {
            //     self.game_log_scroll_adjust = self.game_log_scroll_adjust.saturating_add(1);
            //     // app.vertical_scroll_state = app.vertical_scroll_state.position(app.vertical_scroll);
            // }
            _ => {}
        }

        if self.exit {
            return Ok(EventHandlerResult::Quit);
        }

        Ok(EventHandlerResult::Continue)
    }

    fn ui(&mut self, legend_description: String, legend: Vec<LegendItem>) -> Node<'static> {
        match &self.client_state {
            HanabiClient::Connecting => self.connecting_ui(),
            HanabiClient::Loaded(HanabiGame::Lobby { players, .. }) => {
                self.lobby_ui(players, legend_description, legend)
            }
            HanabiClient::Loaded(_) => {
                self.game_ui(self.clone().into(), legend_description, legend)
            }
        }
    }

    fn connecting_ui(&self) -> Node<'static> {
        HStack::new()
            .layout(LayoutStyle {
                size: LayoutSize {
                    width: percent(1.),
                    height: percent(1.),
                },
                justify_content: Some(JustifyContent::Center),
                ..HStack::default_layout()
            })
            .child(Span::raw(if self.exit {
                "Exiting..."
            } else {
                "Connecting... (yes spelled correctly this time)"
            }))
    }

    fn lobby_ui(
        &self,
        players: &Vec<OnlinePlayer>,
        legend_description: String,
        legend: Vec<LegendItem>,
    ) -> Node<'static> {
        self.game_ui(
            GameProps {
                game_state_index: 0,
                num_rounds: 0,
                board_render_state: BoardProps {
                    highest_played_card_for_suit: HashMap::new(),
                    discards: vec![],
                    draw_remaining: 0,
                    hints_remaining: 0,
                    fuse_remaining: 0,
                },
                players: players
                    .iter()
                    .enumerate()
                    .map(|(index, p)| {
                        player_node_props(
                            PlayerIndex(index),
                            p.name.clone(),
                            (0..5).into_iter().map(|_| None).collect_vec(),
                            PlayerRenderState::Default,
                            HintMode::NotHints,
                        )
                    })
                    .collect_vec(),
                game_log: vec![],
            },
            legend_description,
            legend,
        )
        // VStack::new()
        //     .layout(LayoutStyle {
        //         size: LayoutSize {
        //             width: percent(1.),
        //             height: percent(1.),
        //         },
        //         justify_content: Some(JustifyContent::Center),
        //         ..VStack::default_layout()
        //     })
        //     .child(
        //         Block::new()
        //             .borders(Borders::ALL)
        //             .border_type(BorderType::Rounded)
        //             .title("Game Lobby")
        //             .layout(VStack::default_layout())
        //             .childs(
        //                 players
        //                     .iter()
        //                     .map(|p| Span::raw(p.name.clone()).node())
        //                     .collect_vec(),
        //             ),
        //     )
    }

    fn game_ui(
        &self,
        game_props: GameProps,
        legend_description: String,
        legend: Vec<LegendItem>,
    ) -> Node<'static> {
        use taffy::prelude::*;

        GridStack::new().children(
            LayoutStyle {
                grid_template_columns: vec![fr(1.), length(40.)],
                grid_template_rows: vec![fr(1.), length(4.)],
                padding: LayoutRect {
                    top: length(1.),
                    left: length(4.),
                    right: length(10.),
                    bottom: length(1.),
                },

                size: Size {
                    width: percent(1.),
                    height: percent(1.),
                },
                ..GridStack::default_layout()
            },
            [
                // board/player area
                // grid_row: line(1), grid_column: line(1),
                VStack::new().children(
                    LayoutStyle {
                        // padding: padding(2.),
                        grid_row: line(1),
                        grid_column: line(1),

                        gap: Size {
                            width: length(0.),
                            height: length(1.),
                        },
                        // size: Size {
                        //     width: auto(),
                        //     height: auto(),
                        // },
                        justify_content: Some(JustifyContent::SpaceBetween),
                        ..VStack::default_layout()
                    },
                    Vec::from([
                        HStack::new().children(
                            LayoutStyle {
                                justify_content: Some(JustifyContent::Center),
                                size: Size {
                                    width: auto(),
                                    height: auto(),
                                },
                                gap: Size {
                                    width: length(1.),
                                    height: length(0.),
                                },
                                ..HStack::default_layout()
                            },
                            game_props
                                .players
                                .into_iter()
                                .map(|i| player_node(i))
                                .collect_vec(),
                        ),
                        board_node_tree(game_props.board_render_state),
                    ]),
                ),
                self.render_game_log(game_props.game_log.iter().map(|log| log.log_entries.clone()).flatten().collect_vec())
                    .append_layout(|layout| LayoutStyle {
                        grid_row: line(1),
                        grid_column: line(2),

                        ..layout
                    }),
                VStack::new()
                    .layout(LayoutStyle {
                        grid_row: line(2),
                        grid_column: line(1),

                        align_items: Some(AlignItems::Center),
                        gap: Size {
                            width: length(1.),
                            height: length(1.),
                        },
                        ..VStack::default_layout()
                    })
                    .child(Span::from(legend_description))
                    .child(HStack::new().children(
                        LayoutStyle {
                            size: Size {
                                width: auto(),
                                height: length(3.),
                            },
                            gap: Size {
                                width: length(1.),
                                height: length(0.),
                            },
                            justify_content: Some(JustifyContent::Center),

                            ..HStack::default_layout()
                        },
                        legend.into_iter().map(game_action_item_tree).collect_vec(),
                    )),
                HStack::new()
                    .layout(LayoutStyle {
                        grid_row: line(2),
                        grid_column: line(2),

                        justify_content: Some(JustifyContent::Center),
                        gap: Size {
                            width: length(0.),
                            height: length(0.),
                        },
                        ..HStack::default_layout()
                    })
                    .child(
                        Span::from(format!("History"))
                            .style(default_style().bg(SELECTION_COLOR).fg(Color::White)),
                    )
                    .childs(
                        [
                            if game_props.game_state_index + 1 < game_props.num_rounds {
                                Some(LegendItem {
                                    desc: "".to_string(),
                                    key_code: KeyCode::Up,
                                    action: AppAction::AdjustCurrentState(1),
                                })
                            } else {
                                None
                            },
                            if game_props.game_state_index > 0 {
                                Some(LegendItem {
                                    desc: "".to_string(),
                                    key_code: KeyCode::Down,
                                    action: AppAction::AdjustCurrentState(-1),
                                })
                            } else {
                                None
                            },
                        ]
                        .into_iter()
                        .flatten()
                        .map(game_action_item_tree)
                        .collect_vec(),
                    ),
            ],
        )
    }

    fn render_game_log(&self, mut log: Vec<Line<'static>>) -> Node<'static> {
        use taffy::prelude::*;

        let log_color = Color::Gray;

        let current_scroll = self.game_log_scroll_adjust;
        let max_scroll = log.len();

        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Game Log")
            .style(default_style().fg(log_color).bg(BACKGROUND_COLOR))
            .children(
                LayoutStyle {
                    flex_direction: taffy::FlexDirection::Column,

                    ..Block::default_layout()
                },
                vec![Node::new_scrollview(
                    LayoutStyle {
                        size: Size {
                            width: percent(1.),
                            height: percent(1.),
                        },
                        overflow: Point {
                            x: Overflow::Visible,
                            y: Overflow::Scroll,
                        },
                        ..Stack::default_layout()
                    },
                    Text::from(log),
                    self.game_log_scroll_adjust as i64,
                )
                .scrollable(
                    AppAction::ScrollGameLog(if current_scroll + 1 < max_scroll {
                        1
                    } else {
                        0
                    }),
                    AppAction::ScrollGameLog(if current_scroll > 0 { -1 } else { 0 }),
                )],
            )
    }

    fn legend_for_command_state(&self, game_state: &HanabiClient) -> (String, Vec<LegendItem>) {
        use KeyCode::*;
        match game_state {
            HanabiClient::Connecting { .. } => (
                "Connecting...".to_string(),
                vec![LegendItem {
                    desc: format!("Quit"),
                    key_code: KeyCode::Esc,
                    action: AppAction::Quit,
                }],
            ),
            HanabiClient::Loaded(game_state) => match game_state {
                HanabiGame::Lobby { .. } => (
                    "When you friends are done joining press 's' to start the game".to_string(),
                    vec![
                        LegendItem {
                            desc: format!("Leave"),
                            key_code: KeyCode::Esc,
                            action: AppAction::Quit,
                        },
                        LegendItem {
                            desc: format!("Start Game"),
                            key_code: Char('s'),
                            action: AppAction::Start,
                        },
                    ],
                ),
                HanabiGame::Started {
                    game_state,
                    players,
                    ..
                } => self.legend_for_command_state_game(game_state, players),

                HanabiGame::Ended { .. } => (
                    "Even good things come to an end (unfortunately)".to_string(),
                    vec![LegendItem {
                        desc: format!("Quit"),
                        key_code: KeyCode::Esc,
                        action: AppAction::Quit,
                    }],
                ),
            },
        }
    }

    fn legend_for_command_state_game(
        &self,
        game_state: &GameStateSnapshot,
        players: &Vec<OnlinePlayer>,
    ) -> (String, Vec<LegendItem>) {
        if let Some(outcome) = &game_state.outcome {
            return (
                format!(
                    "The game has ended, you {}",
                    match outcome {
                        GameOutcome::Win => "won!".to_string(),
                        GameOutcome::Fail { score } => format!("failed with the {score}"),
                    }
                ),
                vec![LegendItem {
                    desc: format!("Quit"),
                    key_code: KeyCode::Esc,
                    action: AppAction::Quit,
                }],
            );
        }

       

        fn readable_slot_index(SlotIndex(idx): SlotIndex) -> &'static str {
            match idx {
                0 => "First",
                1 => "Second",
                2 => "Third",
                3 => "Fourth",
                4 => "Fifth",
                _ => "wtf?",
            }
        }

        use KeyCode::*;
        match self.command.current_command {
            CommandBuilder::Empty =>  if game_state.current_turn_player_index != game_state.this_client_player_index {
                 (
                    format!(
                        "{}'s turn",
                        players[game_state.current_turn_player_index.0].name
                    ),
                    vec![LegendItem {
                        desc: "Move Card".to_string(),
                        key_code: Char('m'),
                        action: AppAction::GameAction(GameAction::StartMove),
                    }],
                )
            } else {
                (
                    format!(
                        "{}, it's your turn, choose an action! Your teammates are waiting...",
                        players[game_state.this_client_player_index.0]
                            .name
                            .clone()
                            .fg(SELECTION_COLOR),
                    ),
                    [
                        Some(LegendItem {
                            desc: "Play Card".to_string(),
                            key_code: Char('p'),
                            action: AppAction::GameAction(GameAction::StartPlay),
                        }),
                        Some(LegendItem {
                            desc: "Discard Card".to_string(),
                            key_code: Char('d'),
                            action: AppAction::GameAction(GameAction::StartDiscard),
                        }),
    
                        match game_state.remaining_hint_count {
                            0 => None,
                            _ => Some(LegendItem {
                                desc: "Give Hint".to_string(),
                                key_code: Char('h'),
                                action: AppAction::GameAction(GameAction::StartHint),
                            }),
                        },
                        Some(LegendItem {
                            desc: "Move Card".to_string(),
                            key_code: Char('m'),
                            action: AppAction::GameAction(GameAction::StartMove),
                        }),
                    ]
                    .into_iter()
                    .flatten()
                    .collect(),
                )
            }
            
            ,
            CommandBuilder::Hinting(HintState::ChoosingPlayer) => (
                "Choose a player index".to_string(),
                (0..game_state.players.len())
                    .filter(|&index| game_state.this_client_player_index.0 != index)
                    .map(|index| LegendItem {
                        desc: format!("{}", players[index].name),
                        key_code: Char(from_digit(index as u32 + 1, 10).unwrap()),
                        action: AppAction::GameAction(GameAction::SelectPlayer {
                            player_index: index as u8,
                        }),
                    })
                    .chain(iter::once(LegendItem {
                        desc: "Back".to_string(),
                        key_code: Backspace,
                        action: AppAction::GameAction(GameAction::Undo),
                    }))
                    .collect_vec(),
            ),

            CommandBuilder::Hinting(HintState::ChoosingHint { .. }) => (
                "Choose a suit or face hint".to_string(),
                vec![
                    LegendItem {
                        desc: "One".to_string(),
                        key_code: Char('1'),
                        action: AppAction::GameAction(GameAction::SelectFace(CardFace::One)),
                    },
                    LegendItem {
                        desc: "Two".to_string(),
                        key_code: Char('2'),
                        action: AppAction::GameAction(GameAction::SelectFace(CardFace::Two)),
                    },
                    LegendItem {
                        desc: "Three".to_string(),
                        key_code: Char('3'),
                        action: AppAction::GameAction(GameAction::SelectFace(CardFace::Three)),
                    },
                    LegendItem {
                        desc: "Four".to_string(),
                        key_code: Char('4'),
                        action: AppAction::GameAction(GameAction::SelectFace(CardFace::Four)),
                    },
                    LegendItem {
                        desc: "Five".to_string(),
                        key_code: Char('5'),
                        action: AppAction::GameAction(GameAction::SelectFace(CardFace::Five)),
                    },
                    LegendItem {
                        desc: "Blue".to_string(),
                        key_code: Char('b'),
                        action: AppAction::GameAction(GameAction::SelectSuit(CardSuit::Blue)),
                    },
                    LegendItem {
                        desc: "Green".to_string(),
                        key_code: Char('g'),
                        action: AppAction::GameAction(GameAction::SelectSuit(CardSuit::Green)),
                    },
                    LegendItem {
                        desc: "Red".to_string(),
                        key_code: Char('r'),
                        action: AppAction::GameAction(GameAction::SelectSuit(CardSuit::Red)),
                    },
                    LegendItem {
                        desc: "White".to_string(),
                        key_code: Char('w'),
                        action: AppAction::GameAction(GameAction::SelectSuit(CardSuit::White)),
                    },
                    LegendItem {
                        desc: "Yellow".to_string(),
                        key_code: Char('y'),
                        action: AppAction::GameAction(GameAction::SelectSuit(CardSuit::Yellow)),
                    },
                    LegendItem {
                        desc: "Back".to_string(),
                        key_code: Backspace,
                        action: AppAction::GameAction(GameAction::Undo),
                    },
                ],
            ),

            CommandBuilder::PlayingCard(CardState::ChoosingCard { card_type })
            | CommandBuilder::DiscardingCard(CardState::ChoosingCard { card_type }) | CommandBuilder::MovingCard(MovingCardState::ChoosingCard { card_type})  => {
                let (action, description) = match card_type {
                    CardBuilderType::Play => ("Play", "Choose a card to play"),
                    CardBuilderType::Discard => ("Discard", "Choose a card to send to the bin"),
                    CardBuilderType::Move => ("Move", "Choose a card to move"),
                };
                match game_state
                    .players
                    .get(game_state.this_client_player_index.0)
                {
                    Some(ClientPlayerView::Me { hand, .. }) => (
                        description.to_string(),
                        hand.iter()
                            .enumerate()
                            .filter(|(_, slot)| slot.is_some())
                            .map(|(index, _)| LegendItem {
                                desc: format!("{}", readable_slot_index(SlotIndex(index))),
                                key_code: Char(from_digit(index as u32 + 1, 10).unwrap()),
                                action: AppAction::GameAction(GameAction::SelectCard(SlotIndex(
                                    index,
                                ))),
                            })
                            .chain(iter::once(LegendItem {
                                desc: "Back".to_string(),
                                key_code: Backspace,
                                action: AppAction::GameAction(GameAction::Undo),
                            }))
                            .collect(),
                    ),
                    _ => panic!("Shouldn't be able to play as another player"),
                }
            }

            CommandBuilder::ConfirmingAction(action) => (
                {
                    match action {
                        PlayerAction::PlayCard(index) => {
                            format!("Confirm: Play {} card", readable_slot_index(index))
                        }
                        PlayerAction::DiscardCard(index) => {
                            format!("Confirm: Discard {} card", readable_slot_index(index))
                        }
                        PlayerAction::GiveHint(PlayerIndex(player_index), hint_action) => {
                            format!(
                                "Confirm: Give {} hint to {}",
                                match hint_action {
                                    HintAction::SameSuit(suit) =>
                                        format!("{suit:?}").fg(colorize_suit(suit)).bold(),
                                    HintAction::SameFace(face) => format!("{face:?}").bold(),
                                },
                                players[player_index].name,
                            )
                        }
                        PlayerAction::MoveSlot(_, _, _ ) => unreachable!("MoveSlot should not be confirmed"),
                    }
                },
                Vec::from([
                    LegendItem {
                        desc: "Confirm".to_string(),
                        key_code: KeyCode::Enter,
                        action: AppAction::GameAction(GameAction::Confirm(true)),
                    },
                    LegendItem {
                        desc: "Cancel".to_string(),
                        key_code: KeyCode::Esc,
                        action: AppAction::GameAction(GameAction::Confirm(false)),
                    },
                ]),
            ),
            CommandBuilder::MovingCard(MovingCardState::ChangeSlot { from_slot_index, new_slot_index }) => {
                (   format!("Move {} to {}", readable_slot_index(from_slot_index), readable_slot_index(new_slot_index)),
                    
                    vec![
                    LegendItem {
                        desc: "Left".to_string(),
                        key_code: KeyCode::Left,
                        action: AppAction::GameAction(GameAction::SelectSlot(SlotIndex(new_slot_index.0.saturating_sub(1).max(0)))),
                    },
                    LegendItem {
                        desc: "Right".to_string(),
                        key_code: KeyCode::Right,
                        action: AppAction::GameAction(GameAction::SelectSlot(SlotIndex(new_slot_index.0.saturating_add(1).min(game_state.game_config.hand_size as usize - 1)))),
                    },
                    LegendItem {
                        desc: "Confirm".to_string(),
                        key_code: KeyCode::Enter,
                        action: AppAction::GameAction(GameAction::Confirm(true)),
                    },
                ])
            },
        }
    }
}

struct GameLogRow {
    player_index: usize,
    action: PlayerAction,
    effects: Vec<GameEffect>,
    outcome: Option<GameOutcome>,
}

fn generate_game_log(
    game_state: &GameStateSnapshot,
    log: &Vec<GameSnapshotEvent>,
    players: &Vec<OnlinePlayer>,
    selected_turn_index: Option<u8>,
    highlighted_card_focus: Option<(PlayerIndex, usize)>,
) -> Vec<GameLogEntryProps> {
    use shared::model::GameEffect as Eff;

    let self_player = game_state.this_client_player_index.0;

    let player_name_span = |player_index: usize| {
        let name = &players[player_index].name;
        Span::from(name[..8.min(name.len())].to_string())
            .fg(if player_index == self_player {
                SELECTION_COLOR
            } else {
                Color::White
            })
            .bold()
    };

    // fn player_name(player: &OnlinePlayer, is_me: bool) -> Span<'static> {
    //     let name = if player.name.len() > 8 {
    //         format!("{}...", &player.name[0..8].to_string())
    //     } else {
    //         player.name.clone()
    //     };

    // }

    fn card(c: Card) -> Span<'static> {
        let Card { suit, face } = c;
        format!("{}{}", suit_span(suit), face_span(face))
            .fg(colorize_suit(suit))
            .bold()
    }

    fn slot(slot: usize) -> Span<'static> {
        Span::from(format!(
            "{} card",
            match slot + 1 {
                1 => Span::from("1st"),
                2 => Span::from("2nd"),
                3 => Span::from("3rd"),
                4 => Span::from("4th"),
                5 => Span::from("5th"),
                _ => Span::from("??"),
            }
        ))
    }

    fn suit_span(suit: CardSuit) -> Span<'static> {
        // Span::from(suit.key())
        //     .style(default_style().fg(BACKGROUND_COLOR).bg(colorize_suit(suit)))
        //     .bold()
        Span::from(match suit {
            CardSuit::Red => "R",
            CardSuit::Green => "G",
            CardSuit::Yellow => "Y",
            CardSuit::White => "W",
            CardSuit::Blue => "B",
            // CardSuit::Red => "\u{f0b19}",
            // CardSuit::Green => "\u{f0b0e}",
            // CardSuit::Yellow => "\u{f0b20}",
            // CardSuit::White => "\u{f0b1e}",
            // CardSuit::Blue => "\u{f0b09}",
        })
        .fg(colorize_suit(suit))
        .bold()
        // .fg(BACKGROUND_COLOR)
    }

    fn face_span(face: CardFace) -> Span<'static> {
        Span::from(match face {
            CardFace::One => "1",
            CardFace::Two => "2",
            CardFace::Three => "3",
            CardFace::Four => "4",
            CardFace::Five => "5",
            // CardFace::One => "\u{f03a6}",
            // CardFace::Two => "\u{f03a9}",
            // CardFace::Three => "\u{f03ac}",
            // CardFace::Four => "\u{f03ae}",
            // CardFace::Five => "\u{f03b0}",
        })
        .fg(ALMOST_WHITE)
        .bold()
    }

    fn card_played(effects: &Vec<GameEffect>) -> Card {
        let card_played = effects.iter().find_map(|effect| match effect {
            Eff::PlaceOnBoard(card) => Some(card),
            Eff::AddToDiscard(card) => Some(card),
            _ => None,
        });

        *card_played.unwrap()
    }

    fn hint_slots(effects: &Vec<GameEffect>) -> Vec<usize> {
        effects
            .iter()
            .filter_map(|e| match e {
                Eff::HintCard(_, SlotIndex(index), Hint::IsFace(_) | Hint::IsSuit(_)) => {
                    Some(*index)
                }
                _ => None,
            })
            .collect_vec()
    }

    fn hint_spans(
        hand_size: usize,
        hint_type: HintAction,
        effects: &Vec<GameEffect>,
    ) -> Vec<Span<'static>> {
        (0..hand_size)
            .into_iter()
            .map(|i| {
                let hint = hint_slots(&effects).contains(&i);
                match hint_type {
                    HintAction::SameFace(face) => {
                        if hint {
                            face_span(face).fg(ALMOST_WHITE)
                        } else {
                            "_".fg(DIM_TEXT)
                        }
                    }
                    HintAction::SameSuit(suit) => {
                        if hint {
                            suit_span(suit)
                        } else {
                            "_".fg(DIM_TEXT)
                        }
                    }
                }
            })
            .collect_vec()
        // hint_slots(&effects)
        //     .into_iter()
        //     .map(|_| match hint_type {
        //         HintAction::SameFace(face) => face_span(face),
        //         HintAction::SameSuit(suit) => suit_span(suit),
        //     })
        //     // .intersperse(Span::raw(" "))
        //     .collect_vec()
    }

    fn hint_count(effects: &Vec<GameEffect>) -> usize {
        effects
            .iter()
            .filter(|e| matches!(e, Eff::HintCard(_, _, _)))
            .count()
    }

    fn result_span(effects: &Vec<GameEffect>) -> Span<'static> {
        let result = effects.iter().find_map(|effect| match effect {
            Eff::BurnFuse => Some(" \u{f1052} \u{f0691}"),
            Eff::IncHint => Some(" \u{f15cb} \u{f017}"),
            _ => None,
        });

        match result {
            Some(result) => Span::from(result).style(default_style()),
            None => Span::raw(""),
        }
    }

    fn extra(effects: &Vec<GameEffect>) -> Vec<Span<'static>> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Eff::MarkLastTurn(_) => Some("    LAST ROUND!".fg(TURN_COLOR).bold()),
                _ => None,
            })
            .collect_vec()
    }

    fn outcome_lines(outcome: Option<GameOutcome>) -> Vec<Span<'static>> {
        match outcome {
            Some(outcome) => vec![
                Span::raw("    Game Over: ").fg(TURN_COLOR).bold(),
                match outcome {
                    GameOutcome::Win => Span::raw("Victory!").fg(TURN_COLOR).bold(),
                    GameOutcome::Fail { score } => {
                        Span::raw(format!("Defeat :( (score = {})", score))
                            .fg(TURN_COLOR)
                            .bold()
                    }
                }],
            
            None => vec![]
        } 
    }

    let count_span = |i: u8, render_state: GameLogRenderState| -> Vec<Span<'static>> {
        let count = format!("{}.", i + 1);
        
        let span = Span::raw(format!("{:<3}", count)).fg(match render_state {
            GameLogRenderState::Default => DIM_TEXT,
            _ => BACKGROUND_COLOR
        }).bg(match render_state {
            GameLogRenderState::CurrentSelection |  GameLogRenderState::CurrentSelectionAndHighlighted => TURN_COLOR,
            GameLogRenderState::Highlighted => SELECTION_COLOR,
            _ => BACKGROUND_COLOR
        });

        let spacing = Span::raw(" ").fg(match render_state {
            GameLogRenderState::Default => DIM_TEXT,
            _ => BACKGROUND_COLOR
        }).bg(match render_state {
            GameLogRenderState::Highlighted | GameLogRenderState::CurrentSelectionAndHighlighted => SELECTION_COLOR,
            _ => BACKGROUND_COLOR
        });

        vec![span, spacing]

        //
        // match render_state {
        //     GameLogRenderState::Default => span.fg(DIM_TEXT),
        //     GameLogRenderState::CurrentSelection => span.to_string().bg(SELECTION_COLOR).fg(BACKGROUND_COLOR),
        //     GameLogRenderState::Highlighted => span.to_string().bg(SELECTION_COLOR).fg(BACKGROUND_COLOR),
        // }
    };

    fn log_row<'a>(index: Vec<Span<'a>>, spans: Vec<Span<'a>>) -> Vec<Span<'a>> {
        index.into_iter().chain(spans).collect_vec()
    }; 


    let game_log_lines = |game_event: &GameSnapshotEvent, render_state: GameLogRenderState| -> Vec<Line<'static>> {
        match game_event {
            GameSnapshotEvent {
                current_turn_count: turn_count,
                current_turn_player_index: PlayerIndex(player_index),
                event_player_index,
                event_action: PlayerAction::PlayCard(SlotIndex(slot_index)),
                effects,
                ..
            } => [
                log_row(
                count_span(*turn_count, render_state),
                [
                    player_name_span(*player_index),
                    Span::raw(" plays "),
                    slot(*slot_index),
                    Span::raw(" "),
                    card(card_played(&effects)),
                    result_span(&effects),
                ]
                .to_vec()),
                extra(&effects),
            ]
            .to_vec(),
            GameSnapshotEvent {
                current_turn_count: turn_count,
                current_turn_player_index: PlayerIndex(player_index),
                event_player_index,
                event_action: PlayerAction::DiscardCard(SlotIndex(slot_index)),
                effects,
                ..
            } => [
                log_row(
                count_span(*turn_count, render_state), [
                    player_name_span(*player_index),
                    Span::raw(" dumps "),
                    slot(*slot_index),
                    Span::raw(" "),
                    card(card_played(&effects)),
                    result_span(&effects),
                ]
                .to_vec()),
                extra(&effects),
            ]
            .to_vec(),
            GameSnapshotEvent {
                current_turn_count: turn_count,
                current_turn_player_index: PlayerIndex(player_index),
                event_player_index,
                event_action: PlayerAction::GiveHint(PlayerIndex(hinted_index), hint),
                effects,
                ..
            }
           => [
            log_row(
            count_span(*turn_count, render_state),
                [
                    player_name_span(*player_index),
                    Span::raw(" hints "),
                    player_name_span(*hinted_index),
                    Span::raw(" "),
                ]
                .into_iter()
                .chain(hint_spans(
                    game_state.game_config.hand_size,
                    *hint,
                    &effects,
                ))
                .collect_vec()),
                extra(&effects),
            ]
            .to_vec(),
            GameSnapshotEvent {
                current_turn_count: turn_count,
                current_turn_player_index: PlayerIndex(player_index),
                event_player_index,
                event_action: PlayerAction::MoveSlot(_,_,_),
                effects,
                ..
            } => {
                vec![]
            }
            // GameSnapshotEvent {
            //     event: Ev::GameOver(outcome),
            //     ..
            // } => [[
            //     Span::raw("    Game Over: ").fg(TURN_COLOR).bold(),
            //     match outcome {
            //         GameOutcome::Win => Span::raw("Victory!").fg(TURN_COLOR).bold(),
            //         GameOutcome::Fail { score } => {
            //             Span::raw(format!("Defeat :( (score = {})", score))
            //                 .fg(TURN_COLOR)
            //                 .bold()
            //         }
            //     },
            // ]
            // .to_vec()]
            // .to_vec(),
        }.into_iter()
        .chain(vec![outcome_lines(game_event.post_event_game_snapshot.outcome)])
        .filter(|line| !line.is_empty())
        .map(|spans| Line::from(spans))
        .collect_vec()
    };

    let highlighted_indexes = highlighted_card_focus.map(|(focussed_player_index, focussed_card_num)| {
        let draw_number = |player_index: PlayerIndex, slot_index: SlotIndex, snapshot: &GameStateSnapshot| {
            match &snapshot.players[player_index.0] {
                ClientPlayerView::Me { name, hand } => hand[slot_index.0].as_ref().map(|c| c.draw_number),
                ClientPlayerView::Teammate { name, hand } => hand[slot_index.0].as_ref().map(|c| c.draw_number),
            }
        };

        // let focussed_card_num = draw_number(focussed_player_index, focussed_slot_index, game_state);

        let draw_turn_log_index = log.iter().position(|e| {
            if (0..e.post_event_game_snapshot.game_config.hand_size).any(|slot_index| {
                Some(focussed_card_num) == draw_number(focussed_player_index, SlotIndex(slot_index), &e.post_event_game_snapshot)
            }) {
                true
            } else {
                false
            }
        }).unwrap();

        let remove_turn_log_index = log.iter().skip(draw_turn_log_index).find_map(|e| {
            if (0..e.post_event_game_snapshot.game_config.hand_size).all(|slot_index| {
                Some(focussed_card_num) != draw_number(focussed_player_index, SlotIndex(slot_index), &e.post_event_game_snapshot)
            }) {
                Some(e.current_turn_count)
            } else {
                None
            }
        });

        let hint_turns = log.iter().filter_map(|e| {
            if match e.event_action {
                // PlayerAction::PlayCard(slot_index) => todo!(),
                // PlayerAction::DiscardCard(slot_index) => todo!(),
                PlayerAction::GiveHint(player_index, _) if player_index == focussed_player_index => 
                    e.effects.iter().any(|effect| match effect {
                        Eff::HintCard(player_index, slot_index, Hint::IsFace(_) | Hint::IsSuit(_)) if Some(focussed_card_num) == draw_number(focussed_player_index, *slot_index, &e.post_event_game_snapshot) => true,
                        _ => false,
                    }),
                
                _ => false,
            } {
                Some(e.current_turn_count)
            } else {
                None
            }
        }).collect_vec();

        let mut highlighted_indexes = vec![];
        highlighted_indexes.push(log[draw_turn_log_index].current_turn_count);
        highlighted_indexes.extend(hint_turns);
        if let Some(remove_turn_log_index) = remove_turn_log_index {
            highlighted_indexes.push(remove_turn_log_index);
        }
        highlighted_indexes

    }).unwrap_or(vec![]);



    log.iter().group_by(|event| event.current_turn_count).into_iter().map(|(turn, events)| {
        let events = events.collect_vec();
        let last_event = events.last().unwrap().post_event_game_snapshot.clone();
        let render_state = |game_entry: &GameSnapshotEvent| -> GameLogRenderState {
            match (selected_turn_index, highlighted_indexes.contains(&game_entry.current_turn_count)) {
            (Some(selected_turn), true) if selected_turn == turn => GameLogRenderState::CurrentSelectionAndHighlighted,
            (Some(selected_turn), false) if selected_turn == turn => GameLogRenderState::CurrentSelection,
            (_, true) => GameLogRenderState::Highlighted,
            _ => GameLogRenderState::Default,                
        }
    };
        let events = events.into_iter().map(|event| game_log_lines(event, render_state(event))).flatten().collect_vec();
        GameLogEntryProps {
            turn_count: turn,
            log_entries: events,
            final_state: last_event,
            render_state: GameLogRenderState::Default
        }
    }).collect_vec()

}

#[derive(Debug, Clone, Copy)]
pub enum AppAction {
    Start,
    Quit,
    GameAction(GameAction),
    ScrollGameLog(i8),
    AdjustCurrentState(i8),
    ChangeHintMode(HintMode),
    FocusCard(PlayerIndex, usize),
}

struct LegendItem {
    desc: String,
    key_code: KeyCode,
    action: AppAction,
}

fn game_action_item_tree(item: LegendItem) -> Node<'static> {
    let item_text = match &item {
        LegendItem {
            desc,
            key_code: KeyCode::Char(key),
            ..
        } => format!("{} [{}]", desc, key),

        LegendItem {
            desc,
            key_code: KeyCode::Backspace,
            ..
        } => format!("{} [{}]", desc, "\u{f030d}"),

        LegendItem {
            desc,
            key_code: KeyCode::Esc,
            ..
        } => format!("{} [{}]", desc, "\u{f12b7} "),

        LegendItem {
            desc,
            key_code: KeyCode::Enter,
            ..
        } => format!("{} [{}]", desc, "\u{f0311} "),

        LegendItem {
            desc,
            key_code: KeyCode::Up,
            ..
        } => format!("{} [{}]", desc, "\u{eaa1} "),

        LegendItem {
            desc,
            key_code: KeyCode::Down,
            ..
        } => format!("{} [{}]", desc, "\u{ea9a} "),

        LegendItem {
            desc,
            key_code: KeyCode::Left,
            ..
        } => format!("{} [{}]", desc, "\u{ea9b} "),

        LegendItem {
            desc,
            key_code: KeyCode::Right,
            ..
        } => format!("{} [{}]", desc, "\u{ea9c} "),
        _ => panic!("Unknown keycode"),
    };

    Span::from(item_text)
        .style(default_style().bg(SELECTION_COLOR).fg(Color::White))
        .touchable(item.action)
        .keybinding(item.key_code, item.action)
}

fn board_node_props(
    played_cards: &Vec<Card>,
    discard_pile: &Vec<Card>,
    draw_pile_count: u8,
    remaining_hint_count: u8,
    remaining_bomb_count: u8,
) -> BoardProps {
    let all_suits = [
        CardSuit::Blue,
        CardSuit::Green,
        CardSuit::Red,
        CardSuit::White,
        CardSuit::Yellow,
    ];

    let highest_cards = all_suits
        .iter()
        .enumerate()
        .map(|(_, &cur_suit)| {
            let mut card_faces: Vec<_> = played_cards
                .iter()
                .filter_map(|c| match c {
                    &Card { suit, face } if suit == cur_suit => Some(face),
                    _ => None,
                })
                .collect();
            card_faces.sort();
            card_faces
        })
        .collect_vec();

    BoardProps {
        highest_played_card_for_suit: all_suits
            .iter()
            .enumerate()
            .filter_map(|(suit_index, &cur_suit)| {
                let highest_face = highest_cards[suit_index].last().copied();
                Some((cur_suit, highest_face?))
            })
            .collect::<HashMap<CardSuit, CardFace>>(),
        discards: discard_pile.clone(),
        draw_remaining: draw_pile_count as usize,
        hints_remaining: remaining_hint_count as usize,
        fuse_remaining: remaining_bomb_count as usize,
    }
}

fn slot_node_props(
    player_index: PlayerIndex,
    slot_index: SlotIndex,
    card_draw_num: usize,
    card: Option<Card>,
    hints: Vec<Hint>,
    card_render_state: CardRenderState,
) -> SlotNodeProps {
    let face_hint = hints.clone().into_iter().find_map(|h| match h {
        Hint::IsFace(face) => Some(face),
        _ => None,
    });

    let suit_hint = hints.clone().into_iter().find_map(|h| match h {
        Hint::IsSuit(suit) => Some(suit),
        _ => None,
    });

    let (suit, face) = card
        .map(|c| (Some(c.suit), Some(c.face)))
        .unwrap_or((suit_hint, face_hint));

    SlotNodeProps {
        player_index,
        slot_index,
        card_id: card_draw_num,

        all_hints: hints.clone(),
        card: CardProps {
            card: CardNodeProps::SomeCard(face, suit),
            state: card_render_state,
        },
        face_hint: hints.clone().into_iter().find(|h| match h {
            Hint::IsFace(_) => true,
            _ => false,
        }), //face.map(|f| Hint::IsFace(f)),
        suit_hint: hints.clone().into_iter().find(|h| match h {
            Hint::IsSuit(_) => true,
            _ => false,
        }),
        unique_hints: hints
            .clone()
            .into_iter()
            .filter(|h| match h {
                Hint::IsSuit(_) | Hint::IsFace(_) => true,
                _ => false,
            })
            .unique()
            .collect(),
        unique_not_hints: hints
            .clone()
            .into_iter()
            .filter(|h| match h {
                Hint::IsNotSuit(_) | Hint::IsNotFace(_) => true,
                _ => false,
            })
            .unique()
            .collect(),
  
    }
}

fn player_node_props(
    player_index: PlayerIndex,
    name: String,
    hand: Vec<Option<SlotNodeProps>>,
    player_state: PlayerRenderState,
    hint_mode: HintMode,
) -> PlayerNodeProps {
    // let player = &game_state.players[player_index];

    // let hand_size = match player {
    //     ClientPlayerView::Me { hand, .. } => hand.len(),
    //     ClientPlayerView::Teammate { hand, .. } => hand.len(),
    // };

    let slot_props = hand
        .into_iter()
        .enumerate()
        .map(|(index, slot)| match slot {
            Some(slot) => slot,
            None => SlotNodeProps {
                player_index: player_index,
                slot_index: SlotIndex(index),
                card_id: 0,
                card: CardProps { card: CardNodeProps::Empty, state: CardRenderState::Default },
                all_hints: vec![],
                face_hint: None,
                suit_hint: None,
                unique_hints: vec![],
                unique_not_hints: vec![],
             
            },
        })
        .collect_vec();

    PlayerNodeProps {
        name,
        hint_mode: hint_mode,
        hand: slot_props,
        state: player_state,
        // state: match (game_state.turn, command_state) {
        //     (PlayerIndex(turn), _) if turn as usize == player_index => {
        //         PlayerRenderState::CurrentTurn
        //     }
        //     (
        //         _,
        //         &CommandBuilder::Hint(HintState::ChoosingHint {
        //             player_index: command_player_index,
        //         }),
        //     ) if command_player_index as usize == player_index => {
        //         PlayerRenderState::CurrentSelection
        //     }
        //     _ => PlayerRenderState::Default,
        // },
    }
}

/*
    game_state: &GameStateSnapshot,
    log: &Vec<GameSnapshotEvent>,
    players: &Vec<OnlinePlayer>,
    selected_round: Option<u8>,
     */

enum GameLogRenderState {
    Default,
    CurrentSelection,
    Highlighted,
    CurrentSelectionAndHighlighted
}

struct GameLogEntryProps {
    turn_count: u8,
    log_entries: Vec<Line<'static>>,
    final_state: GameStateSnapshot,
    render_state: GameLogRenderState,
}

struct GameProps {
    board_render_state: BoardProps,
    players: Vec<PlayerNodeProps>,
    game_log: Vec<GameLogEntryProps>,
    num_rounds: usize,
    game_state_index: usize,
}

impl From<HanabiApp> for GameProps {
    fn from(app_state: HanabiApp) -> Self {
        let hint_mode = app_state.hint_mode;
        match &app_state.client_state {
            HanabiClient::Connecting => todo!(),
            HanabiClient::Loaded(game) => match game {
                HanabiGame::Lobby { .. } => todo!(),
                HanabiGame::Started {
                    players,
                    game_state,
                    log,
                    ..
                } => {
                    let selected_turn_index = game_state.num_rounds.saturating_sub(app_state.game_state_selection as u8);
                    let merged_game_log = log.clone().into_iter().group_by(|event| event.current_turn_count).into_iter().map(|(t, g)| {
                        let events = g.collect_vec();
                        let last_event = events.last().unwrap();
                        let last_player_acting = last_event.event_player_index;
                        let ending_state = last_event.post_event_game_snapshot.clone();
                        // let events = events.into_iter().map(|event| game_log_lines(event, GameLogRenderState::Default)).flatten().collect_vec();
                        // (last_player_action, last_event)
                        (last_player_acting, ending_state)
                    
                    }).into_iter().collect_vec();
                    
                    let (acting_player, selected_game_state) = if app_state.game_state_selection == 0 {
                        (game_state.current_turn_player_index, game_state)
                    }  else {
                        let selected_game_state = &merged_game_log[selected_turn_index as usize - 1].1;
                        (selected_game_state.current_turn_player_index, selected_game_state)
                        
                        // merged_game_log.iter().nth(selected_turn_index as usize).map(|&(player_index, snapshot)| {
                        //     (player_index, &snapshot)
                        // }).unwrap()
                    };
                    // let selected_game_snapshot_event = log.iter().nth(selected_turn_index as usize).unwrap();
                    // let acting_player = selected_game_snapshot_event.current_turn_player_index;
                    // let selected_game_state = &selected_game_snapshot_event.post_event_game_snapshot;

                    // let (acting_player, selected_game_state) = .unwrap_or();
              
                    // let (selected_game_state_index, acting_player, selected_game_state) =
                    //     if app_state.game_state_selection == 0 {
                    //         (None, game_state.current_turn_player_index, game_state)
                    //     } else {
                    //         log.iter()
                    //             .enumerate()
                    //             .find(|(i, ev)| game_state.num_rounds.saturating_sub(app_state.game_state_selection as u8) == ev.current_turn_count as u8)
                    //             .map(|(i, ev)| {
                    //                 (
                    //                     Some(ev.snapshot.num_rounds),
                    //                     match ev.event {
                    //                         GameEvent::PlayerAction { player_index, .. } => {
                    //                             player_index
                    //                         }
                    //                         GameEvent::GameOver(_) => {
                    //                             ev.snapshot.current_turn_player_index
                    //                         }
                    //                     },
                    //                     &ev.snapshot,
                    //                 )
                    //             })
                    //             .unwrap()
                    //     };

                    GameProps {
                        num_rounds: game_state.num_rounds as usize,
                        game_state_index: app_state.game_state_selection,
                        game_log: generate_game_log(
                            &selected_game_state,
                            log,
                            players,
                            Some(selected_turn_index),
                            app_state.card_focus
                        ),
                        board_render_state: board_node_props(
                            &selected_game_state.played_cards,
                            &selected_game_state.discard_pile,
                            selected_game_state.draw_pile_count,
                            selected_game_state.remaining_hint_count,
                            selected_game_state.remaining_bomb_count,
                        ),
                        players: (0..players.len())
                            .into_iter()
                            .map(|player_index| {
                                let player_state =
                                    match (acting_player, &app_state.command.current_command) {
                                        (PlayerIndex(turn), _) if turn as usize == player_index => {
                                            PlayerRenderState::CurrentTurn
                                        }
                                        (
                                            _,
                                            &CommandBuilder::Hinting(HintState::ChoosingHint {
                                                player_index: command_player_index,
                                            }),
                                        ) if command_player_index as usize == player_index => {
                                            PlayerRenderState::CurrentSelection
                                        }
                                        (
                                            _,
                                            &CommandBuilder::ConfirmingAction(
                                                PlayerAction::GiveHint(
                                                    PlayerIndex(command_player_index),
                                                    _,
                                                ),
                                            ),
                                        ) if command_player_index as usize == player_index => {
                                            PlayerRenderState::CurrentSelection
                                        }
                                        _ => PlayerRenderState::Default,
                                    };
                                // slot_node_props(card.clone(), hints.clone())
                                match &selected_game_state.players[player_index] {
                                    ClientPlayerView::Me { name, hand } => {
                                        let mut slot_props : Vec<_> =  hand.iter().enumerate()
                                        .map(|(slot_index, h)| {
                                            let slot_index_focussed = app_state.card_focus.map(|(focussed_player_index, focussed_slot_index)| {
                                                focussed_player_index == PlayerIndex(player_index) && Some(focussed_slot_index) == h.as_ref().map(|c| c.draw_number)
                                            }).unwrap_or(false);

                                            h.clone().map(|c| {
                                                slot_node_props(PlayerIndex(player_index), SlotIndex(slot_index), c.draw_number, None, c.hints.clone(), match (&app_state.command.current_command, slot_index_focussed) {                    
                                                   ( &CommandBuilder::ConfirmingAction(PlayerAction::PlayCard(SlotIndex(selected_slot_index)) | PlayerAction::DiscardCard(SlotIndex(selected_slot_index))) | &CommandBuilder::MovingCard(MovingCardState::ChangeSlot {  from_slot_index : SlotIndex(selected_slot_index), ..}), _) if slot_index == selected_slot_index => CardRenderState::Highlighted ,
                                                   (_, true) => CardRenderState::Highlighted,
                                                    _ => CardRenderState::Default,                                           
                                                }/* implement for teammates: choosing a card to play or discard */)
                                            })
                                        })
                                        .collect();
                                        match &app_state.command.current_command {

                                            &CommandBuilder::MovingCard(MovingCardState::ChangeSlot { from_slot_index: SlotIndex(from_slot_index), new_slot_index: SlotIndex(new_slot_index) }) => {
                                                if from_slot_index < new_slot_index {
                                                    slot_props[from_slot_index..=new_slot_index].rotate_left(1);
                                                } else {
                                                    slot_props[new_slot_index..=from_slot_index].rotate_right(1);
                                                }
                                            },
                                            _ => {}
                                        }

                                        player_node_props(
                                            PlayerIndex(player_index),
                                        name.clone(),
                                       slot_props,
                                        player_state,
                                        hint_mode,
                                    )
                                },
                                    ClientPlayerView::Teammate { name, hand } => player_node_props(
                                        PlayerIndex(player_index),
                                        name.clone(),
                                        hand.iter().enumerate()
                                            .map(|(slot_index, h)| {
                                                h.clone().map(|s| {
                                                    let slot_index_focussed = app_state.card_focus.map(|(focussed_player_index, focussed_slot_index)| {
                                                        focussed_player_index == PlayerIndex(player_index) && focussed_slot_index == s.draw_number
                                                    }).unwrap_or(false);

                                                    slot_node_props(PlayerIndex(player_index), SlotIndex(slot_index), s.draw_number, Some(s.card), s.hints.clone(), match  (&app_state.command.current_command, slot_index_focussed) {                    
                                                    (&CommandBuilder::ConfirmingAction(PlayerAction::GiveHint(PlayerIndex(hinting_player_index), hint_action)), _) if player_index == hinting_player_index => match hint_action {
                                                            HintAction::SameSuit(suit) if s.card.suit == suit => CardRenderState::Highlighted,
                                                            HintAction::SameFace(face) if s.card.face == face => CardRenderState::Highlighted,
                                                            _ => CardRenderState::Default,
                                                        },
                                                        (_, true) => CardRenderState::Highlighted,
                                                        _ => CardRenderState::Default,                                           
                                                    })
                                                })
                                            })
                                            .collect(),
                                        player_state,
                                        hint_mode,
                                    ),
                                }
                            })
                            .collect(),
                    }
                }
                HanabiGame::Ended {
                    players,
                    game_state,
                    revealed_game_log,
                    ..
                } => {
                    let selected_turn_index = game_state.num_rounds.saturating_sub(app_state.game_state_selection as u8);
                    let merged_game_log = revealed_game_log.log.clone().into_iter().group_by(|event| event.current_turn_count).into_iter().map(|(t, g)| {
                        let events = g.collect_vec();
                        let last_event = events.last().unwrap().clone();
                        // let events = events.into_iter().map(|event| game_log_lines(event, GameLogRenderState::Default)).flatten().collect_vec();
                        last_event
                    
                    }).into_iter().collect_vec();
                    
                    let (acting_player, selected_game_state) = 
                        merged_game_log.iter().nth(selected_turn_index as usize - 1).map(|e| {
                            (e.post_event_game_state.current_player_index() , &e.post_event_game_state)
                        }).unwrap();

                    GameProps {
                        num_rounds: revealed_game_log.current_game_state().turn as usize,
                        game_state_index: app_state.game_state_selection,
                        game_log: generate_game_log(
                            game_state,
                            &revealed_game_log.into_client_game_log(
                                game_state.this_client_player_index,
                                players.iter().map(|p| p.name.clone()).collect(),
                            ),
                            players,
                            Some(selected_turn_index),
                            app_state.card_focus
                        ),
                        board_render_state: board_node_props(
                            &selected_game_state.played_cards,
                            &selected_game_state.discard_pile,
                            selected_game_state.draw_pile.len() as u8,
                            selected_game_state.remaining_hint_count,
                            selected_game_state.remaining_bomb_count,
                        ),
                        players: (0..players.len())
                            .into_iter()
                            .map(|player_index| {
                                let player_state = if acting_player.0 == player_index {
                                    PlayerRenderState::CurrentTurn
                                } else {
                                    PlayerRenderState::Default
                                };

                                player_node_props(
                                    PlayerIndex(player_index),
                                    players[player_index].name.clone(),
                                    selected_game_state.players[player_index]
                                        .hand
                                        .iter()
                                        .enumerate()
                                        .map(|(slot_index, h)| h.clone().map(|c| { 
                                            let slot_index_focussed = app_state.card_focus.map(|(focussed_player_index, focussed_slot_index)| {
                                                focussed_player_index == PlayerIndex(player_index) && Some(focussed_slot_index) == h.as_ref().map(|c| c.draw_number)
                                            }).unwrap_or(false);

                                            slot_node_props(PlayerIndex(player_index), SlotIndex(slot_index), c.draw_number ,Some(c.card), c.hints.clone(), match slot_index_focussed {
                                             true => CardRenderState::Highlighted ,
                                             false => CardRenderState::Default
                                        })
                                        }))
                                        .collect(),
                                    player_state,
                                    app_state.hint_mode,
                                )
                            })
                            .collect(),
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_data::*;

    use super::*;

    // #[test]
    // fn test_game_ui() {
    //     use ratatui::prelude as ratatui;

    //     let players = vec![
    //         OnlinePlayer {
    //             name: "p1".into(),
    //             connection_status: ConnectionStatus::Connected,
    //             is_host: true,
    //         },
    //         OnlinePlayer {
    //             name: "p2".into(),
    //             connection_status: ConnectionStatus::Connected,
    //             is_host: true,
    //         },
    //     ];
    //     let app = HanabiApp {
    //         exit: false,
    //         command: CommandState {
    //             current_command: CommandBuilder::Empty,
    //         },
    //         client_state: HanabiClient::Loaded(HanabiGame::Started {
    //             log: vec![],
    //             session_id: "1".into(),
    //             players: players.clone(),
    //             game_state: generate_minimal_test_game_state(),
    //         }),
    //         game_log_scroll_adjust: 0,
    //         game_state_selection: 0,
    //     };

    //     let mut buf = Buffer::empty(ratatui::Rect {
    //         x: 0,
    //         y: 0,
    //         width: 248,
    //         height: 46,
    //     });

    //     let tree_widget = root_tree_widget(
    //         buf.area,
    //         app.game_ui(app.clone().into(), "".to_string(), vec![]),
    //     );

    //     tree_widget.render_ref(buf.area, &mut buf);

    //     println!(
    //         "top left corner = '{:?}' '{:?}' '{:?}'",
    //         buf.get(buf.area.width - 3, 0).symbol().chars(),
    //         buf.get(buf.area.width - 2, 0).symbol().chars(),
    //         buf.get(buf.area.width - 1, 0).symbol().chars()
    //     );

    //     println!(
    //         "top left corner = '{:?}' '{:?}' '{:?}'",
    //         buf.get(buf.area.width - 3, 1).symbol().chars(),
    //         buf.get(buf.area.width - 2, 1).symbol().chars(),
    //         buf.get(buf.area.width - 1, 1).symbol().chars()
    //     );
    // }

    // #[test]
    // fn test_panic_case_ui() {
    //     use ratatui::prelude as ratatui;

    //     let app_data = generate_example_panic_case_2();

    //     let app = HanabiApp {
    //         exit: false,
    //         command: CommandState {
    //             current_command: CommandBuilder::Empty,
    //         },
    //         client_state: HanabiClient::Loaded(app_data.clone()),
    //         game_log_scroll_adjust: 0,
    //         game_state_selection: 0,
    //     };
    //     // let mut tree = TreeWidget::new();
    //     // let root_id = tree.add_tree(Stack::new().children(
    //     //     LayoutStyle {
    //     //         size: Size {
    //     //             width: length(100. as f32),
    //     //             height: length(100. as f32),
    //     //         },
    //     //         padding: padding(2.),
    //     //         ..Stack::default_layout()
    //     //     },
    //     //     vec![app.game_ui(&generate_minimal_test_game_state(), None, &players)],
    //     // ));

    //     let mut buf = Buffer::empty(ratatui::Rect {
    //         x: 0,
    //         y: 0,
    //         width: 156,
    //         height: 38,
    //     });

    //     match app_data {
    //         HanabiGame::Started {
    //             session_id: _,
    //             players,
    //             game_state,
    //             ..
    //         } => {
    //             let tree_widget = root_tree_widget(
    //                 buf.area,
    //                 app.game_ui(app.clone().into(), "".to_string(), vec![]),
    //             );
    //             tree_widget.render_ref(buf.area, &mut buf);
    //         }
    //         _ => todo!(),
    //     }
    // }
}
