use colored::Colorize;
use itertools::Itertools;
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
use std::fmt;
use std::io;
use strum::IntoEnumIterator;

use crate::model::GameLog;
use crate::model::GameOutcome;
use crate::model::SlotIndex;
mod logic;
mod model;

trait CardKey {
    fn key(&self) -> &str;
}

trait ColoredCard {
    fn color(&self) -> colored::ColoredString;
    fn color_string(&self, string: String) -> colored::ColoredString;
    fn inactive_color(&self) -> colored::ColoredString;
}

impl CardKey for CardSuit {
    fn key(&self) -> &str {
        match self {
            CardSuit::Red => "R",
            CardSuit::Green => "G",
            CardSuit::Yellow => "Y",
            CardSuit::White => "W",
            CardSuit::Blue => "B",
        }
    }
}

impl ColoredCard for CardSuit {
    fn color_string(&self, string: String) -> colored::ColoredString {
        match self {
            CardSuit::Red => string.red(),
            CardSuit::Green => string.green(),
            CardSuit::Yellow => string.yellow(),
            CardSuit::White => string.white(),
            CardSuit::Blue => string.blue(),
        }
    }

    fn color(&self) -> colored::ColoredString {
        self.color_string(self.key().to_string()).bold()
    }

    fn inactive_color(&self) -> colored::ColoredString {
        self.key().to_string().dimmed()
    }
}

impl CardKey for CardFace {
    fn key(&self) -> &str {
        match self {
            CardFace::One => "1",
            CardFace::Two => "2",
            CardFace::Three => "3",
            CardFace::Four => "4",
            CardFace::Five => "5",
        }
    }
}

impl ColoredCard for CardFace {
    fn color_string(&self, string: String) -> colored::ColoredString {
        string.bold()
    }

    fn color(&self) -> colored::ColoredString {
        self.color_string(self.key().to_string())
    }

    fn inactive_color(&self) -> colored::ColoredString {
        self.key().to_string().dimmed()
    }
}

impl fmt::Display for CardSuit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.color())
    }
}

impl fmt::Display for CardFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key())
    }
}

// TODO Simon: To finish new formatter
// impl fmt::Display for Card {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.suit.color_string( self.suit.to_string() + self.face.key()))
//     }
// }

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.suit.color_string(self.face.key().to_string()))
    }
}

fn fmt_card(card: Card, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", card)
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for slot in self.hand.iter() {
            if let Some(Slot { card, hints: _ }) = slot {
                // TODO fmt_card(*card, f)?;
                write!(f, "[{}]", *card)?;
            }
        }
        return fmt::Result::Ok(());
        //write!(f, "[{:?},{:?},{:?},{:?},{:?}]", self.hand[0], self.hand[1], self.hand[2], self.hand[3], self.hand[4])
    }
}

// TODO Simon: To finish new formatter
impl Player {
    /**
     * --new version--
     *
     * Card 1: R + 1 + Not(Y, 2, B)
     * Card 1: Not(Y, 2, B)
     * Card 2: R + 1
     * Card 3: done
     * Card 4: no hints
     *
     */

    fn hints_to_string(&self) -> String {
        let mut output = String::new();
        for (pos, slot) in self.hand.iter().enumerate() {
            output.push_str(format!("Card {}: ", pos + 1).as_str());

            let Some(slot) = slot else {
                output.push_str("done\n");
                continue;
            };

            let Slot { card: _, hints } = slot;

            let positive = hints
                .iter()
                .filter_map(|h| match h {
                    Hint::IsSuit(suit) => Some(format!("{}", suit)),
                    Hint::IsFace(face) => Some(format!("{}", face)),
                    _ => None,
                })
                .join(" + ");

            let negative = hints
                .iter()
                .filter_map(|h| match h {
                    Hint::IsNotSuit(suit) => Some(format!("{}", suit)),
                    Hint::IsNotFace(face) => Some(format!("{}", face)),
                    _ => None,
                })
                .join(", ");

            let negative = format!("Not({negative})", negative = negative);

            let to_print = match (positive.as_str(), negative.as_str()) {
                ("", "Not()") => "no hints".to_string(),
                ("", negative) => negative.to_string(),
                (positive, "Not()") => positive.to_string(),
                (positive, negative) => format!("{} + {}", positive, negative),
            };
            output.push_str(to_print.as_str());
            output.push_str("\n")
        }

        output
    }
}

