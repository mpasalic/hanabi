use crate::client_logic;
use crate::client_logic::CommandState;
use crate::model::Card;
use crate::model::CardFace;
use crate::model::CardSuit;
use crate::model::GameState;
use crate::model::Hint;
use crate::model::HintAction;
use crate::model::Player;
use crate::model::PlayerAction;
use crate::model::PlayerIndex;
use crate::model::Slot;
use automerge::hydrate::List;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::style::Stylize;
use ratatui::widgets::ListState;
use ratatui::Frame;
use ratatui::Terminal;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::io;
use std::ops::ControlFlow;
use std::time::Duration;
use strum::IntoEnumIterator;

use crate::model::ClientGameState;
use crate::model::ClientHiddenCard;
use crate::model::ClientPlayerView;
use crate::model::GameOutcome;
use crate::model::SlotIndex;
use crate::BoxedResult;
use std::{
    error::Error,
    io::{stdout, Stdout},
};

use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Padding, Paragraph, Wrap,
    },
};

use crate::client_logic::*;

trait CardKey {
    fn key(&self) -> &'static str;
}

trait ColoredCard {
    fn color(&self) -> colored::ColoredString;
    fn color_string(&self, string: String) -> colored::ColoredString;
    fn inactive_color(&self) -> colored::ColoredString;
}

impl CardKey for CardSuit {
    fn key(&self) -> &'static str {
        // match self {
        //     CardSuit::Red => "\u{f444}",
        //     CardSuit::Green => "\u{f444}",
        //     CardSuit::Yellow => "\u{f444}",
        //     CardSuit::White => "\u{f444}",
        //     CardSuit::Blue => "\u{f444}",
        // }
        match self {
            CardSuit::Red => "R",
            CardSuit::Green => "G",
            CardSuit::Yellow => "Y",
            CardSuit::White => "W",
            CardSuit::Blue => "B",
        }
        // match self {
        //     CardSuit::Red => "\u{e2a6}",
        //     CardSuit::Green => "\u{f1bb}",
        //     CardSuit::Yellow => "\u{f0238}",
        //     CardSuit::White => "\u{e315}",
        //     CardSuit::Blue => "\u{f043}",
        // }
    }
}

// impl ColoredCard for CardSuit {
//     fn color_string(&self, string: String) -> Span<'_> {
//         match self {
//             CardSuit::Red => string.red(),
//             CardSuit::Green => string.green(),
//             CardSuit::Yellow => string.yellow(),
//             CardSuit::White => string.white(),
//             CardSuit::Blue => string.blue(),
//         }
//     }

//     fn color(&self) -> colored::ColoredString {
//         self.color_string(self.key().to_string()).bold()
//     }

//     fn inactive_color(&self) -> colored::ColoredString {
//         self.key().to_string().dimmed()
//     }
// }

impl CardKey for CardFace {
    fn key(&self) -> &'static str {
        match self {
            CardFace::One => "1",
            CardFace::Two => "2",
            // CardFace::Three => "\u{f03ac}",
            CardFace::Three => "3",
            CardFace::Four => "4",
            CardFace::Five => "5",
        }
    }
}

#[derive(Debug, Default)]
struct StatefulList {
    state: ListState,
    items: Vec<String>,
    last_selected: Option<usize>,
}

