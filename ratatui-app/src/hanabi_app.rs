use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};
use ratatui::{style::Stylize, widgets::WidgetRef, Frame, Terminal};
use std::{char::from_digit, collections::HashMap, error::Error, iter, time::Duration};
use taffy::{Overflow, Point};

use crate::{
    components::*,
    key_code::KeyCode,
    nodes::{GridStack, HStack, LayoutRect, LayoutStyle, Node, NodeBuilder, Stack, VStack},
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

pub struct HanabiApp {
    pub exit: bool,
    command: CommandState,
    // menu_options: StatefulList,
    game_state: HanabiClient,
    connection: Option<Duration>,
    // game_state: BrowsingLobby | CreatingGame | GameLobby |
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
    tree.print_tree();

    tree
}

impl HanabiApp {
    pub fn new(game_state: HanabiClient) -> Self {
        HanabiApp {
            exit: false,
            command: CommandState {
                current_command: CommandBuilder::Empty,
            },
            game_state: game_state,
            connection: None,
            game_log_scroll_adjust: 0,
        }
    }

    /// runs the application's main loop until the user quits
    pub fn draw<T>(&mut self, terminal: &mut Terminal<T>) -> BoxedResult<()>
    where
        T: ratatui::backend::Backend,
    {
        // while !self.exit {

        terminal.draw(|frame| self.ui(frame))?;
        // self.handle_events()?;
        // }

        Ok(())
    }

    pub fn update(&mut self, state: HanabiClient) {
        self.game_state = state;
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
                let options = self.legend_for_command_state(&self.game_state);
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
                    None => {}
                }
            }
        }

        if self.exit {
            return Ok(EventHandlerResult::Quit);
        }

        Ok(EventHandlerResult::Continue)
    }

    fn ui(&mut self, frame: &mut Frame) {
        match &self.game_state {
            HanabiClient::Connecting => self.connecting_ui(frame),
            HanabiClient::Loaded(HanabiGame::Lobby { players, .. }) => {
                self.lobby_ui(players, frame);
            }
            HanabiClient::Loaded(HanabiGame::Started {
                game_state,
                players,
                ..
            }) => {
                let tree = root_tree_widget(frame.size(), self.game_ui(game_state, None, players));

                let area = frame.size();
                frame.buffer_mut().set_style(area, default_style());

                tree.render_ref(frame.size(), frame.buffer_mut());
            }

            HanabiClient::Loaded(HanabiGame::Ended {
                players,
                game_state,
                revealed_game_state,
                ..
            }) => {
                self.game_ui(game_state, Some(revealed_game_state), players);
            }
        }
    }

    fn connecting_ui(&self, frame: &mut Frame) {
        let text: Text = Text::from(if self.exit {
            "Exiting...".to_string()
        } else {
            "Conecting...".to_string()
        });
        let log = Paragraph::new(text);

        frame.render_widget(log, frame.size());
    }

    fn lobby_ui(&self, players: &Vec<OnlinePlayer>, frame: &mut Frame) {
        let lobby_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Game Lobby");

        let mut contents: Vec<String> = Vec::new();

        let connection = match self.connection {
            Some(time) => {
                let elapsed = time.as_millis();
                format!("Connected. Last ping {}ms", elapsed)
            }
            None => "Connecting...".to_string(),
        };
        contents.push(connection);

        let content: Vec<_> = players.iter().map(|p| p.name.clone()).collect();
        contents.extend(content);

        let text = Text::from_iter(contents);
        let players_paragraph = Paragraph::new(text).block(lobby_block);

        frame.render_widget(players_paragraph, frame.size());
    }

    fn game_ui(
        &self,
        game_state: &GameStateSnapshot,
        _full_game_state: Option<&GameState>, // TODO implement end game state
        players: &Vec<OnlinePlayer>,
    ) -> Node<'static> {
        use taffy::prelude::*;

        let player_data = |game_state: &GameStateSnapshot, player_index: usize| {
            let player = &game_state.players[player_index];

            let hand_size = match player {
                ClientPlayerView::Me { hand } => hand.len(),
                ClientPlayerView::Teammate { hand } => hand.len(),
            };

            let slot_props = (0..hand_size)
                .into_iter()
                .map(|slot_index| {
                    let has_card = match &player {
                        ClientPlayerView::Me { hand } => hand[slot_index].is_some(),
                        ClientPlayerView::Teammate { hand } => hand[slot_index].is_some(),
                    };

                    if !has_card {
                        return SlotNodeProps {
                            card: CardNodeProps::Empty,
                            all_hints: vec![],
                            face_hint: None,
                            suit_hint: None,
                            unique_hints: vec![],
                            unique_not_hints: vec![],
                        };
                    }

                    let hints = match &player {
                        ClientPlayerView::Me { hand } => {
                            hand[slot_index].as_ref().map(|h| h.hints.clone())
                        }
                        ClientPlayerView::Teammate { hand } => {
                            hand[slot_index].as_ref().map(|h| h.hints.clone())
                        }
                    };

                    let (suit, face) = match player {
                        ClientPlayerView::Me { hand } => hand[slot_index].as_ref().map(|c| {
                            let suit = c.hints.iter().find_map(|&h| match h {
                                Hint::IsSuit(suit) => Some(suit),
                                _ => None,
                            });

                            let face = c.hints.iter().find_map(|&h| match h {
                                Hint::IsFace(face) => Some(face),
                                _ => None,
                            });

                            (suit, face)
                        }),
                        ClientPlayerView::Teammate { hand } => hand[slot_index]
                            .as_ref()
                            .map(|s| (Some(s.card.suit), Some(s.card.face))),
                    }
                    .unwrap_or((None, None));

                    SlotNodeProps {
                        all_hints: hints.clone().unwrap_or_default().to_vec(),
                        card: CardNodeProps::SomeCard(face, suit),
                        face_hint: hints.clone().unwrap_or_default().into_iter().find(
                            |h| match h {
                                Hint::IsFace(_) => true,
                                _ => false,
                            },
                        ), //face.map(|f| Hint::IsFace(f)),
                        suit_hint: hints.clone().unwrap_or_default().into_iter().find(
                            |h| match h {
                                Hint::IsSuit(_) => true,
                                _ => false,
                            },
                        ),
                        unique_hints: hints
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|h| match h {
                                Hint::IsSuit(_) | Hint::IsFace(_) => true,
                                _ => false,
                            })
                            .unique()
                            .collect(),
                        unique_not_hints: hints
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|h| match h {
                                Hint::IsNotSuit(_) | Hint::IsNotFace(_) => true,
                                _ => false,
                            })
                            .unique()
                            .collect(),
                    }
                })
                .collect_vec();

            PlayerNodeProps {
                name: players[player_index].name.clone(),
                hand: slot_props,
                state: match (game_state.turn, &self.command.current_command) {
                    (PlayerIndex(turn), _) if turn as usize == player_index => {
                        PlayerRenderState::CurrentTurn
                    }
                    (
                        _,
                        &CommandBuilder::Hint(HintState::ChoosingHint {
                            player_index: command_player_index,
                        }),
                    ) if command_player_index as usize == player_index => {
                        PlayerRenderState::CurrentSelection
                    }
                    _ => PlayerRenderState::Default,
                },
            }
        };

        let all_suits = [
            CardSuit::Blue,
            CardSuit::Green,
            CardSuit::Red,
            CardSuit::White,
            CardSuit::Yellow,
        ];

        let board_data = |game_state_snapshot: &GameStateSnapshot| {
            let highest_cards = all_suits
                .iter()
                .enumerate()
                .map(|(_, &cur_suit)| {
                    let mut card_faces: Vec<_> = game_state
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
        };

        GridStack::new().children(
            LayoutStyle {
                grid_template_columns: vec![fr(1.), length(40.)],
                grid_template_rows: vec![fr(1.), length(3.)],
                padding: LayoutRect {
                    top: length(1.),
                    left: length(4.),
                    right: length(10.),
                    bottom: length(1.),
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
                            (0..game_state.players.len())
                                .into_iter()
                                .map(|i| player_node(player_data(game_state, i)))
                                .collect_vec(),
                        ),
                        board_node_tree(board_data(game_state)),
                    ]),
                ),
                self.render_game_log(generate_game_log(game_state, players))
                    .append_layout(|layout| LayoutStyle {
                        grid_row: line(1),
                        grid_column: line(2),

                        ..layout
                    }),
                HStack::new().children(
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
                    self.legend_for_command_state(&self.game_state)
                        .iter()
                        .map(game_action_item_tree)
                        .collect_vec(),
                ),
            ],
        )

        // Old layout
        // HStack::new().children(
        //     LayoutStyle {
        //         size: Size {
        //             width: percent(1.),
        //             height: percent(1.),
        //         },

        //         justify_content: Some(JustifyContent::Center),
        //         align_items: Some(AlignItems::Center),
        //         ..HStack::default_layout()
        //     },
        //     [VStack::new().children(
        //         LayoutStyle {
        //             size: Size {
        //                 width: auto(),
        //                 height: auto(),
        //             },
        //             gap: Size {
        //                 width: length(0.),
        //                 height: length(1.),
        //             },
        //             ..VStack::default_layout()
        //         },
        //         [
        //             HStack::new().children(
        //                 LayoutStyle {
        //                     size: Size {
        //                         width: auto(),
        //                         height: auto(),
        //                     },
        //                     gap: Size {
        //                         width: length(2.),
        //                         height: length(0.),
        //                     },
        //                     ..HStack::default_layout()
        //                 },
        //                 [
        //                     VStack::new().children(
        //                         LayoutStyle {
        //                             // padding: padding(2.),
        //                             gap: Size {
        //                                 width: length(0.),
        //                                 height: length(1.),
        //                             },
        //                             size: Size {
        //                                 width: auto(),
        //                                 height: auto(),
        //                             },
        //                             ..VStack::default_layout()
        //                         },
        //                         Vec::from([
        //                             HStack::new().children(
        //                                 LayoutStyle {
        //                                     justify_content: Some(JustifyContent::Center),
        //                                     size: Size {
        //                                         width: auto(),
        //                                         height: auto(),
        //                                     },
        //                                     gap: Size {
        //                                         width: length(1.),
        //                                         height: length(0.),
        //                                     },
        //                                     ..HStack::default_layout()
        //                                 },
        //                                 (0..game_state.players.len())
        //                                     .into_iter()
        //                                     .map(|i| player_node(player_data(game_state, i)))
        //                                     .collect_vec(),
        //                             ),
        //                             board_node_tree(board_data(game_state)),
        //                         ]),
        //                     ),
        //                     self.render_game_log(generate_game_log(game_state, players)),
        //                 ],
        //             ),
        //             HStack::new().children(
        //                 LayoutStyle {
        //                     size: Size {
        //                         width: auto(),
        //                         height: length(3.),
        //                     },
        //                     gap: Size {
        //                         width: length(1.),
        //                         height: length(0.),
        //                     },
        //                     justify_content: Some(JustifyContent::Center),
        //                     ..HStack::default_layout()
        //                 },
        //                 self.legend_for_command_state(&self.game_state)
        //                     .iter()
        //                     .map(game_action_item_tree)
        //                     .collect_vec(),
        //             ),
        //         ],
        //     )],
        // )
    }

    fn render_game_log(&self, log: Vec<String>) -> Node<'static> {
        use taffy::prelude::*;

        let log_color = Color::Gray;

        let lines: Vec<Span> = log
            .iter()
            .map(|line| Span::from(format!("{}", line)).style(default_style().fg(NORMAL_TEXT)))
            .collect_vec();

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
                    Text::from(lines.into_iter().map(|l| l.into()).collect_vec()),
                    self.game_log_scroll_adjust,
                )],
            )
    }

    fn legend_for_command_state(&self, game_state: &HanabiClient) -> Vec<LegendItem> {
        use KeyCode::*;
        match game_state {
            HanabiClient::Connecting { .. } => {
                return vec![LegendItem {
                    desc: format!("Quit"),
                    key_code: KeyCode::Esc,
                    action: AppAction::Quit,
                }];
            }
            HanabiClient::Loaded(game_state) => match game_state {
                HanabiGame::Lobby { .. } => {
                    return vec![
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
                    ];
                }
                HanabiGame::Started {
                    game_state,
                    players,
                    ..
                } => self.legend_for_command_state_game(game_state, players),

                HanabiGame::Ended { .. } => {
                    return vec![LegendItem {
                        desc: format!("Quit"),
                        key_code: KeyCode::Esc,
                        action: AppAction::Quit,
                    }];
                }
            },
        }
    }

    fn legend_for_command_state_game(
        &self,
        game_state: &GameStateSnapshot,
        players: &Vec<OnlinePlayer>,
    ) -> Vec<LegendItem> {
        if let Some(_) = &game_state.outcome {
            return vec![LegendItem {
                desc: format!("Quit"),
                key_code: KeyCode::Esc,
                action: AppAction::Quit,
            }];
        }

        if game_state.turn != game_state.player_snapshot {
            return vec![];
        }

        use KeyCode::*;
        match self.command.current_command {
            CommandBuilder::Empty => [
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
                    desc: "Undo".to_string(),
                    key_code: Char('u'),
                    action: AppAction::GameAction(GameAction::Undo),
                }),
            ]
            .into_iter()
            .flatten()
            .collect(),
            CommandBuilder::Hint(HintState::ChoosingPlayer) => (0..game_state.players.len())
                .filter(|&index| game_state.turn.0 != index)
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

            CommandBuilder::Hint(HintState::ChoosingHint { .. }) => vec![
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

            CommandBuilder::Play(CardState::ChoosingCard { card_type })
            | CommandBuilder::Discard(CardState::ChoosingCard { card_type }) => {
                let action = match card_type {
                    CardBuilderType::Play => "Play",
                    CardBuilderType::Discard => "Discard",
                };
                match game_state.players.get(game_state.turn.0) {
                    Some(ClientPlayerView::Me { hand }) => hand
                        .iter()
                        .enumerate()
                        .filter(|(_, slot)| slot.is_some())
                        .map(|(index, _)| LegendItem {
                            desc: format!("{} #{}", action, index + 1),
                            key_code: Char(from_digit(index as u32 + 1, 10).unwrap()),
                            action: AppAction::GameAction(GameAction::SelectCard(SlotIndex(index))),
                        })
                        .chain(iter::once(LegendItem {
                            desc: "Back".to_string(),
                            key_code: Backspace,
                            action: AppAction::GameAction(GameAction::Undo),
                        }))
                        .collect(),
                    _ => panic!("Shouldn't be able to play as another player"),
                }
            }
        }
    }
}

