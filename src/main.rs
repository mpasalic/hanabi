use model::Card;
use model::CardFace;
use model::CardSuit;
use model::GameState;
use model::Hint;
use model::HintAction;
use model::Player;
use model::PlayerAction;
use model::PlayerIndex;
use model::Slot;
use ratatui::style::Stylize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::io;
use strum::IntoEnumIterator;

use crate::model::ClientGameState;
use crate::model::ClientHiddenCard;
use crate::model::ClientPlayerView;
use crate::model::GameOutcome;
use crate::model::SlotIndex;
mod client_logic;
mod logic;
mod model;

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

// impl ColoredCard for CardFace {
//     fn color_string(&self, string: String) -> colored::ColoredString {
//         string.bold()
//     }

//     fn color(&self) -> colored::ColoredString {
//         self.color_string(self.key().to_string())
//     }

//     fn inactive_color(&self) -> colored::ColoredString {
//         self.key().to_string().dimmed()
//     }
// }

// impl fmt::Display for CardSuit {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.color())
//     }
// }

// impl fmt::Display for CardFace {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.key())
//     }
// }

// impl fmt::Display for Card {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.suit.color_string(self.face.key().to_string()))
//     }
// }

// fn fmt_card(card: Card, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//     write!(f, "{}", card)
// }

// impl fmt::Display for Player {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         for slot in self.hand.iter() {
//             if let Some(Slot { card, hints: _ }) = slot {
//                 fmt_card(*card, f)?;
//             }
//         }
//         return fmt::Result::Ok(());
//         //write!(f, "[{:?},{:?},{:?},{:?},{:?}]", self.hand[0], self.hand[1], self.hand[2], self.hand[3], self.hand[4])
//     }
// }

// impl Player {
//     /**
//      * Your hand [hints]: [? ![Three, Five, Two][Green]] [5] [?2 ![Green]] [3] [?3 ![Green]]
//      *
//      * --new version--
//      *
//      * Card 1: RGYWB 1 2 3 4 5
//      * Card 1: RYWB 1 4 3 4
//      * Card 2: G5
//      * Card 3:
//      * Card 4: G3
//      * Card 5:
//      *
//      */
//     fn hints_to_string(&self) {
//         let mut counter = 0;
//         let slots = self.hand.iter().filter_map(|slot| slot.as_ref());

//         for slot in slots {
//             let Slot { card: _, hints } = slot;
//             print!("Card {}: ", counter);
//             let mut face_hints_set: HashSet<CardFace> = HashSet::new();
//             let mut suit_hints_set: HashSet<CardSuit> = HashSet::new();

//             for face in CardFace::iter() {
//                 if !hints.iter().any(|hint| match hint {
//                     Hint::IsFace(hint_face) => *hint_face != face,
//                     Hint::IsNotFace(not_face) => *not_face == face,
//                     _ => false,
//                 }) {
//                     face_hints_set.insert(face.clone());
//                 }
//             }

//             for suit in CardSuit::iter() {
//                 if !hints.iter().any(|hint| match hint {
//                     Hint::IsSuit(suit_hint) => *suit_hint != suit,
//                     Hint::IsNotSuit(not_suit) => *not_suit == suit,
//                     _ => false,
//                 }) {
//                     suit_hints_set.insert(suit.clone());
//                 }
//             }

//             let face_hints_output: String = CardFace::iter()
//                 .into_iter()
//                 .map(|face| {
//                     if face_hints_set.contains(&face) {
//                         format!("{}", face.color())
//                     } else {
//                         format!("{}", face.inactive_color())
//                     }
//                 })
//                 .collect();

//             let suit_hints_output: String = CardSuit::iter()
//                 .into_iter()
//                 .map(|suit| {
//                     if suit_hints_set.contains(&suit) {
//                         format!("{}", suit.color())
//                     } else {
//                         format!("{}", suit.inactive_color())
//                     }
//                 })
//                 .collect();

//             println!("{}\t{}", suit_hints_output, face_hints_output);
//             counter = counter + 1;
//         }
//     }
// }

// impl fmt::Display for PlayerIndex {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             PlayerIndex(player_index) => write!(f, "P{}", player_index),
//         }
//     }
// }

