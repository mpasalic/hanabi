use hanabi_app::HanabiApp;
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
mod hanabi_app;
mod logic;
mod model;

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
    let mut app = HanabiApp::new();

    let result = app.run(&mut terminal);
    restore_terminal(terminal)?;

    if let Err(err) = result {
        eprintln!("{err:?}  ");
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
