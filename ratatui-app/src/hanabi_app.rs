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
    client_state: HanabiClient,
    game_log_scroll_adjust: i64,
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
                current_command: CommandBuilder::Empty,
            },
            client_state: game_state,
            game_log_scroll_adjust: 0,
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
                self.game_log_scroll_adjust =
                    self.game_log_scroll_adjust.saturating_add(adjust as i64);
            }
        }

        Ok(EventHandlerResult::Continue)
    }

    pub fn handle_event(&mut self, key: KeyCode) -> BoxedResult<EventHandlerResult> {
        use KeyCode::*;
        match key {
            Char('q') | Esc => {
                self.exit = true;
            }
            Char('w') => {
                self.game_log_scroll_adjust = self.game_log_scroll_adjust.saturating_sub(1);
                // app.vertical_scroll_state = app.vertical_scroll_state.position(app.vertical_scroll);
            }
            Char('s') => {
                self.game_log_scroll_adjust = self.game_log_scroll_adjust.saturating_add(1);
                // app.vertical_scroll_state = app.vertical_scroll_state.position(app.vertical_scroll);
            }
            key_code => {
                let (_, options) = self.legend_for_command_state(&self.client_state);
                let triggered_option = options.into_iter().find(|a| a.key_code == key_code);

                match triggered_option {
                    Some(LegendItem {
                        action: AppAction::GameAction(game_action),
                        ..
                    }) => {
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
                    Some(LegendItem {
                        action: AppAction::Quit,
                        ..
                    }) => {
                        return Ok(EventHandlerResult::Quit);
                    }
                    Some(LegendItem {
                        action: AppAction::Start,
                        ..
                    }) => {
                        return Ok(EventHandlerResult::Start);
                    }
                    Some(_) => {}
                    None => {}
                }
            }
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
                "Conecting..."
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
                board_render_state: BoardProps {
                    highest_played_card_for_suit: HashMap::new(),
                    discards: vec![],
                    draw_remaining: 0,
                    hints_remaining: 0,
                    fuse_remaining: 0,
                },
                players: players
                    .iter()
                    .map(|p| {
                        player_node_props(p.name.clone(), vec![None; 5], PlayerRenderState::Default)
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
                self.render_game_log(game_props.game_log)
                    .append_layout(|layout| LayoutStyle {
                        grid_row: line(1),
                        grid_column: line(2),

                        ..layout
                    }),
                VStack::new()
                    .layout(LayoutStyle {
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

                            grid_row: line(2),
                            grid_column: span(2),
                            ..HStack::default_layout()
                        },
                        legend.into_iter().map(game_action_item_tree).collect_vec(),
                    )),
            ],
        )
    }

    fn render_game_log(&self, log: Vec<Line<'static>>) -> Node<'static> {
        use taffy::prelude::*;

        let log_color = Color::Gray;

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
                    self.game_log_scroll_adjust,
                )
                .scrollable(AppAction::ScrollGameLog(1), AppAction::ScrollGameLog(-1))],
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

        if game_state.current_turn_player_index != game_state.this_client_player_index {
            return (
                format!(
                    "{}'s turn",
                    players[game_state.current_turn_player_index.0].name
                ),
                vec![],
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
            CommandBuilder::Empty => (
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
                ]
                .into_iter()
                .flatten()
                .collect(),
            ),
            CommandBuilder::Hinting(HintState::ChoosingPlayer) => (
                "Choose a player index".to_string(),
                (0..game_state.players.len())
                    .filter(|&index| game_state.current_turn_player_index.0 != index)
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
            | CommandBuilder::DiscardingCard(CardState::ChoosingCard { card_type }) => {
                let (action, description) = match card_type {
                    CardBuilderType::Play => ("Play", "Choose a card to play"),
                    CardBuilderType::Discard => ("Discard", "Choose a card to send to the bin"),
                };
                match game_state
                    .players
                    .get(game_state.current_turn_player_index.0)
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
    players: &Vec<OnlinePlayer>,
) -> Vec<Line<'static>> {
    use shared::model::GameEffect as Eff;
    use shared::model::GameEvent as Ev;

    let self_player = game_state.this_client_player_index.0;

    let player_name_span = |player_index: usize| {
        let name = &players[player_index].name;
        Span::from(name[..8.min(name.len())].to_string())
            .style(default_style().fg(if player_index == self_player {
                SELECTION_COLOR
            } else {
                Color::White
            }))
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
        .fg(NORMAL_TEXT)
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

    fn hint_spans(hint_type: HintAction, effects: &Vec<GameEffect>) -> Vec<Span<'static>> {
        hint_slots(&effects)
            .into_iter()
            .map(|_| match hint_type {
                HintAction::SameFace(face) => face_span(face),
                HintAction::SameSuit(suit) => suit_span(suit),
            })
            // .intersperse(Span::raw(" "))
            .collect_vec()
    }

    fn hint_count(effects: &Vec<GameEffect>) -> usize {
        // match effects
        //     .iter()
        //     .filter(|e| matches!(e, Eff::HintCard(_, _, _)))
        //     .count()
        // {
        //     1 => Span::from(" 1x "),
        //     2 => Span::from(" 2x "),
        //     3 => Span::from(" 3x "),
        //     4 => Span::from(" 4x "),
        //     5 => Span::from(" 5x "),
        //     _ => panic!("invalid hint count"),
        // }
        // .fg(NORMAL_TEXT)
        effects
            .iter()
            .filter(|e| matches!(e, Eff::HintCard(_, _, _)))
            .count()
    }

    fn result_span(effects: &Vec<GameEffect>) -> Span<'static> {
        let result = effects.iter().find_map(|effect| match effect {
            Eff::BurnFuse => Some(" (\u{f1052} \u{f0691} ) "),
            Eff::IncHint => Some(" (\u{f15cb} \u{f017} ) "),
            Eff::MarkLastTurn(_) => Some(" LAST ROUND!"),
            _ => None,
        });

        match result {
            Some(result) => Span::from(result).style(default_style()),
            None => Span::raw(""),
        }
    }

    fn count_span(i: usize) -> Span<'static> {
        Span::raw(format!("{}. ", i + 1))
    }

    let game_log_iter = game_state
        .log
        .iter()
        .enumerate()
        .map(|(turn_index, game_log)| match game_log {
            GameEvent::PlayerAction {
                player_index: PlayerIndex(player_index),
                action: PlayerAction::PlayCard(SlotIndex(index)),
                effects,
            } => [
                count_span(turn_index),
                player_name_span(*player_index),
                Span::raw(" played "),
                card(card_played(&effects)),
                result_span(&effects),
            ]
            .to_vec(),
            GameEvent::PlayerAction {
                player_index: PlayerIndex(player_index),
                action: PlayerAction::DiscardCard(SlotIndex(index)),
                effects,
            } => [
                count_span(turn_index),
                player_name_span(*player_index),
                Span::raw(" discarded "),
                card(card_played(&effects)),
                result_span(&effects),
            ]
            .to_vec(),
            GameEvent::PlayerAction {
                player_index: PlayerIndex(player_index),
                action: PlayerAction::GiveHint(PlayerIndex(hinted_index), hint),
                effects,
            } => [
                count_span(turn_index),
                player_name_span(*player_index),
                Span::raw(" hinted "),
                player_name_span(*hinted_index),
                Span::raw(" "),
            ]
            .into_iter()
            .chain(hint_spans(*hint, &effects))
            .collect_vec(),
            Ev::GameOver(outcome) => [
                Span::raw("Game Over: "),
                match outcome {
                    GameOutcome::Win => Span::raw("Victory!"),
                    GameOutcome::Fail { score } => Span::raw(format!("Defeat (score = {})", score)),
                },
            ]
            .to_vec(),
        });

    game_log_iter.map(|spans| Line::from(spans)).collect_vec()

    // let log_lines: Vec<String> = game_state
    //     .log
    //     .iter()
    //     .filter_map(|event| match event.to_owned() {
    //         Ev::PlayerAction(PlayerIndex(index), action) => {
    //             let player_name = players[index].name.clone().white();
    //             match action {
    //                 PlayerAction::PlayCard(SlotIndex(card)) => {}
    //                 PlayerAction::DiscardCard(SlotIndex(card)) => {
    //                     Some(format!("{} discarded card #{}", player_name, card))
    //                 }
    //                 PlayerAction::GiveHint(
    //                     PlayerIndex(hinted_player),
    //                     HintAction::SameFace(face),
    //                 ) => Some(format!(
    //                     "{} gave a hint on {}'s {}",
    //                     player_name,
    //                     players[hinted_player].name.clone().white(),
    //                     face.key().bold()
    //                 )),
    //                 PlayerAction::GiveHint(
    //                     PlayerIndex(hinted_player),
    //                     HintAction::SameSuit(suit),
    //                 ) => Some(format!(
    //                     "{} gave a hint on {}'s {}",
    //                     player_name,
    //                     players[hinted_player].name.clone().white(),
    //                     suit.key().fg(colorize_suit(suit)).bold()
    //                 )),
    //             }
    //         }
    //         Ev::GameEffect(effect) => match effect {
    //             Eff::AddToDiscrard(Card { suit, face }) => Some(format!(
    //                 "{} added to discard pile",
    //                 face.key().fg(colorize_suit(suit)).bold()
    //             )),
    //             GameEffect::DrawCard(PlayerIndex(player), _) => {
    //                 Some(format!("{} drew a card", players[player].name))
    //             }
    //             GameEffect::RemoveCard(_, _) => None,
    //             GameEffect::PlaceOnBoard(Card { face, suit }) => {
    //                 Some(format!("{}{} added to the board", suit.key(), face.key()))
    //             }
    //             GameEffect::HintCard(_, _, _) => None,
    //             GameEffect::DecHint => None,
    //             GameEffect::IncHint => Some("+1 hint".to_string()),
    //             GameEffect::BurnFuse => Some("-1 fuse".to_string()),
    //             GameEffect::NextTurn(PlayerIndex(player)) => {
    //                 Some(format!("{}'s turn", players[player].name))
    //             }
    //         },
    //         Ev::GameOver(outcome) => Some(match outcome {
    //             GameOutcome::Win => format!("Victory!"),
    //             GameOutcome::Fail { score } => format!("Defeat (score = {})", score),
    //         }),
    //     })
    //     .collect_vec();
    // log_lines
}

