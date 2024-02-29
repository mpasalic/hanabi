use colored::Colorize;
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
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use strum::IntoEnumIterator;
use websocket::Message;
use websocket::OwnedMessage;

use crate::model::GameLog;
use crate::model::GameOutcome;
use crate::model::SlotIndex;
mod logic;
mod model;
use urlencoding::decode;

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
//         for (pos, slot) in self.hand.iter().enumerate() {
//             print!("Card {}: ", pos + 1);

//             let Some(slot) = slot else {
//                 println!("empty");
//                 continue;
//             };

//             let Slot { card: _, hints } = slot;
//             let mut face_hints_set: HashSet<CardFace> = HashSet::new();
//             let mut suit_hints_set: HashSet<CardSuit> = HashSet::new();

//             let positive = hints.iter().filter_map(|h| match h {
//                 Hint::IsSuit(suit) => Some(format!("{}", suit)),
//                 Hint::IsFace(face) => Some(format!("{}", face)),
//                 _ => None
//     // IsNotSuit(CardSuit),
//     // IsNotFace(CardFace),

//             });

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

impl Player {
    /**
     * Your hand [hints]: [? ![Three, Five, Two][Green]] [5] [?2 ![Green]] [3] [?3 ![Green]]
     *
     * --new version--
     *
     * Card 1: RGYWB 1 2 3 4 5
     * Card 1: RYWB 1 4 3 4
     * Card 2: G5
     * Card 3:
     * Card 4: G3
     * Card 5:
     *
     */

    fn hints_to_string(&self) -> String {
        let mut counter = 0;
        let slots = self.hand.iter().filter_map(|slot| slot.as_ref());

        let mut output = String::new();

        for slot in slots {
            let Slot { card: _, hints } = slot;
            output.push_str(format!("Card {}: ", counter).as_str());

            let mut face_hints_set: HashSet<CardFace> = HashSet::new();
            let mut suit_hints_set: HashSet<CardSuit> = HashSet::new();

            for face in CardFace::iter() {
                if !hints.iter().any(|hint| match hint {
                    Hint::IsFace(hint_face) => *hint_face != face,
                    Hint::IsNotFace(not_face) => *not_face == face,
                    _ => false,
                }) {
                    face_hints_set.insert(face.clone());
                }
            }

            for suit in CardSuit::iter() {
                if !hints.iter().any(|hint| match hint {
                    Hint::IsSuit(suit_hint) => *suit_hint != suit,
                    Hint::IsNotSuit(not_suit) => *not_suit == suit,
                    _ => false,
                }) {
                    suit_hints_set.insert(suit.clone());
                }
            }

            let face_hints_output: String = CardFace::iter()
                .into_iter()
                .map(|face| {
                    if face_hints_set.contains(&face) {
                        format!("{}", face.color())
                    } else {
                        format!("{}", face.inactive_color())
                    }
                })
                .collect();

            let suit_hints_output: String = CardSuit::iter()
                .into_iter()
                .map(|suit| {
                    if suit_hints_set.contains(&suit) {
                        format!("{}", suit.color())
                    } else {
                        format!("{}", suit.inactive_color())
                    }
                })
                .collect();

            output.push_str(format!("{}\t{}\n", suit_hints_output, face_hints_output).as_str());
            counter = counter + 1;
        }
        return output;
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

        write!(f, " }}").expect("format");

        let hint_output: Vec<Vec<String>> = self
            .players
            .iter()
            .map(|player| {
                let output_lines: Vec<String> = player
                    .hints_to_string()
                    .split("\n")
                    .map(|s| String::from(s))
                    .collect();
                return output_lines;
            })
            .collect();

        // write!(f, "{:?}", hint_output)?;

        let max_rows = hint_output.iter().map(|row| row.len()).max().unwrap();

        write!(f, "\n")?;
        for row_index in 0..hint_output.len() {
            if row_index == self.current_player_index().0 {
                write!(f, "Player {} (turn)\t\t", row_index)?;
            } else {
                write!(f, "Player {}\t\t", row_index)?;
            }
        }
        write!(f, "\n")?;
        for row_index in 0..max_rows {
            // write!(f, "Card {}\t\t", row_index + 1)?;
            for hint_output_line in hint_output.iter() {
                if let Some(hint_output_line) = hint_output_line.get(row_index) {
                    write!(f, "{}\t", hint_output_line)?;
                }
            }
            write!(f, "\n")?;
        }

        return fmt::Result::Ok(());
    }
}