// impl fmt::Display for GameState {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "State {{").expect("format");
//         write!(f, " bombs={}", self.remaining_bomb_count).expect("format");
//         for _ in 0..self.remaining_bomb_count {
//             write!(f, "X").expect("format");
//         }
//         write!(f, " hints={}", self.remaining_hint_count).expect("format");
//         for _ in 0..self.remaining_hint_count {
//             write!(f, "!").expect("format");
//         }

//         write!(f, " board=").expect("format");
//         for suit in CardSuit::iter() {
//             for face in CardFace::iter() {
//                 let card = Card { face, suit };
//                 if self.played_cards.contains(&card) {
//                     fmt_card(card, f).expect("format");
//                 } else {
//                     write!(f, "_").expect("format")
//                 }
//             }
//             write!(f, "|").expect("format");
//         }

//         write!(f, " discard=").expect("format");
//         for card in self.discard_pile.iter() {
//             fmt_card(*card, f).expect("format");
//         }

//         write!(f, " draw={}", self.draw_pile.len()).expect("format");

//         write!(f, " }}").expect("format");
//         return fmt::Result::Ok(());
//     }
// }

fn run_hanabi() -> Result<GameOutcome, String> {
    let num_players: usize = 5;

    let mut game = GameState::start(num_players)?;
    println!("> Starting Game!");
    // println!(
    //     "> P0:{{{}}} P1:{{{}}} P2:{{{}}} P3:{{{}}} P4:{{{}}}",
    //     players[0], players[1], players[2], players[3], players[4]
    // );

    //let mut last_round = None;

    let mut game_outcome: Option<GameOutcome> = None;

    while let None = game_outcome {
        // println!(
        //     "############ ROUND #{} PLAYER #{} ############",
        //     game.current_round(),
        //     game.current_player_index()
        // );

        loop {
            let action_effects = game.play(player_turn(&game)?);
            match action_effects {
                Ok(effects) => {
                    game.run_effects(effects)?;
                    break;
                }
                Err(msg) => println!("Disallowed action: {}", msg),
            }
        }
        game_outcome = game.check_game_outcome();
    }

    match game_outcome {
        Some(game_outcome) => Ok(game_outcome),
        None => Err("Error".to_string()),
    }
}

fn player_turn(game: &GameState) -> Result<PlayerAction, String> {
    // let current_index = game.current_player_index();
    // let current_player = game.current_player().ok_or_else(|| "No current player")?;
    // // println!("> Game State: {} ", game);

    // print!("> Players: ");
    // game.players
    //     .iter()
    //     .enumerate()
    //     .for_each(
    //         |(player_index, player)| match (player_index, current_index) {
    //             (player_index, PlayerIndex(current_index)) if player_index != current_index => {
    //                 print!("P{}:{{{}}} ", player_index, player)
    //             }
    //             (_, _) => {}
    //         },
    //     );

    // println!("");

    // println!("> Your hand [hints]: ");
    // current_player.hints_to_string();

    loop {
        println!("> What is your move? [play: p (card_index), discard: d (card_index), hint: h (player_index) (suit:RGYWB|face:12345)]");
        let player_action = get_player_input();
        match player_action {
            Ok(player_action) => return Ok(player_action),
            Err(msg) => println!("Failed to parsse action: {}", msg),
        };
    }
}

fn get_player_input() -> Result<PlayerAction, String> {
    let mut action_input = String::new();

    io::stdin()
        .read_line(&mut action_input)
        .expect("Failed to read line");

    let action_input = action_input.trim().to_lowercase();
    let action_input = action_input.split(" ").collect::<Vec<&str>>();

    match action_input[..] {
        ["p", card_index] => match card_index.trim().parse() {
            Ok(card_index) => Ok(PlayerAction::PlayCard(SlotIndex(card_index))),
            Err(_) => Err("invalid card number format".to_string()),
        },
        ["d", card_index] => match card_index.trim().parse() {
            Ok(card_index) => Ok(PlayerAction::DiscardCard(SlotIndex(card_index))),
            Err(_) => Err("Invalid card number format".to_string()),
        },
        ["h", player_index, suit_or_face] => match player_index.trim().parse() {
            Ok(player_index) => {
                let hint = match suit_or_face {
                    "r" => HintAction::SameSuit(CardSuit::Red),
                    "g" => HintAction::SameSuit(CardSuit::Green),
                    "y" => HintAction::SameSuit(CardSuit::Yellow),
                    "w" => HintAction::SameSuit(CardSuit::White),
                    "b" => HintAction::SameSuit(CardSuit::Blue),
                    "1" => HintAction::SameFace(CardFace::One),
                    "2" => HintAction::SameFace(CardFace::Two),
                    "3" => HintAction::SameFace(CardFace::Three),
                    "4" => HintAction::SameFace(CardFace::Four),
                    "5" => HintAction::SameFace(CardFace::Five),
                    _ => return Err("invalid suit or face".to_string()),
                };
                Ok(PlayerAction::GiveHint(PlayerIndex(player_index), hint))
            }
            Err(_) => Err("Bad player number format".to_string()),
        },
        _ => Err("invalid action".to_string()),
    }
}