#[derive(Debug, Clone, Copy)]
pub enum AppAction {
    Start,
    Quit,
    GameAction(GameAction),
    ScrollGameLog(i8),
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

        _ => panic!("Unknown keycode"),
    };

    Span::from(item_text)
        .style(default_style().bg(SELECTION_COLOR).fg(Color::White))
        .touchable(item.action)
        .keybinding(item.key_code, item.action)
}

fn board_node_props(game_state_snapshot: &GameStateSnapshot) -> BoardProps {
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
            let mut card_faces: Vec<_> = game_state_snapshot
                .played_cards
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
        discards: game_state_snapshot.discard_pile.clone(),
        draw_remaining: game_state_snapshot.draw_pile_count as usize,
        hints_remaining: game_state_snapshot.remaining_hint_count as usize,
        fuse_remaining: game_state_snapshot.remaining_bomb_count as usize,
    }
}

fn slot_node_props(card: Option<Card>, hints: Vec<Hint>) -> SlotNodeProps {
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
        all_hints: hints.clone(),
        card: CardNodeProps::SomeCard(face, suit),
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
    name: String,
    hand: Vec<Option<(Option<Card>, Vec<Hint>)>>,
    player_state: PlayerRenderState,
) -> PlayerNodeProps {
    // let player = &game_state.players[player_index];

    // let hand_size = match player {
    //     ClientPlayerView::Me { hand, .. } => hand.len(),
    //     ClientPlayerView::Teammate { hand, .. } => hand.len(),
    // };

    let slot_props = hand
        .into_iter()
        .map(|slot| match &slot {
            Some((card, hints)) => slot_node_props(card.clone(), hints.clone()),
            None => SlotNodeProps {
                card: CardNodeProps::Empty,
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

struct GameProps {
    board_render_state: BoardProps,
    players: Vec<PlayerNodeProps>,
    game_log: Vec<Line<'static>>,
}

impl From<HanabiApp> for GameProps {
    fn from(app_state: HanabiApp) -> Self {
        match &app_state.client_state {
            HanabiClient::Connecting => todo!(),
            HanabiClient::Loaded(game) => match game {
                HanabiGame::Lobby { .. } => todo!(),
                HanabiGame::Started {
                    players,
                    game_state,
                    ..
                } => GameProps {
                    game_log: generate_game_log(game_state, players),
                    board_render_state: board_node_props(game_state),
                    players: (0..players.len())
                        .into_iter()
                        .map(|player_index| {
                            let player_state = match (
                                game_state.current_turn_player_index,
                                &app_state.command.current_command,
                            ) {
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
                                _ => PlayerRenderState::Default,
                            };

                            match &game_state.players[player_index] {
                                ClientPlayerView::Me { name, hand } => player_node_props(
                                    name.clone(),
                                    hand.iter()
                                        .map(|h| h.clone().map(|c| (None, c.hints.clone())))
                                        .collect(),
                                    player_state,
                                ),
                                ClientPlayerView::Teammate { name, hand } => player_node_props(
                                    name.clone(),
                                    hand.iter()
                                        .map(|h| h.clone().map(|c| (Some(c.card), c.hints.clone())))
                                        .collect(),
                                    player_state,
                                ),
                            }
                        })
                        .collect(),
                },
                HanabiGame::Ended {
                    players,
                    game_state,
                    revealed_game_state,
                    ..
                } => GameProps {
                    game_log: generate_game_log(game_state, players),
                    board_render_state: board_node_props(game_state),
                    players: (0..players.len())
                        .into_iter()
                        .map(|player_index| {
                            let player_state = match (
                                game_state.current_turn_player_index,
                                &app_state.command.current_command,
                            ) {
                                (PlayerIndex(turn), _) if turn as usize == player_index => {
                                    PlayerRenderState::CurrentTurn
                                }

                                _ => PlayerRenderState::Default,
                            };

                            player_node_props(
                                players[player_index].name.clone(),
                                revealed_game_state.players[player_index]
                                    .hand
                                    .iter()
                                    .map(|h| h.clone().map(|c| (Some(c.card), c.hints.clone())))
                                    .collect(),
                                player_state,
                            )
                        })
                        .collect(),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_data::*;

    use super::*;

    #[test]
    fn test_game_ui() {
        use ratatui::prelude as ratatui;

        let players = vec![
            OnlinePlayer {
                name: "p1".into(),
                connection_status: ConnectionStatus::Connected,
                is_host: true,
            },
            OnlinePlayer {
                name: "p2".into(),
                connection_status: ConnectionStatus::Connected,
                is_host: true,
            },
        ];
        let app = HanabiApp {
            exit: false,
            command: CommandState {
                current_command: CommandBuilder::Empty,
            },
            client_state: HanabiClient::Loaded(HanabiGame::Started {
                session_id: "1".into(),
                players: players.clone(),
                game_state: generate_minimal_test_game_state(),
            }),
            game_log_scroll_adjust: 0,
        };

        let mut buf = Buffer::empty(ratatui::Rect {
            x: 0,
            y: 0,
            width: 248,
            height: 46,
        });

        let tree_widget = root_tree_widget(
            buf.area,
            app.game_ui(app.clone().into(), "".to_string(), vec![]),
        );

        tree_widget.render_ref(buf.area, &mut buf);

        println!(
            "top left corner = '{:?}' '{:?}' '{:?}'",
            buf.get(buf.area.width - 3, 0).symbol().chars(),
            buf.get(buf.area.width - 2, 0).symbol().chars(),
            buf.get(buf.area.width - 1, 0).symbol().chars()
        );

        println!(
            "top left corner = '{:?}' '{:?}' '{:?}'",
            buf.get(buf.area.width - 3, 1).symbol().chars(),
            buf.get(buf.area.width - 2, 1).symbol().chars(),
            buf.get(buf.area.width - 1, 1).symbol().chars()
        );
    }

    #[test]
    fn test_panic_case_ui() {
        use ratatui::prelude as ratatui;

        let app_data = generate_example_panic_case_2();

        let app = HanabiApp {
            exit: false,
            command: CommandState {
                current_command: CommandBuilder::Empty,
            },
            client_state: HanabiClient::Loaded(app_data.clone()),
            game_log_scroll_adjust: 0,
        };
        // let mut tree = TreeWidget::new();
        // let root_id = tree.add_tree(Stack::new().children(
        //     LayoutStyle {
        //         size: Size {
        //             width: length(100. as f32),
        //             height: length(100. as f32),
        //         },
        //         padding: padding(2.),
        //         ..Stack::default_layout()
        //     },
        //     vec![app.game_ui(&generate_minimal_test_game_state(), None, &players)],
        // ));

        let mut buf = Buffer::empty(ratatui::Rect {
            x: 0,
            y: 0,
            width: 156,
            height: 38,
        });

        match app_data {
            HanabiGame::Started {
                session_id: _,
                players,
                game_state,
            } => {
                let tree_widget = root_tree_widget(
                    buf.area,
                    app.game_ui(app.clone().into(), "".to_string(), vec![]),
                );
                tree_widget.render_ref(buf.area, &mut buf);
            }
            _ => todo!(),
        }
    }
}
