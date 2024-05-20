use ratatui::{style::Stylize, Frame, Terminal};
use shared::model::*;
use std::{char::from_digit, error::Error, iter, time::Duration};
// use tui_big_text::{BigText, PixelSize};

use shared::model::{ClientPlayerView, GameStateSnapshot};

use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use shared::client_logic::*;

type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;

trait CardKey {
    fn key(&self) -> &'static str;
}

impl CardKey for CardSuit {
    fn key(&self) -> &'static str {
        match self {
            CardSuit::Red => "R",
            CardSuit::Green => "G",
            CardSuit::Yellow => "Y",
            CardSuit::White => "W",
            CardSuit::Blue => "B",
        }
    }
}

impl CardKey for CardFace {
    fn key(&self) -> &'static str {
        match self {
            CardFace::One => "1",
            CardFace::Two => "2",
            CardFace::Three => "3",
            CardFace::Four => "4",
            CardFace::Five => "5",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    /// Backspace key.
    Backspace,
    /// Enter key.
    Enter,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page up key.
    PageUp,
    /// Page down key.
    PageDown,
    /// Tab key.
    Tab,
    /// Shift + Tab key.
    BackTab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// F key.
    ///
    /// `KeyCode::F(1)` represents F1 key, etc.
    F(u8),
    /// A character.
    ///
    /// `KeyCode::Char('c')` represents `c` character, etc.
    Char(char),
    /// Null.
    Null,
    /// Escape key.
    Esc,
    /// Caps Lock key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    CapsLock,
    /// Scroll Lock key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    ScrollLock,
    /// Num Lock key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    NumLock,
    /// Print Screen key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    PrintScreen,
    /// Pause key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    Pause,
    /// Menu key.
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    Menu,
    /// The "Begin" key (often mapped to the 5 key when Num Lock is turned on).
    ///
    /// **Note:** this key can only be read if
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    KeypadBegin,
    // Modifier(ModifierKeyCode),
}

static BACKGROUND_COLOR: Color = Color::Rgb(36, 37, 47);
static SELECTION_COLOR: Color = Color::Rgb(117, 158, 179);
static TURN_COLOR: Color = Color::Rgb(239, 119, 189);
static NORMAL_TEXT: Color = Color::Rgb(255, 255, 255);
static DIM_TEXT: Color = Color::Rgb(100, 100, 100);

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
}

pub enum EventHandlerResult {
    PlayerAction(PlayerAction),
    Quit,
    Continue,
    Start,
}

struct GameLayout {
    players: Vec<Rect>,
    board: Rect,
    game_log: Rect,
}

fn default_style() -> Style {
    Style::default().fg(NORMAL_TEXT).bg(BACKGROUND_COLOR)
}