fn init() {
    println!("{}", "Hanabi Simulator v0.1.0".blue());

    let result = run_hanabi();
    print!("Game ended: ");
    match result {
        Ok(GameOutcome::Win) => println!("Won!"),
        Ok(GameOutcome::Fail { score }) => println!("Finished with score: {}", score),
        Err(msg) => println!("Error: {}", msg),
    }
}

use std::{
    error::Error,
    io::{stdout, Stdout},
    ops::ControlFlow,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use ratatui::{
    prelude::*,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Padding, Paragraph, Wrap,
    },
};

// These type aliases are used to make the code more readable by reducing repetition of the generic
// types. They are not necessary for the functionality of the code.
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> BoxedResult<()> {
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(terminal)?;

    if let Err(err) = result {
        eprintln!("{err:?}");
    }
    Ok(())
}

fn setup_terminal() -> BoxedResult<Terminal> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal) -> BoxedResult<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn run(terminal: &mut Terminal) -> BoxedResult<()> {
    loop {
        terminal.draw(ui)?;
        if handle_events()?.is_break() {
            return Ok(());
        }
    }
}

fn handle_events() -> BoxedResult<ControlFlow<()>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(ControlFlow::Break(()));
            }
        }
    }
    Ok(ControlFlow::Continue(()))
}