fn run_hanabi(game_log_mutex: Arc<Mutex<GameLog>>) -> Result<GameOutcome, String> {
    let num_players: usize = 5;

    println!("> Starting Game!");

    loop {
        let game_state = { game_log_mutex.lock().unwrap().generate_state().unwrap() };

        if let Some(game_outcome) = game_state.check_game_outcome() {
            return Ok(game_outcome);
        }

        let player_action = player_turn(&game_state)?;

        match player_action {
            Command::GameMove(player_action) => {
                let mut game_log = game_log_mutex.lock().unwrap();
                game_log.log(player_action);
            }
            Command::Undo => {
                let mut game_log = game_log_mutex.lock().unwrap();
                game_log.undo();
            }
            Command::Quit => {
                return Ok(GameOutcome::Fail { score: 0 });
            }
        }
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

fn get_player_input() -> Result<Command, String> {
    let mut action_input = String::new();

    io::stdin()
        .read_line(&mut action_input)
        .expect("Failed to read line");

    return parse_player_input(&action_input);
}

fn parse_player_input(action_input: &str) -> Result<Command, String> {
    let action_input = action_input.trim().to_lowercase();
    let action_input = action_input.split(" ").collect::<Vec<&str>>();

    match action_input[..] {
        ["q"] => Ok(Command::Quit),
        ["u"] => Ok(Command::Undo),
        ["p", card_index, card_input] => match card_index.trim().parse() {
            Ok(card_index) => {
                let card = parse_card_input(card_input)?;
                Ok(Command::GameMove(PlayerAction::PlayCard(
                    SlotIndex(card_index),
                    card,
                )))
            }
            Err(_) => Err("invalid card number format".to_string()),
        },
        ["d", card_index, card_input] => match card_index.trim().parse() {
            Ok(card_index) => {
                let card = parse_card_input(card_input)?;
                Ok(Command::GameMove(PlayerAction::DiscardCard(
                    SlotIndex(card_index),
                    card,
                )))
            }
            Err(_) => Err("Invalid card number format".to_string()),
        },
        ["h", player_index, suit_or_face, rest] => match player_index.trim().parse() {
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
                let slots: Vec<SlotIndex> = rest
                    .chars()
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, value)| match value {
                        '1' => Some(SlotIndex(index)),
                        _ => None,
                    })
                    .collect();

                Ok(Command::GameMove(PlayerAction::GiveHint(
                    PlayerIndex(player_index),
                    slots,
                    hint,
                )))
            }
            Err(_) => Err("Bad player number format".to_string()),
        },
        _ => Err("invalid action".to_string()),
    }
}

enum MessageAPI {
    PlayerMove,
    Ping,
    Pong,
    Close,
}

struct SocketClient {
    id: String,
    sender: Option<mpsc::Sender<OwnedMessage>>,
}