impl fmt::Display for PlayerIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerIndex(player_index) => write!(f, "P{}", player_index),
        }
    }
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State {{").expect("format");
        write!(f, " bombs={}", self.remaining_bomb_count).expect("format");
        for _ in 0..self.remaining_bomb_count {
            write!(f, "X").expect("format");
        }
        write!(f, " hints={}", self.remaining_hint_count).expect("format");
        for _ in 0..self.remaining_hint_count {
            write!(f, "!").expect("format");
        }

        write!(f, " board=").expect("format");
        for suit in CardSuit::iter() {
            for face in CardFace::iter() {
                let card = Card { face, suit };
                if self.played_cards.contains(&card) {
                    fmt_card(card, f).expect("format");
                } else {
                    write!(f, "_").expect("format")
                }
            }
            write!(f, "|").expect("format");
        }

        write!(f, " discard=").expect("format");
        for card in self.discard_pile.iter() {
            fmt_card(*card, f).expect("format");
        }

        write!(f, " draw={}", self.draw_pile.len()).expect("format");

        write!(f, " }}\n").expect("format");

        self.players.iter().enumerate().for_each(|(index, player)| {
            write!(f, "\nPlayer {}", index + 1).expect("format");
            match self.current_player_index() {
                PlayerIndex(player_index) if player_index == index => {
                    write!(f, " <- current turn").expect("format");
                }
                _ => {}
            }
            write!(f, "\n{}", player.hints_to_string()).expect("format");
        });

        // let hint_output: Vec<Vec<String>> = self
        //     .players
        //     .iter()
        //     .map(|player| {
        //         player.hints_to_string()
        //         // .split("\n")
        //         // .map(|s| String::from(s))
        //         // .collect()
        //     })
        //     .collect();

        // // write!(f, "{:?}", hint_output)?;

        // let max_rows = hint_output.iter().map(|row| row.len()).max().unwrap();

        // write!(f, "\n")?;
        // for row_index in 0..hint_output.len() {
        //     if row_index == self.current_player_index().0 {
        //         write!(f, "Player {} (turn)\t\t", row_index + 1)?;
        //     } else {
        //         write!(f, "Player {}\t\t", row_index + 1)?;
        //     }
        // }
        // write!(f, "\n")?;
        // for row_index in 0..max_rows {
        //     // write!(f, "Card {}\t\t", row_index + 1)?;
        //     for hint_output_line in hint_output.iter() {
        //         if let Some(hint_output_line) = hint_output_line.get(row_index) {
        //             write!(f, "{}\t", hint_output_line)?;
        //         }
        //     }
        //     write!(f, "\n")?;
        // }

        return fmt::Result::Ok(());
    }
}

fn run_hanabi() -> Result<GameOutcome, String> {
    let num_players: usize = 4;
    let hand_size: usize = 4;

    let mut game_log = GameLog::new(num_players, hand_size);

    println!("> Starting Game!");

    let mut game_state = game_log.generate_state()?;

    while let None = game_state.check_game_outcome() {
        let game = game_log.generate_state()?;

        println!(
            "############ ROUND #{} PLAYER #{} ############",
            game.current_round(),
            game.current_player_index()
        );

        loop {
            let player_action = player_turn(&game)?;
            match player_action {
                Command::GameMove(player_action) => {
                    game_log.log(player_action);
                }
                Command::Undo => {
                    game_log.undo();
                }
                Command::Quit => {
                    return Ok(GameOutcome::Fail { score: 0 });
                }
            }

            let new_game_state = game_log.generate_state();
            match new_game_state {
                Ok(new_game_state) => {
                    game_state = new_game_state;
                    break;
                }
                Err(msg) => println!("Disallowed action: {}", msg),
            }
        }
    }

    match game_state.check_game_outcome() {
        Some(game_outcome) => Ok(game_outcome),
        None => Err("Error".to_string()),
    }
}

enum Command {
    GameMove(PlayerAction),
    Undo,
    Quit,
}