impl StatefulList {
    fn with_items(items: Vec<String>) -> StatefulList {
        StatefulList {
            state: ListState::default(),
            items: items,
            last_selected: None,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        let offset = self.state.offset();
        self.last_selected = self.state.selected();
        self.state.select(None);
        *self.state.offset_mut() = offset;
    }
}

#[derive(Debug)]
pub struct HanabiApp {
    counter: u8,
    exit: bool,
    command: CommandState,
    menu_options: StatefulList,
    game_state: ClientGameState,
}

impl HanabiApp {
    pub fn new() -> Self {
        let client_game_state = ClientGameState {
            draw_pile_count: 25,
            played_cards: vec![],
            // played_cards: vec![
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::Two,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Green,
            //     },
            //     Card {
            //         face: CardFace::Three,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::Four,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::Five,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Red,
            //     },
            //     Card {
            //         face: CardFace::Two,
            //         suit: CardSuit::Red,
            //     },
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Yellow,
            //     },
            // ],
            discard_pile: vec![
                Card {
                    face: CardFace::Two,
                    suit: CardSuit::Red,
                },
                Card {
                    face: CardFace::Two,
                    suit: CardSuit::Red,
                },
                Card {
                    face: CardFace::One,
                    suit: CardSuit::Red,
                },
                Card {
                    face: CardFace::Three,
                    suit: CardSuit::Blue,
                },
                Card {
                    face: CardFace::Five,
                    suit: CardSuit::Green,
                },
                Card {
                    face: CardFace::Five,
                    suit: CardSuit::Red,
                },
            ],
            players: vec![
                ClientPlayerView::Me {
                    hand: vec![
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsSuit(CardSuit::Blue)],
                        }),
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsSuit(CardSuit::Blue)],
                        }),
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsNotSuit(CardSuit::Blue)],
                        }),
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsNotSuit(CardSuit::Blue)],
                        }),
                    ],
                },
                ClientPlayerView::Teammate(Player {
                    hand: vec![
                        Some(Slot {
                            card: Card {
                                face: CardFace::Three,
                                suit: CardSuit::Green,
                            },
                            hints: vec![
                                Hint::IsFace(CardFace::Three),
                                Hint::IsSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Four,
                                suit: CardSuit::Yellow,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Five,
                                suit: CardSuit::White,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::One,
                                suit: CardSuit::Red,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                    ],
                }),
                ClientPlayerView::Teammate(Player {
                    hand: vec![
                        Some(Slot {
                            card: Card {
                                face: CardFace::Three,
                                suit: CardSuit::Green,
                            },
                            hints: vec![
                                Hint::IsFace(CardFace::Three),
                                Hint::IsSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Four,
                                suit: CardSuit::Yellow,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Five,
                                suit: CardSuit::White,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::One,
                                suit: CardSuit::Red,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                    ],
                }),
                ClientPlayerView::Teammate(Player {
                    hand: vec![
                        Some(Slot {
                            card: Card {
                                face: CardFace::Three,
                                suit: CardSuit::Green,
                            },
                            hints: vec![
                                Hint::IsFace(CardFace::Three),
                                Hint::IsSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Four,
                                suit: CardSuit::Yellow,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Five,
                                suit: CardSuit::White,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::One,
                                suit: CardSuit::Red,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                    ],
                }),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
        };

        HanabiApp {
            counter: 0,
            exit: false,
            command: CommandState {
                current_command: CommandBuilder::Empty,
            },
            menu_options: StatefulList::default(),
            game_state: client_game_state,
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run<T>(&mut self, terminal: &mut Terminal<T>) -> BoxedResult<()>
    where
        T: ratatui::backend::Backend,
    {
        while !self.exit {
            terminal.draw(|frame| self.ui(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> BoxedResult<ControlFlow<()>> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                use KeyCode::*;
                match key.code {
                    Char('q') | Esc => {
                        self.exit = true;
                    }
                    key_code => {
                        let options = legend_for_command_state(&self.command.current_command);
                        let chosen_option = options.into_iter().find(|a| a.key_code == key_code);
                        if let Some(LegendItem { action, .. }) = chosen_option {
                            self.command = process_app_action(self.command.clone(), action);
                        }
                    } // Char('h') | Left => self.menu_state.unselect(),
                      // Down => self.menu_options.next(),
                      // Up => self.menu_options.previous(),
                      // Char('l') | Right | Enter => self.change_status(),
                      // Char('g') => self.go_top(),
                      // Char('G') => self.go_bottom(),
                }

                if self.exit {
                    return Ok(ControlFlow::Break(()));
                }

                return Ok(ControlFlow::Continue(()));
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    fn ui(&mut self, frame: &mut Frame) {
        let client_game_state = ClientGameState {
            draw_pile_count: 25,
            played_cards: vec![],
            // played_cards: vec![
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::Two,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Green,
            //     },
            //     Card {
            //         face: CardFace::Three,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::Four,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::Five,
            //         suit: CardSuit::Blue,
            //     },
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Red,
            //     },
            //     Card {
            //         face: CardFace::Two,
            //         suit: CardSuit::Red,
            //     },
            //     Card {
            //         face: CardFace::One,
            //         suit: CardSuit::Yellow,
            //     },
            // ],
            discard_pile: vec![
                Card {
                    face: CardFace::Two,
                    suit: CardSuit::Red,
                },
                Card {
                    face: CardFace::Two,
                    suit: CardSuit::Red,
                },
                Card {
                    face: CardFace::One,
                    suit: CardSuit::Red,
                },
                Card {
                    face: CardFace::Three,
                    suit: CardSuit::Blue,
                },
                Card {
                    face: CardFace::Five,
                    suit: CardSuit::Green,
                },
                Card {
                    face: CardFace::Five,
                    suit: CardSuit::Red,
                },
            ],
            players: vec![
                ClientPlayerView::Me {
                    hand: vec![
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsSuit(CardSuit::Blue)],
                        }),
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsSuit(CardSuit::Blue)],
                        }),
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsNotSuit(CardSuit::Blue)],
                        }),
                        Some(ClientHiddenCard {
                            hints: vec![Hint::IsNotSuit(CardSuit::Blue)],
                        }),
                    ],
                },
                ClientPlayerView::Teammate(Player {
                    hand: vec![
                        Some(Slot {
                            card: Card {
                                face: CardFace::Three,
                                suit: CardSuit::Green,
                            },
                            hints: vec![
                                Hint::IsFace(CardFace::Three),
                                Hint::IsSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Four,
                                suit: CardSuit::Yellow,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Five,
                                suit: CardSuit::White,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::One,
                                suit: CardSuit::Red,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                    ],
                }),
                ClientPlayerView::Teammate(Player {
                    hand: vec![
                        Some(Slot {
                            card: Card {
                                face: CardFace::Three,
                                suit: CardSuit::Green,
                            },
                            hints: vec![
                                Hint::IsFace(CardFace::Three),
                                Hint::IsSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Four,
                                suit: CardSuit::Yellow,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Five,
                                suit: CardSuit::White,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::One,
                                suit: CardSuit::Red,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                    ],
                }),
                ClientPlayerView::Teammate(Player {
                    hand: vec![
                        Some(Slot {
                            card: Card {
                                face: CardFace::Three,
                                suit: CardSuit::Green,
                            },
                            hints: vec![
                                Hint::IsFace(CardFace::Three),
                                Hint::IsSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Four,
                                suit: CardSuit::Yellow,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::Five,
                                suit: CardSuit::White,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                        Some(Slot {
                            card: Card {
                                face: CardFace::One,
                                suit: CardSuit::Red,
                            },
                            hints: vec![
                                Hint::IsNotFace(CardFace::Three),
                                Hint::IsNotSuit(CardSuit::Green),
                            ],
                        }),
                    ],
                }),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
        };

        // let player = Block::new()
        //     .borders(Borders::ALL)
        //     .border_type(BorderType::Rounded)
        //     .border_style(Style::new().white().add_modifier(Modifier::DIM))
        //     .title(format!("Mirza"));

        for (index, client) in client_game_state.players.iter().enumerate() {
            render_player(
                client,
                match (client_game_state.turn, &self.command.current_command) {
                    (turn, _) if turn as usize == index => PlayerRenderState::CurrentTurn,
                    (_, &CommandBuilder::Hint(HintState::ChoosingHintType { player_index }))
                        if player_index as usize == index =>
                    {
                        PlayerRenderState::CurrentSelection
                    }
                    _ => PlayerRenderState::Default,
                },
                frame,
                Rect {
                    x: 2 + 14 * index as u16,
                    y: 2,
                    width: 4 * 3 + 2,
                    height: 16,
                },
            );
        }

        self.render_board(
            frame,
            Rect {
                x: 2,
                y: 18,
                width: 14 * 4,
                height: 14,
            },
        );

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

        self.render_game_log(
            &client_game_state,
            frame,
            Rect {
                x: 14 * 4 + 2,
                y: 2,
                width: frame.size().width - 14 * 4,
                height: 30,
            },
        );

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

    fn render_board(&self, frame: &mut Frame, area: Rect) {
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
            let mut card_faces: Vec<_> = self
                .game_state
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
                    for (face_index, &cur_face) in card_faces.iter().enumerate() {
                        let card_ui = render_card(Some(cur_face), Some(cur_suit));

                        let x = inner_rect.x + suit_index as u16 * 4;
                        let y = inner_rect.y + face_index as u16;

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
                    }
                }
            }
        }

        let hint_title = Span::from(format!("{:<8}", "hints:"))
            .style(Style::default().not_bold().fg(Color::Gray).dim());
        let hint_span =
            Span::from("\u{f444} ".repeat(self.game_state.remaining_hint_count as usize))
                .style(Style::default().fg(Color::White));

        let hints_remaining = [hint_title, hint_span];

        let bomb_title = Span::from(format!("{:<8}", "bombs:"))
            .style(Style::default().not_bold().fg(Color::Gray).dim());
        let bomb_span =
            Span::from("\u{f0691} ".repeat(self.game_state.remaining_bomb_count as usize))
                .style(Style::default().fg(Color::White));
        let bombs_remaining: [Span<'_>; 2] = [bomb_title, bomb_span];

        let discards: Vec<_> = all_suits
            .iter()
            .enumerate()
            .map(|(suit_index, &cur_suit)| {
                let mut card_faces: Vec<_> = self
                    .game_state
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
                        .style(Style::default().fg(colorize_suit(cur_suit)).bold()),
                    " ".into(),
                    Span::from(card_faces.into_iter().map(|f| f.key()).join(" "))
                        .style(Style::default().fg(colorize_suit(cur_suit)).dim()),
                ])
            })
            .collect_vec();

        // let discards = game_state.discard_pile.map(|c| {

        // })

        let hints: Paragraph<'_> =
            Paragraph::new(Line::from_iter(hints_remaining)).style(Style::new().bold());
        let bombs = Paragraph::new(Line::from_iter(bombs_remaining)).style(Style::new().bold());
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
            .border_style(Style::default().gray().dim())
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

    fn render_game_log(&mut self, game: &ClientGameState, frame: &mut Frame, area: Rect) {
        let game_log_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Game Log");
        let inner_rect = game_log_block.inner(area);

        let log = Paragraph::new("").block(game_log_block);
        frame.render_widget(log, area);

        // use ratatui::{prelude::*, widgets::*};

        // let items: Vec<ListItem> = self
        //     .menu_options
        //     .items
        //     .iter()
        //     .enumerate()
        //     .map(|(i, item)| ListItem::new(format!("{}", item)))
        //     .collect();

        // let list = List::new(items)
        //     .block(Block::default().title("List").borders(Borders::ALL))
        //     .style(Style::default().fg(Color::White))
        //     .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        //     .highlight_symbol(">>")
        //     .repeat_highlight_symbol(true)
        //     .direction(ListDirection::BottomToTop);

        // frame.render_stateful_widget(list, area, &mut self.menu_options.state);
    }

    pub fn render_game_actions(&mut self, frame: &mut Frame, area: Rect) {
        use KeyCode::*;

        let actions = legend_for_command_state(&self.command.current_command);

        let legend_string: Vec<_> = actions.iter().map(Some).intersperse(None).collect();
        let lines: Vec<_> = legend_string
            .into_iter()
            .map(|legend| match legend {
                Some(LegendItem {
                    desc,
                    key_code: KeyCode::Char(key),
                    action: actions,
                }) => Span::from(format!("{} [{}]", desc, key)).style(
                    Style::default()
                        .bg(Color::Rgb(117, 158, 179))
                        .fg(Color::White),
                ),

                Some(LegendItem {
                    desc,
                    key_code: KeyCode::Backspace,
                    action: actions,
                }) => Span::from(format!("{} [{}]", desc, "\u{f030d}")).style(
                    Style::default()
                        .bg(Color::Rgb(117, 158, 179))
                        .fg(Color::White),
                ),

                Some(_) => panic!("Unknown keycode"),

                None => Span::raw(" "),
            })
            .collect();

        frame.render_widget(Line::from_iter(lines.into_iter()), area);
    }
}

#[derive(PartialEq, Eq, Hash)]
enum LegendMode {
    PickActionType,
    PickPlayer,
    PickCard,
    PickHint,
}

struct LegendItem {
    desc: String,
    key_code: KeyCode,
    action: AppAction,
}

fn colorize_suit(suit: CardSuit) -> Color {
    match suit {
        CardSuit::Red => Color::Red,
        CardSuit::Green => Color::Green,
        CardSuit::Yellow => Color::Yellow,
        CardSuit::White => Color::White,
        CardSuit::Blue => Color::Blue,
    }
}

fn render_placeholder(suit: Option<CardSuit>) -> Block<'static> {
    let color = suit.map(colorize_suit).unwrap_or(Color::DarkGray);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color).dim());
    // .bg(colorize_suit(card.suit));
    block
}

fn render_card(face: Option<CardFace>, suit: Option<CardSuit>) -> Paragraph<'static> {
    let color = suit.map(colorize_suit).unwrap_or(Color::Gray);
    let p = Paragraph::new(face.map(|f| f.key()).unwrap_or("?").to_string())
        .style(Style::new().fg(color).bold());
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color).add_modifier(Modifier::BOLD));
    // .bg(colorize_suit(card.suit));

    p.block(block)
}

enum PlayerRenderState {
    CurrentTurn,
    CurrentSelection,
    Default,
}

fn render_player(
    player: &ClientPlayerView,
    render_state: PlayerRenderState,
    frame: &mut Frame,
    area: Rect,
) {
    let num_cards = match player {
        ClientPlayerView::Me { hand } => hand.len(),
        ClientPlayerView::Teammate(Player { hand }) => hand.len(),
    };
    let player_block = Block::new()
        .borders(Borders::ALL)
        .border_type(match render_state {
            PlayerRenderState::CurrentTurn => BorderType::Double,
            PlayerRenderState::CurrentSelection => BorderType::Double,
            _ => BorderType::Rounded,
        })
        .border_style(match render_state {
            PlayerRenderState::CurrentTurn => Style::default().fg(Color::Magenta),
            PlayerRenderState::CurrentSelection => Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White)
                .rapid_blink(),
            _ => Style::default().fg(Color::White),
        })
        .title("Player");
    let player_rect = player_block.inner(area);

    let not_hints_block = Block::new()
        .borders(Borders::TOP)
        .border_type(BorderType::Plain)
        .border_style(Style::new().dim())
        .title("not")
        .title_alignment(Alignment::Center);

    frame.render_widget(
        not_hints_block,
        Rect {
            x: player_rect.x,
            y: player_rect.y + 5,
            width: player_rect.width,
            height: player_rect.height - 5,
        },
    );

    frame.render_widget(player_block, area);

    for index in 0..num_cards {
        let hints = match &player {
            ClientPlayerView::Me { hand } => hand[index].as_ref().map(|h| h.hints.as_slice()),
            ClientPlayerView::Teammate(Player { hand }) => {
                hand[index].as_ref().map(|h| h.hints.as_slice())
            }
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
            ClientPlayerView::Teammate(Player { hand }) => hand[index]
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
                .enumerate()
                .filter_map(|(index, hint)| {
                    Some(Line::from(match hint {
                        Hint::IsSuit(suit) => Span::styled(
                            suit.key().to_string(),
                            Style::new().fg(colorize_suit(*suit)),
                        ),
                        Hint::IsFace(face) => {
                            Span::styled(face.key().to_string(), Style::new().fg(Color::DarkGray))
                        }
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
                .enumerate()
                .filter_map(|(index, hint)| {
                    Some(Line::from(match hint {
                        Hint::IsNotSuit(suit) => Span::styled(
                            suit.key().to_string(),
                            Style::new().fg(colorize_suit(*suit)),
                        ),
                        Hint::IsNotFace(face) => {
                            Span::styled(face.key().to_string(), Style::new().fg(Color::DarkGray))
                        }
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
pub fn legend_for_command_state(command: &CommandBuilder) -> Vec<LegendItem> {
    use KeyCode::*;
    match command {
        CommandBuilder::Empty => {
            vec![
                // LegendItem {
                //     desc: "Play Card".to_string(),
                //     key_code: Char('p'),
                //     action: AppA,
                // },
                // LegendItem {
                //     desc: "Discard Card".to_string(),
                //     key_code: Char('d'),
                //     action: vec![LegendMode::PickCard],
                // },
                LegendItem {
                    desc: "Give Hint".to_string(),
                    key_code: Char('h'),
                    action: AppAction::StartHint,
                },
                LegendItem {
                    desc: "Undo".to_string(),
                    key_code: Char('u'),
                    action: AppAction::Undo,
                },
            ]
        }
        CommandBuilder::Hint(HintState::ChoosingPlayer) => vec![
            LegendItem {
                desc: "One".to_string(),
                key_code: Char('1'),
                action: AppAction::SelectPlayer { player_index: 0 },
            },
            LegendItem {
                desc: "Two".to_string(),
                key_code: Char('2'),
                action: AppAction::SelectPlayer { player_index: 1 },
            },
            LegendItem {
                desc: "Three".to_string(),
                key_code: Char('3'),
                action: AppAction::SelectPlayer { player_index: 2 },
            },
            LegendItem {
                desc: "Four".to_string(),
                key_code: Char('4'),
                action: AppAction::SelectPlayer { player_index: 3 },
            },
            LegendItem {
                desc: "Back".to_string(),
                key_code: Backspace,
                action: AppAction::Undo,
            },
        ],
        CommandBuilder::Hint(HintState::ChoosingFace { player_index }) => vec![
            LegendItem {
                desc: "One".to_string(),
                key_code: Char('1'),
                action: AppAction::SelectFace(CardFace::One),
            },
            LegendItem {
                desc: "Two".to_string(),
                key_code: Char('2'),
                action: AppAction::SelectFace(CardFace::Two),
            },
            LegendItem {
                desc: "Three".to_string(),
                key_code: Char('3'),
                action: AppAction::SelectFace(CardFace::Three),
            },
            LegendItem {
                desc: "Four".to_string(),
                key_code: Char('4'),
                action: AppAction::SelectFace(CardFace::Four),
            },
            LegendItem {
                desc: "Five".to_string(),
                key_code: Char('5'),
                action: AppAction::SelectFace(CardFace::Five),
            },
            LegendItem {
                desc: "Back".to_string(),
                key_code: Backspace,
                action: AppAction::Undo,
            },
        ],
        CommandBuilder::Hint(HintState::ChoosingSuit { player_index }) => vec![
            LegendItem {
                desc: "Blue".to_string(),
                key_code: Char('b'),
                action: AppAction::SelectSuit(CardSuit::Blue),
            },
            LegendItem {
                desc: "Green".to_string(),
                key_code: Char('g'),
                action: AppAction::SelectSuit(CardSuit::Green),
            },
            LegendItem {
                desc: "Red".to_string(),
                key_code: Char('r'),
                action: AppAction::SelectSuit(CardSuit::Red),
            },
            LegendItem {
                desc: "White".to_string(),
                key_code: Char('w'),
                action: AppAction::SelectSuit(CardSuit::White),
            },
            LegendItem {
                desc: "Yellow".to_string(),
                key_code: Char('y'),
                action: AppAction::SelectSuit(CardSuit::Yellow),
            },
            LegendItem {
                desc: "Back".to_string(),
                key_code: Backspace,
                action: AppAction::Undo,
            },
        ],
        CommandBuilder::Hint(HintState::ChoosingHintType { player_index }) => vec![
            LegendItem {
                desc: "Suit".to_string(),
                key_code: Char('s'),
                action: AppAction::SelectHintType {
                    hint_type: HintBuilderType::Suite,
                },
            },
            LegendItem {
                desc: "Face".to_string(),
                key_code: Char('f'),
                action: AppAction::SelectHintType {
                    hint_type: HintBuilderType::Face,
                },
            },
            LegendItem {
                desc: "Back".to_string(),
                key_code: Backspace,
                action: AppAction::Undo,
            },
        ],
    }
}
