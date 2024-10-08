use crossterm::{
    event,
    event::{Event, KeyCode},
};
use ratatui::{style::Stylize, Frame, Terminal};
use shared::model::*;
use std::{char::from_digit, iter, ops::ControlFlow, time::Duration};
use tui_big_text::{BigText, PixelSize};

use shared::model::{ClientPlayerView, GameStateSnapshot};

use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use shared::client_logic::*;

use crate::BoxedResult;

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

static SELECTION_COLOR: Color = Color::Rgb(117, 158, 179);

// #[derive(Debug, Default)]
// struct StatefulList {
//     state: ListState,
//     items: Vec<String>,
//     last_selected: Option<usize>,
// }

// impl StatefulList {
//     fn with_items(items: Vec<String>) -> StatefulList {
//         StatefulList {
//             state: ListState::default(),
//             items: items,
//             last_selected: None,
//         }
//     }

//     fn next(&mut self) {
//         let i = match self.state.selected() {
//             Some(i) => {
//                 if i >= self.items.len() - 1 {
//                     0
//                 } else {
//                     i + 1
//                 }
//             }
//             None => self.last_selected.unwrap_or(0),
//         };
//         self.state.select(Some(i));
//     }

//     fn previous(&mut self) {
//         let i = match self.state.selected() {
//             Some(i) => {
//                 if i == 0 {
//                     self.items.len() - 1
//                 } else {
//                     i - 1
//                 }
//             }
//             None => self.last_selected.unwrap_or(0),
//         };
//         self.state.select(Some(i));
//     }

//     fn unselect(&mut self) {
//         let offset = self.state.offset();
//         self.last_selected = self.state.selected();
//         self.state.select(None);
//         *self.state.offset_mut() = offset;
//     }
// }

pub struct HanabiApp {
    exit: bool,
    command: CommandState,
    // menu_options: StatefulList,
}

pub enum EventHandlerResult {
    PlayerAction(PlayerAction),
    Quit,
    Continue,
}