fn ui(frame: &mut Frame) {
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
                        hints: vec![Hint::IsFace(CardFace::Three), Hint::IsSuit(CardSuit::Green)],
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
                        hints: vec![Hint::IsFace(CardFace::Three), Hint::IsSuit(CardSuit::Green)],
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
                        hints: vec![Hint::IsFace(CardFace::Three), Hint::IsSuit(CardSuit::Green)],
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

    let (title_area, layout) = calculate_layout(frame.size());

    render_title(frame, title_area);

    let paragraph = placeholder_paragraph();

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
            .border_style(
                Style::new()
                    .fg(color)
                    .add_modifier(Modifier::BOLD)
                    .dim()
                    .dim(),
            );
        // .bg(colorize_suit(card.suit));
        block
    }

    fn render_card(face: Option<CardFace>, suit: Option<CardSuit>) -> Paragraph<'static> {
        let color = suit.map(colorize_suit).unwrap_or(Color::DarkGray);
        let p = Paragraph::new(face.map(|f| f.key()).unwrap_or("?").to_string())
            .style(Style::new().fg(color).bold());
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(color).add_modifier(Modifier::BOLD));
        // .bg(colorize_suit(card.suit));

        p.block(block)
    }

    fn render_board(game_state: &ClientGameState, frame: &mut Frame, area: Rect) {
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
        let hint_span = Span::from("\u{f444} ".repeat(game_state.remaining_hint_count as usize))
            .style(Style::default().fg(Color::White));

        let hints_remaining = [hint_title, hint_span];

        let bomb_title = Span::from(format!("{:<8}", "bombs:"))
            .style(Style::default().not_bold().fg(Color::Gray).dim());
        let bomb_span = Span::from("\u{f0691} ".repeat(game_state.remaining_bomb_count as usize))
            .style(Style::default().fg(Color::White));
        let bombs_remaining: [Span<'_>; 2] = [bomb_title, bomb_span];

        let discards: Vec<_> = all_suits
            .iter()
            .enumerate()
            .map(|(suit_index, &cur_suit)| {
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

    fn render_player(
        player: &ClientPlayerView,
        is_current_turn: bool,
        frame: &mut Frame,
        area: Rect,
    ) {
        let num_cards = match player {
            ClientPlayerView::Me { hand } => hand.len(),
            ClientPlayerView::Teammate(Player { hand }) => hand.len(),
        };
        let player_block = Block::new()
            .borders(Borders::ALL)
            .border_type(if is_current_turn {
                BorderType::Double
            } else {
                BorderType::Rounded
            })
            .border_style(Style::default().fg(if is_current_turn {
                Color::Magenta
            } else {
                Color::White
            }))
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

        // match player {
        //     ClientPlayerView::Me { hand } => {}
        //     ClientPlayerView::Teammate(Player { hand }) => {
        //         for (index, slot) in hand.iter().enumerate() {
        //             match slot {
        //                 Some(Slot { card, hints }) => {
        //                     let card_ui = render_card(Some(card.face), Some(card.suit));
        //                     let x = player_rect.x + index as u16 * 3;
        //                     let y = player_rect.y;

        //                     frame.render_widget(
        //                         card_ui,
        //                         // Paragraph::new("card 1".dark_gray())
        //                         //     .wrap(Wrap { trim: true })
        //                         //     .block(player),
        //                         Rect {
        //                             x: x,
        //                             y: y,
        //                             width: 3,
        //                             height: 3,
        //                         }, //layout[0][0],
        //                     );
        //                 }
        //                 _ => {}
        //             }
        //         }
        //     }
        // }

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
                            Hint::IsFace(face) => Span::styled(
                                face.key().to_string(),
                                Style::new().fg(Color::DarkGray),
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
                    .enumerate()
                    .filter_map(|(index, hint)| {
                        Some(Line::from(match hint {
                            Hint::IsNotSuit(suit) => Span::styled(
                                suit.key().to_string(),
                                Style::new().fg(colorize_suit(*suit)),
                            ),
                            Hint::IsNotFace(face) => Span::styled(
                                face.key().to_string(),
                                Style::new().fg(Color::DarkGray),
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

    // let player = Block::new()
    //     .borders(Borders::ALL)
    //     .border_type(BorderType::Rounded)
    //     .border_style(Style::new().white().add_modifier(Modifier::DIM))
    //     .title(format!("Mirza"));

    for (index, client) in client_game_state.players.iter().enumerate() {
        render_player(
            client,
            client_game_state.turn == index as u8,
            frame,
            Rect {
                x: 2 + 14 * index as u16,
                y: 2,
                width: 4 * 3 + 2,
                height: 16,
            },
        );
    }

    render_board(
        &client_game_state,
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

    fn render_game_log(game: &ClientGameState, frame: &mut Frame, area: Rect) {
        let game_log_block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Game Log");
        let inner_rect = game_log_block.inner(area);

        let log = Paragraph::new("").block(game_log_block);
        frame.render_widget(log, area)
    }

    fn render_game_actions(frame: &mut Frame, area: Rect) {
        let legend_items: HashMap<LegendMode, Vec<LegendItem>> = [
            (
                LegendMode::PickActionType,
                vec![
                    LegendItem {
                        desc: "Play Card".to_string(),
                        key: "p".to_string(),
                        actions: vec![LegendMode::PickCard],
                    },
                    LegendItem {
                        desc: "Discard Card".to_string(),
                        key: "d".to_string(),
                        actions: vec![LegendMode::PickCard],
                    },
                    LegendItem {
                        desc: "Give Hint".to_string(),
                        key: "h".to_string(),
                        actions: vec![LegendMode::PickPlayer, LegendMode::PickHint],
                    },
                ],
            ),
            (
                LegendMode::PickPlayer,
                vec![LegendItem {
                    desc: "Select".to_string(),
                    key: "\u{f09f}".to_string(),
                    actions: vec![],
                }],
            ),
            (
                LegendMode::PickCard,
                vec![LegendItem {
                    desc: "Select".to_string(),
                    key: "\u{f09f}".to_string(),
                    actions: vec![],
                }],
            ),
            (
                LegendMode::PickHint,
                vec![
                    LegendItem {
                        desc: "Suit".to_string(),
                        key: "rgbyw".to_string(),
                        actions: vec![],
                    },
                    LegendItem {
                        desc: "Face".to_string(),
                        key: "12345".to_string(),
                        actions: vec![],
                    },
                ],
            ),
        ]
        .into_iter()
        .collect();

        let legend_items = [
            LegendItem {
                desc: "Play Card".to_string(),
                key: "p".to_string(),
                actions: vec![LegendMode::PickCard],
            },
            LegendItem {
                desc: "Discard Card".to_string(),
                key: "d".to_string(),
                actions: vec![LegendMode::PickCard],
            },
            LegendItem {
                desc: "Give Hint".to_string(),
                key: "h".to_string(),
                actions: vec![LegendMode::PickPlayer, LegendMode::PickHint],
            },
        ];

        let legend_string: Vec<_> = legend_items.iter().map(Some).intersperse(None).collect();
        let lines: Vec<_> = legend_string
            .into_iter()
            .map(|legend| match legend {
                Some(LegendItem { desc, key, actions }) => {
                    Span::from(format!("{} [{}]", desc, key)).style(
                        Style::default()
                            .bg(Color::Rgb(117, 158, 179))
                            .fg(Color::White),
                    )
                }
                None => Span::raw(" "),
            })
            .collect();

        let legend: Paragraph<'_> =
            Paragraph::new(Line::from_iter(lines.into_iter())).style(Style::new());

        frame.render_widget(legend, area)
    }

    render_game_log(
        &client_game_state,
        frame,
        Rect {
            x: 14 * 4 + 2,
            y: 2,
            width: frame.size().width - 14 * 4,
            height: 30,
        },
    );

    let mut state = ActionPickerState {
        current_mode: LegendMode::PickActionType,
        stack: vec![],
    };

    frame.render_stateful_widget(
        ActionPicker {},
        Rect {
            x: 14 * 4 + 2,
            y: 30 + 2,
            width: frame.size().width - 14 * 4,
            height: 1,
        },
        &mut state,
    );

    // render_game_actions(
    //     frame,
    //     Rect {
    //         x: 14 * 4 + 2,
    //         y: 30 + 2,
    //         width: frame.size().width - 14 * 4,
    //         height: 1,
    //     },
    // );

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

#[derive(PartialEq, Eq, Hash)]
enum LegendMode {
    PickActionType,
    PickPlayer,
    PickCard,
    PickHint,
}

struct LegendItem {
    desc: String,
    key: String,
    actions: Vec<LegendMode>,
}

pub struct ActionPickerState {
    current_mode: LegendMode,
    stack: Vec<LegendMode>,
}

pub struct ActionPicker {}

impl ActionPicker {
    pub fn render_game_actions(self, area: Rect, buf: &mut Buffer) {
        let legend_items: HashMap<LegendMode, Vec<LegendItem>> = [
            (
                LegendMode::PickActionType,
                vec![
                    LegendItem {
                        desc: "Play Card".to_string(),
                        key: "p".to_string(),
                        actions: vec![LegendMode::PickCard],
                    },
                    LegendItem {
                        desc: "Discard Card".to_string(),
                        key: "d".to_string(),
                        actions: vec![LegendMode::PickCard],
                    },
                    LegendItem {
                        desc: "Give Hint".to_string(),
                        key: "h".to_string(),
                        actions: vec![LegendMode::PickPlayer, LegendMode::PickHint],
                    },
                ],
            ),
            (
                LegendMode::PickPlayer,
                vec![LegendItem {
                    desc: "Select".to_string(),
                    key: "\u{f09f}".to_string(),
                    actions: vec![],
                }],
            ),
            (
                LegendMode::PickCard,
                vec![LegendItem {
                    desc: "Select".to_string(),
                    key: "\u{f09f}".to_string(),
                    actions: vec![],
                }],
            ),
            (
                LegendMode::PickHint,
                vec![
                    LegendItem {
                        desc: "Suit".to_string(),
                        key: "rgbyw".to_string(),
                        actions: vec![],
                    },
                    LegendItem {
                        desc: "Face".to_string(),
                        key: "12345".to_string(),
                        actions: vec![],
                    },
                ],
            ),
        ]
        .into_iter()
        .collect();

        let legend_items = [
            LegendItem {
                desc: "Play Card".to_string(),
                key: "p".to_string(),
                actions: vec![LegendMode::PickCard],
            },
            LegendItem {
                desc: "Discard Card".to_string(),
                key: "d".to_string(),
                actions: vec![LegendMode::PickCard],
            },
            LegendItem {
                desc: "Give Hint".to_string(),
                key: "h".to_string(),
                actions: vec![LegendMode::PickPlayer, LegendMode::PickHint],
            },
        ];

        let legend_string: Vec<_> = legend_items.iter().map(Some).intersperse(None).collect();
        let lines: Vec<_> = legend_string
            .into_iter()
            .map(|legend| match legend {
                Some(LegendItem { desc, key, actions }) => {
                    Span::from(format!("{} [{}]", desc, key)).style(
                        Style::default()
                            .bg(Color::Rgb(117, 158, 179))
                            .fg(Color::White),
                    )
                }
                None => Span::raw(" "),
            })
            .collect();

        buf.set_line(
            area.x,
            area.y,
            &Line::from_iter(lines.into_iter()),
            area.width,
        );
    }
}

impl StatefulWidget for ActionPicker {
    type State = ActionPickerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.render_game_actions(area, buf);
    }
}

/// Calculate the layout of the UI elements.
///
/// Returns a tuple of the title area and the main areas.
fn calculate_layout(area: Rect) -> (Rect, Vec<Vec<Rect>>) {
    let main_layout = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]);
    let block_layout = Layout::vertical([Constraint::Max(4); 9]);
    let [title_area, main_area] = main_layout.areas(area);
    let main_areas = block_layout
        .split(main_area)
        .iter()
        .map(|&area| {
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area)
                .to_vec()
        })
        .collect_vec();
    (title_area, main_areas)
}

fn render_title(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new("Block example. Press q to quit")
            .dark_gray()
            .alignment(Alignment::Center),
        area,
    );
}

fn placeholder_paragraph() -> Paragraph<'static> {
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";
    Paragraph::new(text.dark_gray()).wrap(Wrap { trim: true })
}

fn render_borders(paragraph: &Paragraph, border: Borders, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(border)
        .title(format!("Borders::{border:#?}"));
    frame.render_widget(paragraph.clone().block(block), area);
}

fn render_hidden_card(card: Card) {}

fn render_border_type(
    paragraph: &Paragraph,
    border_type: BorderType,
    frame: &mut Frame,
    area: Rect,
) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(border_type)
        .title(format!("BorderType::{border_type:#?}"));
    frame.render_widget(paragraph.clone().block(block), area);
}
fn render_styled_borders(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::new().blue().on_white().bold().italic())
        .title("Styled borders");
    frame.render_widget(paragraph.clone().block(block), area);
}

fn render_styled_block(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .style(Style::new().blue().on_white().bold().italic())
        .title("Styled block");
    frame.render_widget(paragraph.clone().block(block), area);
}

// Note: this currently renders incorrectly, see https://github.com/ratatui-org/ratatui/issues/349
fn render_styled_title(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .title("Styled title")
        .title_style(Style::new().blue().on_white().bold().italic());
    frame.render_widget(paragraph.clone().block(block), area);
}

fn render_styled_title_content(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        "Styled ".blue().on_white().bold().italic(),
        "title content".red().on_white().bold().italic(),
    ]);
    let block = Block::new().borders(Borders::ALL).title(title);
    frame.render_widget(paragraph.clone().block(block), area);
}

fn render_multiple_titles(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .title("Multiple".blue().on_white().bold().italic())
        .title("Titles".red().on_white().bold().italic());
    frame.render_widget(paragraph.clone().block(block), area);
}

fn render_multiple_title_positions(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .title(
            Title::from("top left")
                .position(Position::Top)
                .alignment(Alignment::Left),
        )
        .title(
            Title::from("top center")
                .position(Position::Top)
                .alignment(Alignment::Center),
        )
        .title(
            Title::from("top right")
                .position(Position::Top)
                .alignment(Alignment::Right),
        )
        .title(
            Title::from("bottom left")
                .position(Position::Bottom)
                .alignment(Alignment::Left),
        )
        .title(
            Title::from("bottom center")
                .position(Position::Bottom)
                .alignment(Alignment::Center),
        )
        .title(
            Title::from("bottom right")
                .position(Position::Bottom)
                .alignment(Alignment::Right),
        );
    frame.render_widget(paragraph.clone().block(block), area);
}

fn render_padding(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let block = Block::new()
        .borders(Borders::ALL)
        .title("Padding")
        .padding(Padding::new(5, 10, 1, 2));
    frame.render_widget(paragraph.clone().block(block), area);
}

fn render_nested_blocks(paragraph: &Paragraph, frame: &mut Frame, area: Rect) {
    let outer_block = Block::new().borders(Borders::ALL).title("Outer block");
    let inner_block = Block::new().borders(Borders::ALL).title("Inner block");
    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);
    frame.render_widget(paragraph.clone().block(inner_block), inner);
}