fn generate_game_log(game_state: &GameStateSnapshot, players: &Vec<OnlinePlayer>) -> Vec<String> {
    use shared::model::GameEffect as Eff;
    use shared::model::GameEvent as Ev;
    let log_lines: Vec<String> = game_state
        .log
        .iter()
        .filter_map(|event| match event.to_owned() {
            Ev::PlayerAction(PlayerIndex(index), action) => {
                let player_name = players[index].name.clone().white();
                match action {
                    PlayerAction::PlayCard(SlotIndex(card)) => {
                        Some(format!("{} played card #{}", player_name, card))
                    }
                    PlayerAction::DiscardCard(SlotIndex(card)) => {
                        Some(format!("{} discarded card #{}", player_name, card))
                    }
                    PlayerAction::GiveHint(
                        PlayerIndex(hinted_player),
                        HintAction::SameFace(face),
                    ) => Some(format!(
                        "{} gave a hint on {}'s {}",
                        player_name,
                        players[hinted_player].name.clone().white(),
                        face.key().bold()
                    )),
                    PlayerAction::GiveHint(
                        PlayerIndex(hinted_player),
                        HintAction::SameSuit(suit),
                    ) => Some(format!(
                        "{} gave a hint on {}'s {}",
                        player_name,
                        players[hinted_player].name.clone().white(),
                        suit.key().fg(colorize_suit(suit)).bold()
                    )),
                }
            }
            Ev::GameEffect(effect) => match effect {
                Eff::AddToDiscrard(Card { suit, face }) => Some(format!(
                    "{} added to discard pile",
                    face.key().fg(colorize_suit(suit)).bold()
                )),
                GameEffect::DrawCard(PlayerIndex(player), _) => {
                    Some(format!("{} drew a card", players[player].name))
                }
                GameEffect::RemoveCard(_, _) => None,
                GameEffect::PlaceOnBoard(Card { face, suit }) => {
                    Some(format!("{}{} added to the board", suit.key(), face.key()))
                }
                GameEffect::HintCard(_, _, _) => None,
                GameEffect::DecHint => None,
                GameEffect::IncHint => Some("+1 hint".to_string()),
                GameEffect::BurnFuse => Some("-1 fuse".to_string()),
                GameEffect::NextTurn(PlayerIndex(player)) => {
                    Some(format!("{}'s turn", players[player].name))
                }
            },
        })
        .collect_vec();
    log_lines
}

pub enum AppAction {
    Start,
    Quit,
    GameAction(GameAction),
}

struct LegendItem {
    desc: String,
    key_code: KeyCode,
    action: AppAction,
}

fn game_action_item_tree(item: &LegendItem) -> Node<'static> {
    let item_text = |a: &LegendItem| match a {
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

        _ => panic!("Unknown keycode"),
    };

    Span::from(item_text(item))
        .style(default_style().bg(SELECTION_COLOR).fg(Color::White))
        .into()
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
            game_state: HanabiClient::Loaded(HanabiGame::Started {
                session_id: "1".into(),
                players: players.clone(),
                game_state: generate_minimal_test_game_state(),
            }),
            connection: None,
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
            app.game_ui(&generate_minimal_test_game_state(), None, &players),
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
            game_state: HanabiClient::Loaded(app_data.clone()),
            connection: None,
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
                let tree_widget =
                    root_tree_widget(buf.area, app.game_ui(&game_state, None, &players));
                tree_widget.render_ref(buf.area, &mut buf);
            }
            _ => todo!(),
        }
    }
}