impl HanabiApp {
    pub fn new(game_state: GameStateSnapshot) -> Self {
        HanabiApp {
            exit: false,
            command: CommandState {
                current_command: CommandBuilder::Empty,
            },

            game_state: game_state,
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

    /// runs the application's main loop until the user quits
    pub fn draw<T>(&mut self, terminal: &mut Terminal<T>) -> BoxedResult<()>
    where
        T: ratatui::backend::Backend,
    {
        terminal.draw(|frame| self.ui(frame))?;

        Ok(())
    }

    pub fn handle_events(&mut self) -> BoxedResult<EventHandlerResult> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                use KeyCode::*;
                match key.code {
                    Char('q') | Esc => {
                        self.exit = true;
                    }
                    _ => {}
                }

                if self.exit {
                    return Ok(EventHandlerResult::Quit);
                }

                return Ok(EventHandlerResult::Continue);
            }
        }
        Ok(EventHandlerResult::Continue)
    }

    fn ui(&mut self, frame: &mut Frame) {
        // let player = Block::new()
        //     .borders(Borders::ALL)
        //     .border_type(BorderType::Rounded)
        //     .border_style(Style::new().white().add_modifier(Modifier::DIM))
        //     .title(format!("Mirza"));

        for (index, client) in self.game_state.players.iter().enumerate() {
            render_player(
                client,
                index,
                match (self.game_state.turn, &self.command.current_command) {
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
            .style(Style::default().not_bold().fg(Color::Gray));
        let hint_span =
            Span::from("\u{f444} ".repeat(self.game_state.remaining_hint_count as usize))
                .style(Style::default().fg(Color::White));

        let hints_remaining = [hint_title, hint_span];

        let bomb_title = Span::from(format!("{:<8}", "bombs:"))
            .style(Style::default().not_bold().fg(Color::Gray));
        let bomb_span =
            Span::from("\u{f0691} ".repeat(self.game_state.remaining_bomb_count as usize))
                .style(Style::default().fg(Color::White));
        let bombs_remaining: [Span<'_>; 2] = [bomb_title, bomb_span];

        let discards: Vec<_> = all_suits
            .iter()
            .map(|&cur_suit| {
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
            .border_style(Style::default().gray())
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

    fn render_game_log(&mut self, frame: &mut Frame, area: Rect) {
        let game_log_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Game Log");
        let log_color = Color::Gray;

        let initial: Vec<Span> = vec![
            Span::from("Game Start").style(Style::default().fg(log_color)),
            Span::from("Player #0's turn").style(Style::default().fg(log_color)),
        ];

        // let lines: Vec<Span> = self
        //     .game_log
        //     .log
        //     .iter()
        //     .map(|(action, state)| {
        //         let text: String = match action {
        //             PlayerAction::GiveHint(PlayerIndex(hinted_player), hint) => {
        //                 let hint_text = match hint {
        //                     HintAction::SameSuit(suit) => {
        //                         format!("Hint to {}'s {} cards.", hinted_player, suit.key())
        //                     }
        //                     HintAction::SameFace(face) => {
        //                         format!("Hint to {}'s {} cards.", hinted_player, face.key())
        //                     }
        //                 };
        //                 hint_text
        //             }
        //             PlayerAction::PlayCard(SlotIndex(slot)) => {
        //                 format!("Played card #{}", slot)
        //             }
        //             PlayerAction::DiscardCard(SlotIndex(slot)) => {
        //                 format!("Discarded card #{}", slot)
        //             }
        //         };

        //         vec![
        //             Span::from(format!("{}", text)).style(Style::default().fg(log_color)),
        //             Span::from(format!("Player #{}'s turn", state.current_player_index().0))
        //                 .style(Style::default().fg(log_color)),
        //         ]
        //     })
        //     .flatten()
        //     .collect_vec();
        let lines = vec![];

        let outcome_lines = match &self.game_state.outcome {
            Some(outcome) => vec![Span::from(format!("Game Over: {:?}", outcome))
                .style(Style::default().fg(log_color))],
            None => vec![],
        };

        let lines = initial
            .into_iter()
            .chain(lines.into_iter())
            .chain(outcome_lines.into_iter())
            .collect_vec();

        let text = Text::from_iter(lines);
        let log = Paragraph::new(text).block(game_log_block);

        frame.render_widget(log, area);

        render_title(
            frame,
            Rect {
                x: area.x - 3,
                y: area.y + 1,
                width: area.width,
                height: area.height - 5,
            },
        )
        .expect("big text error");
    }

    pub fn render_game_actions(&mut self, frame: &mut Frame, area: Rect) {
        let actions = self.legend_for_command_state();

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
                    .style(Style::default().bg(SELECTION_COLOR).fg(Color::White)),

                Some(LegendItem {
                    desc,
                    key_code: KeyCode::Backspace,
                    ..
                }) => Span::from(format!("{} [{}]", desc, "\u{f030d}"))
                    .style(Style::default().bg(SELECTION_COLOR).fg(Color::White)),

                Some(LegendItem {
                    desc,
                    key_code: KeyCode::Esc,
                    ..
                }) => Span::from(format!("{} [{}]", desc, "\u{f12b7} "))
                    .style(Style::default().bg(Color::LightMagenta).fg(Color::White)),

                Some(_) => panic!("Unknown keycode"),

                None => Span::raw(" "),
            })
            .collect();

        frame.render_widget(Line::from_iter(lines.into_iter()), area);
    }

    fn legend_for_command_state(&self) -> Vec<LegendItem> {
        if let Some(outcome) = &self.game_state.outcome {
            return vec![LegendItem {
                desc: format!("Quit"),
                key_code: KeyCode::Esc,
                action: AppAction::Quit,
            }];
        }

        use KeyCode::*;
        match self.command.current_command {
            CommandBuilder::Empty => [
                Some(LegendItem {
                    desc: "Play Card".to_string(),
                    key_code: Char('p'),
                    action: AppAction::StartPlay,
                }),
                Some(LegendItem {
                    desc: "Discard Card".to_string(),
                    key_code: Char('d'),
                    action: AppAction::StartDiscard,
                }),
                match self.game_state.remaining_hint_count {
                    0 => None,
                    _ => Some(LegendItem {
                        desc: "Give Hint".to_string(),
                        key_code: Char('h'),
                        action: AppAction::StartHint,
                    }),
                },
                Some(LegendItem {
                    desc: "Undo".to_string(),
                    key_code: Char('u'),
                    action: AppAction::Undo,
                }),
            ]
            .into_iter()
            .flatten()
            .collect(),
            CommandBuilder::Hint(HintState::ChoosingPlayer) => (0..self.game_state.players.len())
                .filter(|&index| self.game_state.turn.0 != index)
                .map(|index| LegendItem {
                    desc: format!("Player #{}", index + 1),
                    key_code: Char(from_digit(index as u32 + 1, 10).unwrap()),
                    action: AppAction::SelectPlayer {
                        player_index: index as u8,
                    },
                })
                .chain(iter::once(LegendItem {
                    desc: "Back".to_string(),
                    key_code: Backspace,
                    action: AppAction::Undo,
                }))
                .collect_vec(),

            CommandBuilder::Hint(HintState::ChoosingFace { .. }) => vec![
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
            CommandBuilder::Hint(HintState::ChoosingSuit { .. }) => vec![
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
            CommandBuilder::Hint(HintState::ChoosingHintType { .. }) => vec![
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
            CommandBuilder::Play(CardState::ChoosingCard { card_type })
            | CommandBuilder::Discard(CardState::ChoosingCard { card_type }) => {
                let action = match card_type {
                    CardBuilderType::Play => "Play",
                    CardBuilderType::Discard => "Discard",
                };
                match self.game_state.players.get(self.game_state.turn.0) {
                    Some(ClientPlayerView::Me { hand }) => hand
                        .iter()
                        .enumerate()
                        .filter(|(_, slot)| slot.is_some())
                        .map(|(index, _)| LegendItem {
                            desc: format!("{} #{}", action, index + 1),
                            key_code: Char(from_digit(index as u32 + 1, 10).unwrap()),
                            action: AppAction::SelectCard(SlotIndex(index)),
                        })
                        .chain(iter::once(LegendItem {
                            desc: "Back".to_string(),
                            key_code: Backspace,
                            action: AppAction::Undo,
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

fn render_title(frame: &mut Frame, area: Rect) -> Result<(), String> {
    let big_text = BigText::builder()
        .pixel_size(PixelSize::Quadrant)
        .alignment(Alignment::Right)
        .style(Style::new().fg(Color::Rgb(60, 60, 60)))
        .lines(vec![
            "hanabi".into(),
            // "h".into(),
            // "a".into(),
            // "n".into(),
            // "a".into(),
            // "b".into(),
            // "i".into(),
        ])
        .build()
        .map_err(|e| e.to_string())?;
    frame.render_widget(big_text, area);
    Ok(())
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
    let p = Paragraph::new(
        face.map(|f| f.key().set_style(Style::new().fg(color).bold()))
            .unwrap_or("?".set_style(Style::new().fg(Color::Gray))),
    );
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color).add_modifier(Modifier::BOLD));
    // .bg(colorize_suit(card.suit));

    p.block(block)
}

fn render_card_span(face: Option<CardFace>, suit: Option<CardSuit>) -> Span<'static> {
    let color = suit.map(colorize_suit).unwrap_or(Color::Gray);
    Span::styled(
        face.map(|f| f.key()).unwrap_or("?").to_string(),
        Style::new().fg(color).bold(),
    )
}

enum PlayerRenderState {
    CurrentTurn,
    CurrentSelection,
    Default,
}

fn render_player(
    player: &ClientPlayerView,
    index: usize,
    render_state: PlayerRenderState,
    frame: &mut Frame,
    area: Rect,
) {
    let num_cards = match player {
        ClientPlayerView::Me { hand } => hand.len(),
        ClientPlayerView::Teammate { hand } => hand.len(),
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
                .fg(SELECTION_COLOR)
                .rapid_blink(),
            _ => Style::default().fg(Color::White),
        })
        .title(
            format!("Player {}", index + 1).set_style(match render_state {
                PlayerRenderState::CurrentTurn => Style::default(),
                PlayerRenderState::CurrentSelection => {
                    Style::default().fg(Color::Black).bg(SELECTION_COLOR).dim()
                }
                PlayerRenderState::Default => Style::default(),
            }),
        );
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
                            Style::new().fg(colorize_suit(*suit)).bold(),
                        ),
                        Hint::IsFace(face) => Span::styled(
                            face.key().to_string(),
                            Style::new().fg(Color::Gray).bold(),
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
                            Style::new().fg(colorize_suit(*suit)).dim(),
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
