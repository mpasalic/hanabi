mod hanabi_app;
// mod lobby_app;
use std::{
    env,
    error::Error,
    io::{self, stdout, Stdout},
    process::exit,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::{self, Duration},
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use hanabi_app::HanabiApp;
use ratatui::prelude::*;
use shared::{
    client_logic::{ClientToServerMessage, GameLog, ServerToClientMessage},
    model::{ClientPlayerView, GameConfig, GameState, GameStateSnapshot, PlayerIndex},
};
use std::net::TcpStream;
use websocket::{
    sync::{Client, Writer},
    ClientBuilder, Message, OwnedMessage,
};

// These type aliases are used to make the code more readable by reducing repetition of the generic
// types. They are not necessary for the functionality of the code.
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;

enum Messages {
    UserInput(String),
    ServerMessage(ServerToClientMessage),
}

enum ConnectionError {}

struct ServerConnection {}

// impl ServerConnection {
//     pub fn connect(host: String, player_name: String, session_id: String) -> anyhow::Result<Self> {
//         let mut client = ClientBuilder::new(&host)?.connect_insecure()?;

//         let channel
//         let r = client.split()?;

//         let msg = OwnedMessage::Text(serde_json::to_string(&ClientToServerMessage::Join {
//             player_name,
//             session_id,
//         })?);

//         client.send_message(&msg)?;

//         Ok(Self { socket: client })
//     }

//     pub fn wait_for_next_message(&mut self) -> anyhow::Result<ServerToClientMessage> {
//         self.socket.set_nonblocking(false)?;

//         loop {
//             let message = self.socket.recv_message()?;
//             match message {
//                 OwnedMessage::Text(text) => {
//                     let message: ServerToClientMessage = serde_json::from_str(&text)?;
//                     return Ok(message);
//                 }
//                 _ => {}
//             }
//         }
//     }

//     pub fn next_message(&mut self) -> anyhow::Result<Option<ServerToClientMessage>> {
//         self.socket.set_nonblocking(true)?;

//         let message = self.socket.recv_message()?;
//         println!("Received: {:?}", message);
//         match message {
//             OwnedMessage::Text(text) => {
//                 let message: ServerToClientMessage = serde_json::from_str(&text)?;
//                 Ok(Some(message))
//             }
//             // OwnedMessage::Ping() => self.socket.send_message(OwnedMessage::Pong()),
//             _ => Ok(None),
//         }
//     }

//     pub fn send_message(&mut self, message: ClientToServerMessage) -> anyhow::Result<()> {
//         println!("Sending: {:?}", message);
//         self.socket
//             .send_message(&OwnedMessage::Text(serde_json::to_string(&message)?))?;
//         Ok(())
//     }

//     pub fn wait_for_game_start(&mut self) -> anyhow::Result<GameStateSnapshot> {
//         let stdin_channel = spawn_stdin_channel();
//         loop {
//             match stdin_channel.try_recv() {
//                 Ok(key) if key == "s" => {
//                     println!("Sending start signal!");
//                     self.send_message(ClientToServerMessage::StartGame)
//                         .expect("Game connection failed");
//                     println!("Received: {}", key)
//                 }
//                 Ok(_) => {}
//                 Err(TryRecvError::Empty) => println!("Channel empty"),
//                 Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
//             }

//             let message = self.wait_for_next_message()?;

//             match message {
//                 ServerToClientMessage::PlayerJoined { players } => {
//                     println!("Players: {}", players.join(", "));
//                     // todo
//                 }
//                 ServerToClientMessage::GameStarted {
//                     player_index,
//                     game_state,
//                 } => {
//                     return Ok(game_state);
//                 }
//                 ServerToClientMessage::UpdatedGameState(game_state) => {
//                     return Ok(game_state);
//                 }
//                 _ => {}
//             }

//             sleep(1000);
//         }
//     }
// }

fn spawn_stdin_channel(tx: Sender<Messages>) {
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .expect("Failed to read stdin");
        if buffer == "start\n" {
            tx.send(Messages::UserInput(buffer)).expect("channel error");
            break;
        }
    });
}