fn default_dim_style() -> Style {
    default_style().not_bold().fg(Color::Gray)
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
        frame.render_widget(
            Paragraph::new("")
                .style(default_style())
                .alignment(Alignment::Center),
            frame.size(),
        );

        match &self.game_state {
            HanabiClient::Connecting => self.connecting_ui(frame),
            HanabiClient::Loaded(HanabiGame::Lobby { players, log }) => {
                self.lobby_ui(players, frame);

                self.render_game_log(
                    log,
                    frame,
                    Rect {
                        x: 14 * 4 + 2,
                        y: 2,
                        width: frame.size().width - 14 * 4,
                        height: 30,
                    },
                );
            }
            HanabiClient::Loaded(HanabiGame::Started {
                game_state,
                players,
            }) => {
                self.game_ui(game_state, None, players, frame);
            }

            HanabiClient::Loaded(HanabiGame::Ended {
                players,
                game_state,
                revealed_game_state,
            }) => {
                self.game_ui(game_state, Some(revealed_game_state), players, frame);
            }
        }

        // match &self.game_state {
        //     HanabiGame::Connecting { log } => self.connecting_ui(frame),
        //      => {
        //         self.lobby_ui(players, frame);

        //         self.render_game_log(
        //             log,
        //             frame,
        //             Rect {
        //                 x: 14 * 4 + 2,
        //                 y: 2,
        //                 width: frame.size().width - 14 * 4,
        //                 height: 30,
        //             },
        //         );
        //     }
        //     HanabiGame::Started {
        //         game_state,
        //         players,
        //     } => {
        //         self.game_ui(game_state, players, frame);
        //     }
        // }
    }

    fn connecting_ui(&self, frame: &mut Frame) {
        let text: Text = Text::from(if self.exit {
            "Exiting...".to_string()
        } else {
            "Conecting...".to_string()
        });
        let log = Paragraph::new(text);

        frame.render_widget(log, frame.size());

        self.render_game_actions(
            frame,
            Rect {
                x: 14 * 4 + 2,
                y: 30 + 2,
                width: frame.size().width - 14 * 4,
                height: 1,
            },
        );
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

        self.render_game_actions(
            frame,
            Rect {
                x: 14 * 4 + 2,
                y: 30 + 2,
                width: frame.size().width - 14 * 4,
                height: 1,
            },
        );
    }

    fn layout(&self, players: usize, hand_size: usize, frame: &mut Frame) -> GameLayout {
        use taffy::prelude::*;

        // First create an instance of TaffyTree
        let mut tree: TaffyTree<()> = TaffyTree::new();

        // Create a tree of nodes using `TaffyTree.new_leaf` and `TaffyTree.new_with_children`.
        // These functions both return a node id which can be used to refer to that node
        // The Style struct is used to specify styling information
        let board_node = tree
            .new_leaf(Style {
                size: Size {
                    width: length(14.0 * 4.0),
                    height: length(14.0),
                },
                flex_grow: 1.0,
                flex_shrink: 1.0,
                max_size: Size {
                    width: auto(),
                    height: length(14.0),
                },
                ..Default::default()
            })
            .unwrap();

        let player_nodes = (0..players)
            .into_iter()
            .map(|_| {
                tree.new_leaf(Style {
                    size: Size {
                        width: length(hand_size as f32 * 3.0 + 2.0),
                        height: length(16.0),
                    },
                    flex_grow: 0.0,
                    ..Default::default()
                })
                .unwrap()
            })
            .collect_vec();

        let player_container_node = tree
            .new_with_children(
                Style {
                    flex_direction: FlexDirection::Row,
                    justify_content: Some(JustifyContent::Center),
                    size: Size {
                        width: auto(),
                        height: auto(),
                    },
                    ..Default::default()
                },
                player_nodes.as_slice(),
            )
            .unwrap();

        let left_pane = tree
            .new_with_children(
                Style {
                    flex_direction: FlexDirection::Column,
                    size: Size {
                        width: auto(),
                        height: auto(),
                    },
                    ..Default::default()
                },
                &[player_container_node, board_node],
            )
            .unwrap();

        let game_log = tree
            .new_leaf(Style {
                size: Size {
                    width: auto(),
                    height: auto(),
                },
                flex_grow: 1.0,
                ..Default::default()
            })
            .unwrap();

        let root_node = tree
            .new_with_children(
                Style {
                    flex_direction: FlexDirection::Row,
                    size: Size {
                        width: length(frame.size().width as f32),
                        height: length(frame.size().height as f32),
                    },
                    ..Default::default()
                },
                &[left_pane, game_log],
            )
            .unwrap();

        // Call compute_layout on the root of your tree to run the layout algorithm
        tree.compute_layout(
            root_node,
            Size {
                width: length(frame.size().width as f32),
                height: length(frame.size().height as f32),
            },
        )
        .unwrap();

        GameLayout {
            players: player_nodes
                .iter()
                .map(|p| {
                    let layout = tree.layout(*p).unwrap();
                    ratatui::layout::Rect {
                        x: layout.location.x as u16,
                        y: layout.location.y as u16,
                        width: layout.size.width as u16,
                        height: layout.size.height as u16,
                    }
                })
                .collect(),
            board: tree
                .layout(board_node)
                .map(|b| ratatui::layout::Rect {
                    x: b.location.x as u16,
                    y: b.location.y as u16,
                    width: b.size.width as u16,
                    height: b.size.height as u16,
                })
                .unwrap(),
            game_log: tree
                .layout(game_log)
                .map(|b| ratatui::layout::Rect {
                    x: b.location.x as u16,
                    y: b.location.y as u16,
                    width: b.size.width as u16,
                    height: b.size.height as u16,
                })
                .unwrap(),
        }
    }

    fn game_ui(
        &self,
        game_state: &GameStateSnapshot,
        full_game_state: Option<&GameState>,
        players: &Vec<OnlinePlayer>,
        frame: &mut Frame,
    ) {
        // let board_rect = Rect {
        //     x: 2,
        //     y: 18,
        //     width: 14 * 4,
        //     height: 14,
        // };

        // let player_rect = Rect {
        //     x: 0,
        //     y: 0,
        //     width: 14,
        //     height: 16,
        // };

        // let layout = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(
        //         [
        //             Constraint::Length(players.len() as u16 * player_rect.width),
        //             Constraint::Min(1),
        //         ]
        //         .into_iter(),
        //     )
        //     .split(frame.size());

        // let left_pane = layout[0];
        // let right_pane = layout[1];

        // let bottom_layout = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(
        //         [
        //             Constraint::Length(player_rect.height),
        //             Constraint::Length(board_rect.height),
        //         ]
        //         .into_iter(),
        //     )
        //     .split(left_pane);

        // let player_area_rect = bottom_layout[0];
        // let board_area_rect = bottom_layout[1];

        // let outer_layout = Layout::default()
        //     .direction(Direction::Vertical)
        //     .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        //     .split(f.size());

        // let inner_layout = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
        //     .split(outer_layout[1]);

        let game_layout = self.layout(players.len(), game_state.game_config.hand_size, frame);

        for (index, (client, layout)) in game_state
            .players
            .iter()
            .zip(game_layout.players)
            .enumerate()
        {
            render_player(
                client,
                &players[index].name,
                match (game_state.turn, &self.command.current_command) {
                    (PlayerIndex(turn), _) if turn as usize == index => {
                        PlayerRenderState::CurrentTurn
                    }
                    (
                        _,
                        &CommandBuilder::Hint(
                            HintState::ChoosingHintType { player_index }
                            | HintState::ChoosingFace { player_index }
                            | HintState::ChoosingSuit { player_index },
                        ),
                    ) if player_index as usize == index => PlayerRenderState::CurrentSelection,
                    _ => PlayerRenderState::Default,
                },
                frame,
                layout,
                // Rect {
                //     x: 2 + 14 * index as u16,
                //     y: 2,
                //     width: 4 * 3 + 2,
                //     height: 16,
                // },
            );
        }

        self.render_board(game_state, frame, game_layout.board);

        // frame.render_widget(
        //     Paragraph::new("1".white()).bg(Color::Cyan),
        //     // Paragraph::new("card 1".dark_gray())
        //     //     .wrap(Wrap { trim: true })
        //     //     .block(player),
        //     Rect {
        //         x: 5,
        //         y: 5,
        //         width: 1,
        //         height: 1,
        //     }, //layout[0][0],
        // );

        let log_lines = generate_game_log(game_state, players);

        // let outcome_lines = match &game_state.outcome {
        //     Some(outcome) => vec![Span::from(format!("Game Over: {:?}", outcome))
        //         .style(default_style().fg(log_color))],
        //     None => vec![],
        // };

        // let lines = initial
        //     .into_iter()
        //     .chain(lines.into_iter())
        //     .chain(outcome_lines.into_iter())
        //     .collect_vec();

        self.render_game_log(&log_lines, frame, game_layout.game_log);

        // frame.render_stateful_widget(
        //     ActionPicker {},
        //     Rect {
        //         x: 14 * 4 + 2,
        //         y: 30 + 2,
        //         width: frame.size().width - 14 * 4,
        //         height: 1,
        //     },
        //     &mut state,
        // );

        self.render_game_actions(
            frame,
            Rect {
                x: 14 * 4 + 2,
                y: 30 + 2,
                width: frame.size().width - 14 * 4,
                height: 1,
            },
        );

        // render_borders(&paragraph, Borders::ALL, frame, layout[0][1]);
        // render_borders(&paragraph, Borders::NONE, frame, layout[0][1]);
        // render_borders(&paragraph, Borders::LEFT, frame, layout[1][0]);
        // render_borders(&paragraph, Borders::RIGHT, frame, layout[1][1]);
        // render_borders(&paragraph, Borders::TOP, frame, layout[2][0]);
        // render_borders(&paragraph, Borders::BOTTOM, frame, layout[2][1]);

        // render_border_type(&paragraph, BorderType::Plain, frame, layout[3][0]);
        // render_border_type(&paragraph, BorderType::Rounded, frame, layout[3][1]);
        // render_border_type(&paragraph, BorderType::Double, frame, layout[4][0]);
        // render_border_type(&paragraph, BorderType::Thick, frame, layout[4][1]);

        // render_styled_block(&paragraph, frame, layout[5][0]);
        // render_styled_borders(&paragraph, frame, layout[5][1]);
        // render_styled_title(&paragraph, frame, layout[6][0]);
        // render_styled_title_content(&paragraph, frame, layout[6][1]);
        // render_multiple_titles(&paragraph, frame, layout[7][0]);
        // render_multiple_title_positions(&paragraph, frame, layout[7][1]);
        // render_padding(&paragraph, frame, layout[8][0]);
        // render_nested_blocks(&paragraph, frame, layout[8][1]);
    }

    fn endgame_ui(
        &self,
        game_state: &GameStateSnapshot,
        revealed_game_state: &GameState,
        players: &Vec<OnlinePlayer>,
        frame: &mut Frame,
    ) {
        let game_layout = self.layout(players.len(), game_state.game_config.hand_size, frame);

        for (index, (client, layout)) in game_state
            .players
            .iter()
            .zip(game_layout.players)
            .enumerate()
        {
            render_player(
                client,
                &players[index].name,
                PlayerRenderState::Default,
                frame,
                layout,
                // Rect {
                //     x: 2 + 14 * index as u16,
                //     y: 2,
                //     width: 4 * 3 + 2,
                //     height: 16,
                // },
            );
        }

        self.render_board(game_state, frame, game_layout.board);

        // frame.render_widget(
        //     Paragraph::new("1".white()).bg(Color::Cyan),
        //     // Paragraph::new("card 1".dark_gray())
        //     //     .wrap(Wrap { trim: true })
        //     //     .block(player),
        //     Rect {
        //         x: 5,
        //         y: 5,
        //         width: 1,
        //         height: 1,
        //     }, //layout[0][0],
        // );

        let log_lines = generate_game_log(game_state, players);

        // let outcome_lines = match &game_state.outcome {
        //     Some(outcome) => vec![Span::from(format!("Game Over: {:?}", outcome))
        //         .style(default_style().fg(log_color))],
        //     None => vec![],
        // };

        // let lines = initial
        //     .into_iter()
        //     .chain(lines.into_iter())
        //     .chain(outcome_lines.into_iter())
        //     .collect_vec();

        self.render_game_log(&log_lines, frame, game_layout.game_log);

        // frame.render_stateful_widget(
        //     ActionPicker {},
        //     Rect {
        //         x: 14 * 4 + 2,
        //         y: 30 + 2,
        //         width: frame.size().width - 14 * 4,
        //         height: 1,
        //     },
        //     &mut state,
        // );

        self.render_game_actions(
            frame,
            Rect {
                x: 14 * 4 + 2,
                y: 30 + 2,
                width: frame.size().width - 14 * 4,
                height: 1,
            },
        );

        // render_borders(&paragraph, Borders::ALL, frame, layout[0][1]);
        // render_borders(&paragraph, Borders::NONE, frame, layout[0][1]);
        // render_borders(&paragraph, Borders::LEFT, frame, layout[1][0]);
        // render_borders(&paragraph, Borders::RIGHT, frame, layout[1][1]);
        // render_borders(&paragraph, Borders::TOP, frame, layout[2][0]);
        // render_borders(&paragraph, Borders::BOTTOM, frame, layout[2][1]);

        // render_border_type(&paragraph, BorderType::Plain, frame, layout[3][0]);
        // render_border_type(&paragraph, BorderType::Rounded, frame, layout[3][1]);
        // render_border_type(&paragraph, BorderType::Double, frame, layout[4][0]);
        // render_border_type(&paragraph, BorderType::Thick, frame, layout[4][1]);

        // render_styled_block(&paragraph, frame, layout[5][0]);
        // render_styled_borders(&paragraph, frame, layout[5][1]);
        // render_styled_title(&paragraph, frame, layout[6][0]);
        // render_styled_title_content(&paragraph, frame, layout[6][1]);
        // render_multiple_titles(&paragraph, frame, layout[7][0]);
        // render_multiple_title_positions(&paragraph, frame, layout[7][1]);
        // render_padding(&paragraph, frame, layout[8][0]);
        // render_nested_blocks(&paragraph, frame, layout[8][1]);
    }

    fn render_board(&self, game_state: &GameStateSnapshot, frame: &mut Frame, area: Rect) {
        let game_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Board");
        let inner_rect = game_block.inner(area);

        let all_suits = [
            CardSuit::Blue,
            CardSuit::Green,
            CardSuit::Red,
            CardSuit::White,
            CardSuit::Yellow,
        ];

        for (suit_index, &cur_suit) in all_suits.iter().enumerate() {
            let mut card_faces: Vec<_> = game_state
                .played_cards
                .iter()
                .filter_map(|c| match c {
                    &Card { suit, face } if suit == cur_suit => Some(face),
                    _ => None,
                })
                .collect();
            card_faces.sort();

            match card_faces.as_slice() {
                [] => {
                    let placeholder_ui = render_placeholder(Some(cur_suit));

                    let x = inner_rect.x + suit_index as u16 * 4 + 2;
                    let y = inner_rect.y;

                    frame.render_widget(
                        placeholder_ui,
                        Rect {
                            x: x,
                            y: y,
                            width: 3,
                            height: 3,
                        },
                    )
                }
                [rest @ ..] => {
                    for (face_index, &cur_face) in rest.iter().enumerate() {
                        let card_ui = render_card(Some(cur_face), Some(cur_suit));

                        let x = inner_rect.x + suit_index as u16 * 4 + 2;
                        let y = inner_rect.y + face_index as u16;

                        frame.render_widget(
                            card_ui,
                            Rect {
                                x: x,
                                y: y,
                                width: 3,
                                height: 3,
                            },
                        );
                    }
                }
            }
        }

        let hint_title = Span::from(format!("{:<8}", "hints:"))
            .style(default_style().not_bold().fg(Color::Gray));
        let hint_span = Span::from("\u{f444} ".repeat(game_state.remaining_hint_count as usize))
            .style(default_style().fg(Color::White));

        let hints_remaining = [hint_title, hint_span];

        let bomb_title = Span::from(format!("{:<8}", "bombs:"))
            .style(default_style().not_bold().fg(Color::Gray));
        let bomb_span = Span::from("\u{f0691} ".repeat(game_state.remaining_bomb_count as usize))
            .style(default_style().fg(Color::White));
        let bombs_remaining: [Span<'_>; 2] = [bomb_title, bomb_span];

        let discards: Vec<_> = all_suits
            .iter()
            .map(|&cur_suit| {
                let mut card_faces: Vec<_> = game_state
                    .discard_pile
                    .iter()
                    .filter_map(|c| match c {
                        &Card { suit, face } if suit == cur_suit => Some(face),
                        _ => None,
                    })
                    .collect();
                card_faces.sort();

                Line::from(vec![
                    Span::from(cur_suit.key())
                        .style(default_style().fg(colorize_suit(cur_suit)).bold()),
                    " ".into(),
                    Span::from(card_faces.into_iter().map(|f| f.key()).join(" "))
                        .style(default_style().fg(colorize_suit(cur_suit)).dim()),
                ])
            })
            .collect_vec();

        let hints: Paragraph<'_> =
            Paragraph::new(Line::from_iter(hints_remaining)).style(default_style().bold());
        let bombs = Paragraph::new(Line::from_iter(bombs_remaining)).style(default_style().bold());
        let discards = Paragraph::new(Text::from(discards));

        frame.render_widget(
            hints,
            Rect {
                x: area.x + area.width / 2,
                y: area.y + 1,
                width: area.width / 2,
                height: 1,
            },
        );

        frame.render_widget(
            bombs,
            Rect {
                x: area.x + area.width / 2,
                y: area.y + 2,
                width: area.width / 2,
                height: 1,
            },
        );

        let discard_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(default_style().gray())
            .title("discards");
        frame.render_widget(
            discards.block(discard_block),
            Rect {
                x: area.x + area.width / 2 - 1,
                y: area.y + 4,
                width: area.width / 2,
                height: all_suits.len() as u16 + 2,
            },
        );

        frame.render_widget(game_block, area);
    }

    fn render_game_log(&self, log: &Vec<String>, frame: &mut Frame, area: Rect) {
        let game_log_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Game Log");
        let log_color = Color::Gray;

        let lines: Vec<Span> = log
            .iter()
            .map(|line| Span::from(format!("{}", line)).style(default_style().fg(log_color)))
            .collect_vec();

        let text = Text::from_iter(lines);
        let log = Paragraph::new(text).block(game_log_block);

        frame.render_widget(log, area);

        // render_title(
        //     frame,
        //     Rect {
        //         x: area.x - 3,
        //         y: area.y + 1,
        //         width: area.width,
        //         height: area.height - 5,
        //     },
        // )
        // .expect("big text error");
    }

    pub fn render_game_actions(&self, frame: &mut Frame, area: Rect) {
        let actions = self.legend_for_command_state(&self.game_state);

        let legend_string: Vec<_> =
            Itertools::intersperse(actions.iter().map(Some), None).collect();
        let lines: Vec<_> = legend_string
            .into_iter()
            .map(|legend| match legend {
                Some(LegendItem {
                    desc,
                    key_code: KeyCode::Char(key),
                    ..
                }) => Span::from(format!("{} [{}]", desc, key))
                    .style(default_style().bg(SELECTION_COLOR).fg(Color::White)),

                Some(LegendItem {
                    desc,
                    key_code: KeyCode::Backspace,
                    ..
                }) => Span::from(format!("{} [{}]", desc, "\u{f030d}"))
                    .style(default_style().bg(SELECTION_COLOR).fg(Color::White)),

                Some(LegendItem {
                    desc,
                    key_code: KeyCode::Esc,
                    ..
                }) => Span::from(format!("{} [{}]", desc, "\u{f12b7} "))
                    .style(default_style().bg(Color::LightMagenta).fg(Color::White)),

                Some(_) => panic!("Unknown keycode"),

                None => Span::raw(" "),
            })
            .collect();

        frame.render_widget(Line::from_iter(lines.into_iter()), area);
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
                HanabiGame::Lobby { players, .. } => {
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
                } => self.legend_for_command_state_game(game_state, players),

                HanabiGame::Ended {
                    players,
                    game_state,
                    revealed_game_state,
                } => {
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
        if let Some(outcome) = &game_state.outcome {
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

            CommandBuilder::Hint(HintState::ChoosingFace { .. }) => vec![
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
                    desc: "Back".to_string(),
                    key_code: Backspace,
                    action: AppAction::GameAction(GameAction::Undo),
                },
            ],
            CommandBuilder::Hint(HintState::ChoosingSuit { .. }) => vec![
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
            CommandBuilder::Hint(HintState::ChoosingHintType { .. }) => vec![
                LegendItem {
                    desc: "Suit".to_string(),
                    key_code: Char('s'),
                    action: AppAction::GameAction(GameAction::SelectHintType {
                        hint_type: HintBuilderType::Suite,
                    }),
                },
                LegendItem {
                    desc: "Face".to_string(),
                    key_code: Char('f'),
                    action: AppAction::GameAction(GameAction::SelectHintType {
                        hint_type: HintBuilderType::Face,
                    }),
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

                // vec![
                //     LegendItem {
                //         desc: format!("{} #1", action),
                //         key_code: Char('1'),
                //         action: AppAction::SelectCard(SlotIndex(0)),
                //     },
                //     LegendItem {
                //         desc: format!("{} #2", action),
                //         key_code: Char('2'),
                //         action: AppAction::SelectCard(SlotIndex(1)),
                //     },
                //     LegendItem {
                //         desc: format!("{} #3", action),
                //         key_code: Char('3'),
                //         action: AppAction::SelectCard(SlotIndex(2)),
                //     },
                //     LegendItem {
                //         desc: format!("{} #4", action),
                //         key_code: Char('4'),
                //         action: AppAction::SelectCard(SlotIndex(3)),
                //     },
                // ]
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

fn colorize_suit(suit: CardSuit) -> Color {
    match suit {
        CardSuit::Red => Color::Rgb(235, 90, 78),
        CardSuit::Green => Color::Rgb(113, 244, 120),
        CardSuit::Yellow => Color::Rgb(238, 249, 137),
        CardSuit::White => Color::Rgb(255, 255, 255),
        CardSuit::Blue => Color::Rgb(90, 90, 245),
    }
}

fn render_title(frame: &mut Frame, area: Rect) -> Result<(), String> {
    // let big_text = BigText::builder()
    //     .pixel_size(PixelSize::Quadrant)
    //     .alignment(Alignment::Right)
    //     .style(default_style().fg(Color::Rgb(60, 60, 60)))
    //     .lines(vec![
    //         "hanabi".into(),
    //         // "h".into(),
    //         // "a".into(),
    //         // "n".into(),
    //         // "a".into(),
    //         // "b".into(),
    //         // "i".into(),
    //     ])
    //     .build()
    //     .map_err(|e| e.to_string())?;
    // frame.render_widget(big_text, area);
    Ok(())
}

fn render_placeholder(suit: Option<CardSuit>) -> Block<'static> {
    let color = suit.map(colorize_suit).unwrap_or(Color::DarkGray);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(default_style().fg(color))
        .style(default_style());
    // .bg(colorize_suit(card.suit));
    block
}

fn render_card(face: Option<CardFace>, suit: Option<CardSuit>) -> Paragraph<'static> {
    let color = suit.map(colorize_suit).unwrap_or(Color::Gray);
    let p = Paragraph::new(
        face.map(|f| f.key().set_style(default_style().fg(color).bold()))
            .unwrap_or("?".set_style(default_style().fg(Color::Gray))),
    );
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(default_style().fg(color).add_modifier(Modifier::BOLD))
        .style(default_style());
    // .bg(colorize_suit(card.suit));

    p.block(block)
}

fn render_card_span(face: Option<CardFace>, suit: Option<CardSuit>) -> Span<'static> {
    let color = suit.map(colorize_suit).unwrap_or(Color::Gray);
    Span::styled(
        face.map(|f| f.key()).unwrap_or("?").to_string(),
        default_style().fg(color).bold(),
    )
}

enum PlayerRenderState {
    CurrentTurn,
    CurrentSelection,
    Default,
}

fn render_player(
    player: &ClientPlayerView,
    name: &str,
    render_state: PlayerRenderState,
    frame: &mut Frame,
    area: Rect,
) {
    let num_cards = match player {
        ClientPlayerView::Me { hand } => hand.len(),
        ClientPlayerView::Teammate { hand } => hand.len(),
    };
    let player_block = Block::new()
        .style(default_style())
        .borders(Borders::ALL)
        .border_type(match render_state {
            PlayerRenderState::CurrentTurn => BorderType::Double,
            PlayerRenderState::CurrentSelection => BorderType::Double,
            _ => BorderType::Rounded,
        })
        .border_style(match render_state {
            PlayerRenderState::CurrentTurn => default_style().fg(TURN_COLOR),
            PlayerRenderState::CurrentSelection => default_style()
                .add_modifier(Modifier::BOLD)
                .fg(SELECTION_COLOR)
                .slow_blink(),
            _ => default_style().fg(Color::White),
        })
        .title(format!("{}", name).set_style(match render_state {
            PlayerRenderState::CurrentTurn => default_style(),
            PlayerRenderState::CurrentSelection => {
                default_style().fg(Color::Black).bg(SELECTION_COLOR).dim()
            }
            PlayerRenderState::Default => default_style(),
        }));
    let player_rect = player_block.inner(area);

    let not_hints_block = Block::new()
        .borders(Borders::TOP)
        .border_type(BorderType::Plain)
        .border_style(default_style().not_bold().gray())
        .title("not".gray())
        .title_alignment(Alignment::Center);

    frame.render_widget(player_block, area);
    frame.render_widget(
        not_hints_block,
        Rect {
            x: player_rect.x,
            y: player_rect.y + 5,
            width: player_rect.width,
            height: player_rect.height - 5,
        },
    );

    for index in 0..num_cards {
        let has_card = match &player {
            ClientPlayerView::Me { hand } => hand[index].is_some(),
            ClientPlayerView::Teammate { hand } => hand[index].is_some(),
        };
        if !has_card {
            continue;
        }

        let hints = match &player {
            ClientPlayerView::Me { hand } => hand[index].as_ref().map(|h| h.hints.as_slice()),
            ClientPlayerView::Teammate { hand } => hand[index].as_ref().map(|h| h.hints.as_slice()),
        };

        let (suit, face) = match player {
            ClientPlayerView::Me { hand } => hand[index].as_ref().map(|c| {
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
            ClientPlayerView::Teammate { hand } => hand[index]
                .as_ref()
                .map(|s| (Some(s.card.suit), Some(s.card.face))),
        }
        .unwrap_or((None, None));

        let card_ui = render_card(face, suit);

        let x = player_rect.x + index as u16 * 3;
        let y = player_rect.y;

        frame.render_widget(
            card_ui,
            // Paragraph::new("card 1".dark_gray())
            //     .wrap(Wrap { trim: true })
            //     .block(player),
            Rect {
                x: x,
                y: y,
                width: 3,
                height: 3,
            }, //layout[0][0],
        );

        let y = y + 3;

        let lines: Vec<_> = match hints {
            Some(hints) => hints
                .iter()
                .filter_map(|hint| {
                    Some(Line::from(match hint {
                        Hint::IsSuit(suit) => Span::styled(
                            suit.key().to_string(),
                            default_style().fg(colorize_suit(*suit)).bold(),
                        ),
                        Hint::IsFace(face) => Span::styled(
                            face.key().to_string(),
                            default_style().fg(Color::Gray).bold(),
                        ),
                        _ => return None,
                    }))
                })
                .collect(),
            None => vec![],
        };

        let hint_lines = lines.len();

        let not_lines: Vec<_> = match hints {
            Some(hints) => hints
                .iter()
                .filter_map(|hint| {
                    Some(Line::from(match hint {
                        Hint::IsNotSuit(suit) => Span::styled(
                            suit.key().to_string(),
                            default_style().fg(colorize_suit(*suit)).dim(),
                        ),
                        Hint::IsNotFace(face) => Span::styled(
                            face.key().to_string(),
                            default_style().fg(Color::DarkGray),
                        ),
                        _ => return None,
                    }))
                })
                .collect(),
            None => vec![],
        };
        let not_line_count = not_lines.len();

        let text = Text::from(lines);
        let p = Paragraph::new(text);
        frame.render_widget(
            p,
            // Paragraph::new("card 1".dark_gray())
            //     .wrap(Wrap { trim: true })
            //     .block(player),
            Rect {
                x: x + 1,
                y: y,
                width: 1,
                height: hint_lines as u16,
            }, //layout[0][0],
        );

        let text = Text::from(not_lines);
        let p = Paragraph::new(text);
        frame.render_widget(
            p,
            Rect {
                x: x + 1,
                y: y + 3,
                width: 1,
                height: not_line_count as u16,
            },
        )
    }

    // frame.render_widget(paragraph.clone().block(inner_block), inner);
}