fn main() {
    println!("{}", "Hanabi Simulator v0.1.0".blue());
    let num_players: usize = 5;

    // Channel: When progress happens, triggers a refresh of all socket clients.
    let (broadcast_sender, broadcast_receiver) = mpsc::channel();

    let game_log = Arc::new(Mutex::new(GameLog::new(num_players)));
    let socket_clients: Arc<Mutex<HashMap<String, SocketClient>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let game_server_handle = {
        let game_log = Arc::clone(&game_log);
        thread::spawn(move || {
            let result = run_hanabi(game_log);
            match result {
                Ok(GameOutcome::Win) => println!("Won!"),
                Ok(GameOutcome::Fail { score }) => println!("Finished with score: {}", score),
                Err(msg) => println!("Error: {}", msg),
            }
        })
    };

    let broadcast_receiver_handle = {
        let socket_clients = Arc::clone(&socket_clients);
        thread::spawn(move || {
            while let Ok(_) = broadcast_receiver.recv() {
                let socket_clients = socket_clients.lock().unwrap();
                for socket_client in socket_clients.values().into_iter() {
                    if let Some(socket_client_sender) = &socket_client.sender {
                        let refresh_message = OwnedMessage::Text(String::from("refresh"));
                        let result = socket_client_sender.send(refresh_message);
                        match result {
                            Ok(_) => println!("Refreshed client {}", socket_client.id),
                            Err(error) => {
                                println!(
                                    "Error: Could not refresh client {}: {}",
                                    socket_client.id, error
                                )
                            }
                        }
                    }
                }
            }
        })
    };

    let rest_handle = {
        let broadcast_sender = broadcast_sender.clone();

        thread::spawn(move || {
            let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
            println!("Listening for HTTP requests");
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                let game_log = Arc::clone(&game_log);

                let broadcast_sender = broadcast_sender.clone();
                thread::spawn(move || {
                    let result = handle_connection(stream, game_log);
                    if let Ok(true) = result {
                        broadcast_sender
                            .send("refresh")
                            .expect("Failed to broadcast refresh");
                    }
                })
                .join()
                .expect("Error handling connection");
            }
        })
    };

    let socket_handle = {
        let socket_handle_clients = Arc::clone(&socket_clients);
        thread::spawn(move || {
            println!("Listening for Socket requests");
            let socket_listener = websocket::sync::Server::bind("127.0.0.1:7879").unwrap();
            let mut counter = 0;
            for connection in socket_listener.filter_map(Result::ok) {
                let socket_clients = Arc::clone(&socket_handle_clients);
                let socket_id = counter;
                counter = counter + 1;
                thread::spawn(move || {
                    let mut ws_client = connection.accept().unwrap();

                    let (channel_sender, channel_receiver) = mpsc::channel();

                    let channel_id = String::from(format!("{}", socket_id));
                    println!("New socket connection {}", channel_id);

                    let socket_client = SocketClient {
                        id: channel_id.clone(),
                        sender: Some(channel_sender.clone()),
                    };

                    {
                        let mut socket_clients = socket_clients.lock().unwrap();
                        socket_clients.insert(channel_id.clone(), socket_client);
                    }

                    let message = Message::text("Hello, Hanabi client!");
                    ws_client
                        .send_message(&message)
                        .expect("Failed to make initial connection");

                    let (mut ws_receiver, mut ws_sender) = ws_client.split().unwrap();
                    {
                        let channel_id = channel_id.clone();
                        thread::spawn(move || {
                            println!("Socket channel {} listening", channel_id);
                            while let Ok(message) = channel_receiver.recv() {
                                println!(
                                    "Socket channel {} received message: {:?}",
                                    channel_id, message
                                );
                                let result = ws_sender.send_message(&message);

                                match result {
                                    Ok(_) => {
                                        println!("Sent message through socket: {:?}", message)
                                    }
                                    Err(error) => {
                                        println!("Error sending message to client {}", error)
                                    }
                                }
                            }
                        });
                    }

                    let ws_forwarder = channel_sender.clone();

                    println!("Socket TCP {} listening", channel_id);
                    for message in ws_receiver.incoming_messages() {
                        let message = message.unwrap();
                        println!("Socket TCP {} received message: {:?}", channel_id, message);
                        match message {
                            OwnedMessage::Close(_) => {
                                let message = OwnedMessage::Close(None);

                                ws_forwarder.send(message).expect("Error forwarding");
                                println!("Client disconnected");
                                break;
                            }
                            OwnedMessage::Ping(ping) => {
                                let message = OwnedMessage::Pong(ping);
                                ws_forwarder.send(message).expect("Error forwarding");
                            }
                            _ => ws_forwarder.send(message).expect("Error forwarding"),
                        }
                    }

                    {
                        let mut socket_clients = socket_clients.lock().unwrap();
                        socket_clients.remove(&channel_id);
                    }
                });
            }
        })
    };

    game_server_handle.join().unwrap();
    broadcast_receiver_handle.join().unwrap();
    rest_handle.join().unwrap();
    socket_handle.join().unwrap();
}