fn spawn_server_connection(
    host: String,
    tx: Sender<Messages>,
) -> anyhow::Result<Writer<TcpStream>> {
    let client = ClientBuilder::new(&host)?.connect_insecure()?;
    let (mut socket_reader, socket_writer) = client.split()?;
    thread::spawn(move || loop {
        for message in socket_reader.incoming_messages() {
            match message {
                Ok(OwnedMessage::Text(text)) => {
                    let message: ServerToClientMessage =
                        serde_json::from_str(&text).expect("failed to parse json");
                    tx.send(Messages::ServerMessage(message))
                        .expect("channel error");
                }
                _ => {}
            }
        }
    });

    Ok(socket_writer)
}

fn sleep(millis: u64) {
    let duration = time::Duration::from_millis(millis);
    thread::sleep(duration);
}

fn main() -> BoxedResult<()> {
    let args: Vec<String> = env::args().collect();

    let (tx, rx) = mpsc::channel::<Messages>();

    let mut server = match args.as_slice() {
        [_, host, player_name, session_id] => {
            println!(
                "Connect to {} (session: {}) as {}...",
                host, session_id, player_name
            );
            let mut server = spawn_server_connection(host.clone(), tx.clone())?;
            server.send_message(&OwnedMessage::Text(serde_json::to_string(
                &ClientToServerMessage::Join {
                    player_name: player_name.clone(),
                    session_id: session_id.clone(),
                },
            )?))?;
            server
        }
        _ => {
            println!("Usage: hanabi [host] [username] [session_id]");
            println!("Error: Invalid format {:?}", args);
            exit(1);
        }
    };

    let mut terminal = setup_terminal()?;

    // println!("Connected! Waiting for players... press 's' to start the game!");

    // loop {
    //     let message = rx.recv();

    //     match message {
    //         Ok(Messages::ServerMessage(ServerToClientMessage::GameStarted {
    //             game_state, ..
    //         })) => {
    //             initial_game_state = Some(game_state);
    //             break;
    //         }
    //         Ok(Messages::ServerMessage(ServerToClientMessage::PlayerJoined { players })) => {
    //             println!("Current players: {}", players.join(", "));
    //         }
    //         Ok(Messages::UserInput(msg)) => {
    //             if msg == "start\n" {
    //                 server.send_message(&OwnedMessage::Text(serde_json::to_string(
    //                     &ClientToServerMessage::StartGame,
    //                 )?))?;
    //             }
    //         }
    //         Err(_) => todo!(),
    //         _ => {}
    //     }
    // }

    let mut current_player = PlayerIndex(0);
    let initial_game_state = GameStateSnapshot {
        player_snapshot: PlayerIndex(0),
        draw_pile_count: 0,
        played_cards: vec![],
        discard_pile: vec![],
        players: vec![ClientPlayerView::Me { hand: vec![] }],
        remaining_bomb_count: 0,
        remaining_hint_count: 0,
        turn: PlayerIndex(0),
        num_rounds: 0,
        last_turn: None,
        outcome: None,
    };

    let mut app = HanabiApp::new(initial_game_state);

    loop {
        app.run(&mut terminal)?;
        let result = app.handle_events()?;

        match result {
            hanabi_app::EventHandlerResult::PlayerAction(action) => {
                server.send_message(&OwnedMessage::Text(serde_json::to_string(
                    &ClientToServerMessage::PlayerAction {
                        player_index: current_player,
                        action,
                    },
                )?))?;
            }
            hanabi_app::EventHandlerResult::Start => {
                server.send_message(&OwnedMessage::Text(serde_json::to_string(
                    &ClientToServerMessage::StartGame,
                )?))?;
            }
            hanabi_app::EventHandlerResult::Quit => {
                break;
            }
            hanabi_app::EventHandlerResult::Continue => {}
        }

        let message = rx.try_recv();

        match message {
            Ok(Messages::ServerMessage(ServerToClientMessage::GameStarted {
                game_state,
                player_index,
            })) => {
                current_player = player_index;
                app.update(game_state);
            }
            Ok(Messages::ServerMessage(ServerToClientMessage::UpdatedGameState(game_state))) => {
                app.update(game_state);
            }
            Ok(Messages::ServerMessage(ServerToClientMessage::PlayerJoined { players })) => {

                // println!("Current players: {}", players.join(", "));
            }
            Ok(Messages::UserInput(msg)) => {}
            Err(_) => {}
            _ => {}
        }
    }

    // let result: Result<(), Box<dyn Error>> = app.run(&mut terminal);
    restore_terminal(terminal)?;

    // if let Err(err) = result {
    //     eprintln!("{err:?}  ");
    // }
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