fn player_turn(game: &GameState) -> Result<Command, String> {
    println!("> Game State: {} ", game);

    loop {
        println!("> What is your move? [play: p (slot) (suit)(face), discard: d (slot) (suit)(face), hint: h (player_index) (suit|face) (slots), undo: u, quit: q ] (suits = rgywb, faces = 12345)");
        let player_action = get_player_input();
        match player_action {
            Ok(player_action) => return Ok(player_action),
            Err(msg) => println!("Failed to parsse action: {}", msg),
        };
    }
}

fn parse_card_input(card_input: &str) -> Result<Card, String> {
    let card_input = card_input.trim().to_lowercase();
    let card_input = card_input.chars().into_iter().collect::<Vec<char>>();

    fn match_card_suit(x: &char) -> Result<CardSuit, String> {
        match *x {
            'r' => Ok(CardSuit::Red),
            'g' => Ok(CardSuit::Green),
            'y' => Ok(CardSuit::Yellow),
            'w' => Ok(CardSuit::White),
            'b' => Ok(CardSuit::Blue),
            _ => Err("invalid card suit".to_string()),
        }
    }

    fn match_card_num(x: &char) -> Result<CardFace, String> {
        match *x {
            '1' => Ok(CardFace::One),
            '2' => Ok(CardFace::Two),
            '3' => Ok(CardFace::Three),
            '4' => Ok(CardFace::Four),
            '5' => Ok(CardFace::Five),
            _ => Err("invalid card number".to_string()),
        }
    }

    match card_input[..] {
        [suit, face] => match (match_card_suit(&suit), match_card_num(&face)) {
            (Ok(suit), Ok(face)) => Ok(Card { face, suit }),
            (_, _) => Err("invalid card number format".to_string()),
        },
        _ => Err("invalid action".to_string()),
    }
}

fn parse_player_index(player_index_input: &str) -> Result<PlayerIndex, String> {
    let player_index = player_index_input.trim().parse::<usize>();
    Ok(PlayerIndex(
        player_index
            .map_err(|_err| "Cannot parse player index")?
            .checked_sub(1)
            .ok_or_else(|| "Cannot parse player index")?,
    ))
}

fn parse_slot_index(slot_index_input: &str) -> Result<SlotIndex, String> {
    let slot_index = slot_index_input.trim().parse::<usize>();
    Ok(SlotIndex(
        slot_index
            .map_err(|_err| "Cannot parse card index")?
            .checked_sub(1)
            .ok_or_else(|| "Cannot parse card index")?,
    ))
}

fn get_player_input() -> Result<Command, String> {
    let mut action_input = String::new();

    io::stdin()
        .read_line(&mut action_input)
        .expect("Failed to read line");

    let action_input = action_input.trim().to_lowercase();
    let action_input = action_input.split(" ").collect::<Vec<&str>>();

    match action_input[..] {
        ["q"] => Ok(Command::Quit),
        ["u"] => Ok(Command::Undo),
        ["p", card_index, card_input] => {
            let card = parse_card_input(card_input)?;
            Ok(Command::GameMove(PlayerAction::PlayCard(
                parse_slot_index(card_index)?,
                card,
            )))
        }
        ["d", card_index, card_input] => {
            let card = parse_card_input(card_input)?;
            Ok(Command::GameMove(PlayerAction::DiscardCard(
                parse_slot_index(card_index)?,
                card,
            )))
        }
        ["h", player_index, suit_or_face, rest] => {
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
            let slots: Vec<SlotIndex> = rest
                .chars()
                .into_iter()
                .filter_map(|c| parse_slot_index(c.to_string().as_str()).ok())
                .collect();

            Ok(Command::GameMove(PlayerAction::GiveHint(
                parse_player_index(player_index)?,
                slots,
                hint,
            )))
        }
        _ => Err("invalid action".to_string()),
    }
}

fn main() {
    println!("{}", "Hanabi Simulator v0.1.0".blue());

    let result = run_hanabi();
    print!("Game ended: ");
    match result {
        Ok(GameOutcome::Win) => println!("Won!"),
        Ok(GameOutcome::Fail { score }) => println!("Finished with score: {}", score),
        Err(msg) => println!("Error: {}", msg),
    }
}