fn handle_connection(mut stream: TcpStream, game_log: Arc<Mutex<GameLog>>) -> Result<bool, String> {
    let (request_line, body) = parse_request(&mut stream).unwrap();

    println!("{}", request_line);
    println!("{:?}", body);

    if request_line == "GET / HTTP/1.1" {
        let game_state = { game_log.lock().unwrap().generate_state() };
        let response = generate_output(&game_state);
        stream.write_all(response.as_bytes()).unwrap();
        return Ok(false);
    } else if request_line == "POST / HTTP/1.1" {
        let body = body.unwrap();
        let body = decode(&body).unwrap();
        if let [field, value] = body.split("=").collect::<Vec<&str>>()[..] {
            if field == "command" {
                let decoded_command = value.replace("+", " ");
                println!("decoded as '{:?}'", decoded_command);
                let player_action = parse_player_input(&decoded_command);

                let mut game_log = game_log.lock().unwrap();
                match player_action {
                    Ok(Command::GameMove(game_move)) => {
                        game_log.log(game_move);
                        let game_state = game_log.generate_state();
                        let response = generate_output(&game_state);
                        stream.write_all(response.as_bytes()).unwrap();
                        return Ok(true);
                    }
                    Err(error) => {
                        let game_state = Err(error);
                        let response = generate_output(&game_state);
                        stream.write_all(response.as_bytes()).unwrap();
                    }
                    _ => {
                        todo!("Unsupported Player Action")
                    }
                }
            }
        } else {
            stream
                .write_all(
                    generate_error(format!(
                        "Unable to parse request: {} body: {}",
                        request_line, body
                    ))
                    .as_bytes(),
                )
                .unwrap();
        }
    } else {
        stream
            .write_all(generate_error(format!("Unsupported method: '{}'", request_line)).as_bytes())
            .unwrap();
    }
    return Err("TODO".to_string());
}

fn parse_request(mut stream: &TcpStream) -> Result<(String, Option<String>), String> {
    let mut buf_reader = BufReader::new(&mut stream);

    let mut request_line = "".to_string();
    buf_reader
        .read_line(&mut request_line)
        .map_err(|e| e.to_string())?;
    let request_line = request_line.trim().to_string();

    let mut content_length: Option<usize> = None;

    loop {
        let mut header_line = String::new();
        buf_reader
            .read_line(&mut header_line)
            .map_err(|e| e.to_string())?;

        if header_line.starts_with("Content-Length: ") {
            let content_length_str = header_line.split(": ").collect::<Vec<&str>>()[1].trim();
            content_length = Some(content_length_str.parse::<usize>().unwrap());
        }
        // The final line is just /r/n
        if header_line.len() == 2 {
            break;
        }
    }

    if let Some(content_length) = content_length {
        let mut content = vec![0u8; content_length];
        buf_reader
            .read_exact(&mut content)
            .map_err(|e| e.to_string())?;

        let body = String::from_utf8(content);

        return Ok((request_line, body.ok()));
    } else {
        return Ok((request_line, None));
    }
}

fn generate_output(game_state: &Result<GameState, String>) -> String {
    let status_line = "HTTP/1.1 200 OK";
    let base_content = fs::read_to_string("hello.html").unwrap();
    let contents = match game_state {
        Ok(game_state) => {
            let game_state_output = format!("{}", game_state);
            let game_state_html_output = ansi_to_html::convert(&game_state_output).unwrap();
            base_content.replace("{{output}}", game_state_html_output.as_str())
        }
        Err(msg) => base_content.replace("{{output}}", format!("Error: {}", msg).as_str()),
    };

    let length = contents.len();
    return format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}").to_string();
}

fn generate_error(error: String) -> String {
    println!("Error from connection: {}", error);

    let status_line = "HTTP/1.1 500  Internal Server Error";
    let length = error.len();
    return format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{error}").to_string();
}
