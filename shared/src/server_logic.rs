// use std::collections::HashMap;
// use std::sync::mpsc;
// use std::sync::Arc;
// use std::sync::Mutex;
// use std::thread;

// use shared::client_logic::ClientToServerMessage;
// use shared::client_logic::GameLog;
// use shared::client_logic::ServerToClientMessage;
// use shared::model::GameConfig;
// use shared::model::PlayerIndex;

// #[derive(Debug, Clone)]
// struct SocketClient {
//     id: i32,
//     sender: Option<mpsc::Sender<OwnedMessage>>,
// }

// #[derive(Debug, Clone)]
// struct SocketPlayer {
//     name: String,
//     socket_id: i32,
// }

// #[derive(Debug, Clone)]
// enum GameLobbyStatus {
//     Waiting,
//     Playing(GameLog),
// }

// #[derive(Debug, Clone)]
// struct GameLobby {
//     players: Vec<SocketPlayer>,
//     status: GameLobbyStatus,
// }

// #[derive(Debug, Clone)]
// struct SocketMessage<T> {
//     message: T,
//     socket_id: i32,
// }

// pub struct LobbyServer {
//     game_lobby_map: HashMap<String, GameLobby>,
//     // socket_clients: HashMap<i32, SocketClient>,
// }

// pub impl LobbyServer {
//     pub fn new() -> Self {
//         LobbyServer {
//             game_lobby_map: HashMap::new(),
//             // socket_clients: HashMap::new(),
//         }
//     }
// }

//     pub fn message_received(message_from_client: SocketMessage) {
//         match message_from_client {
//             SocketMessage { message, socket_id } => {
//                 let mut game_lobby_map = game_lobby_map.lock().unwrap();

//                 match message {
//                     ClientToServerMessage::Join {
//                         player_name,
//                         session_id,
//                     } => {
//                         let game_lobby = game_lobby_map.get_mut(&session_id);
//                         if let Some(game_lobby) = game_lobby {
//                             println!("Player {} joined {:?}", player_name, game_lobby);

//                             game_lobby.players.push(SocketPlayer {
//                                 name: player_name.clone(),
//                                 socket_id: socket_id.clone(),
//                             });

//                             let messages = game_lobby.players.iter().map(|p| SocketMessage {
//                                 message: ServerToClientMessage::PlayerJoined {
//                                     players: game_lobby
//                                         .players
//                                         .iter()
//                                         .map(|p| p.name.clone())
//                                         .collect(),
//                                 },
//                                 socket_id: p.socket_id,
//                             });

//                             for message in messages.into_iter() {
//                                 println!("Sending Joined {:?}", message);
//                                 broadcast_something.send(message).expect("channel error");
//                             }
//                         } else {
//                             println!("Player {} joined new lobby {}", player_name, session_id);
//                             let players = vec![SocketPlayer {
//                                 name: player_name.clone(),
//                                 socket_id: socket_id,
//                             }];

//                             game_lobby_map.insert(
//                                 session_id,
//                                 GameLobby {
//                                     players: players.clone(),
//                                     status: GameLobbyStatus::Waiting,
//                                 },
//                             );
//                         }
//                     }
//                     ClientToServerMessage::StartGame => {
//                         let game_lobby = game_lobby_map.values_mut().find(|game_log| {
//                             game_log
//                                 .players
//                                 .iter()
//                                 .find(|p| p.socket_id == socket_id)
//                                 .is_some()
//                         });

//                         if let Some(game_lobby) = game_lobby {
//                             let game_log = GameLog::new(GameConfig {
//                                 num_players: game_lobby.players.len(),
//                                 hand_size: 4,
//                                 num_fuses: 3,
//                                 num_hints: 8,
//                                 starting_player: PlayerIndex(0),
//                                 seed: 0,
//                             });

//                             let game_state = game_log.current_game_state();
//                             game_lobby.status = GameLobbyStatus::Playing(game_log);

//                             let messages =
//                                 game_lobby.players.iter().enumerate().map(|(index, p)| {
//                                     SocketMessage {
//                                         message: ServerToClientMessage::GameStarted {
//                                             player_index: PlayerIndex(index),
//                                             game_state: game_state
//                                                 .clone()
//                                                 .into_client_game_state(PlayerIndex(index)),
//                                         },
//                                         socket_id: p.socket_id,
//                                     }
//                                 });
//                             for message in messages.into_iter() {
//                                 broadcast_something.send(message).expect("Channel error");
//                             }
//                         }
//                     }

//                     ClientToServerMessage::PlayerAction { action, .. } => {
//                         let game_lobby = game_lobby_map.values_mut().find(|game_log| {
//                             game_log
//                                 .players
//                                 .iter()
//                                 .find(|p| p.socket_id == socket_id)
//                                 .is_some()
//                         });

//                         if let Some(GameLobby {
//                             players,
//                             status: GameLobbyStatus::Playing(game_log),
//                         }) = game_lobby
//                         {
//                             let new_game_state = game_log.log(action);

//                             if let Ok(game_state) = new_game_state {
//                                 let messages =
//                                     players.iter().enumerate().map(|(index, p)| SocketMessage {
//                                         message: ServerToClientMessage::UpdatedGameState(
//                                             game_state
//                                                 .clone()
//                                                 .into_client_game_state(PlayerIndex(index)),
//                                         ),
//                                         socket_id: p.socket_id,
//                                     });
//                                 for message in messages.into_iter() {
//                                     broadcast_something.send(message).expect("channel error");
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

// fn main() {
//     println!("{}", "Hanabi Simulator v0.1.0");
//     // let num_players: usize = 5;

//     // We use this channel to broadcast a message (from the server) to all the clients.
//     // If you want to broadcast something, server_to_client_sender.send(...)
//     let (broadcast_something, listener_for_things_to_broadcast) =
//         mpsc::channel::<SocketMessage<ServerToClientMessage>>();

//     let (got_message_from_client, listener_for_messages_from_client) =
//         mpsc::channel::<SocketMessage<ClientToServerMessage>>();

//     let game_lobby_map: Arc<Mutex<HashMap<String, GameLobby>>> =
//         Arc::new(Mutex::new(HashMap::new()));

//     let socket_clients: Arc<Mutex<HashMap<i32, SocketClient>>> =
//         Arc::new(Mutex::new(HashMap::new()));

//     let broadcasting_handler = {
//         // Automatic reference counting
//         let socket_clients = Arc::clone(&socket_clients);
//         thread::spawn(move || loop {
//             println!("Listening for broadcasts");
//             let message_to_broadcast = listener_for_things_to_broadcast.recv().unwrap();
//             println!("Broadcasting {:?}", message_to_broadcast);

//             let message_text = serde_json::to_string(&message_to_broadcast.message)
//                 .expect("Failed to parse the message");

//             {
//                 let socket_clients = socket_clients.lock().unwrap();

//                 if let Some(socket_client) = socket_clients.get(&message_to_broadcast.socket_id) {
//                     if let Some(socket_client_sender) = &socket_client.sender {
//                         println!(
//                             "Sending message to {}: {}",
//                             message_to_broadcast.socket_id, message_text
//                         );
//                         let message = OwnedMessage::Text(message_text.clone());
//                         let result = socket_client_sender
//                             .send(message)
//                             .expect("Failed to broadcast");
//                     }
//                 }
//             }
//         })
//     };

//     let message_from_client_handler = {
//         let game_lobby_map = Arc::clone(&game_lobby_map);

//         thread::spawn(move || loop {
//             let message_from_client = listener_for_messages_from_client.recv().unwrap();
//             println!("Handling {:?}", message_from_client);

//             match message_from_client {
//                 SocketMessage { message, socket_id } => {
//                     let mut game_lobby_map = game_lobby_map.lock().unwrap();

//                     match message {
//                         ClientToServerMessage::Join {
//                             player_name,
//                             session_id,
//                         } => {
//                             let game_lobby = game_lobby_map.get_mut(&session_id);
//                             if let Some(game_lobby) = game_lobby {
//                                 println!("Player {} joined {:?}", player_name, game_lobby);

//                                 game_lobby.players.push(SocketPlayer {
//                                     name: player_name.clone(),
//                                     socket_id: socket_id.clone(),
//                                 });

//                                 let messages = game_lobby.players.iter().map(|p| SocketMessage {
//                                     message: ServerToClientMessage::PlayerJoined {
//                                         players: game_lobby
//                                             .players
//                                             .iter()
//                                             .map(|p| p.name.clone())
//                                             .collect(),
//                                     },
//                                     socket_id: p.socket_id,
//                                 });

//                                 for message in messages.into_iter() {
//                                     println!("Sending Joined {:?}", message);
//                                     broadcast_something.send(message).expect("channel error");
//                                 }
//                             } else {
//                                 println!("Player {} joined new lobby {}", player_name, session_id);
//                                 let players = vec![SocketPlayer {
//                                     name: player_name.clone(),
//                                     socket_id: socket_id,
//                                 }];

//                                 game_lobby_map.insert(
//                                     session_id,
//                                     GameLobby {
//                                         players: players.clone(),
//                                         status: GameLobbyStatus::Waiting,
//                                     },
//                                 );
//                             }
//                         }
//                         ClientToServerMessage::StartGame => {
//                             let game_lobby = game_lobby_map.values_mut().find(|game_log| {
//                                 game_log
//                                     .players
//                                     .iter()
//                                     .find(|p| p.socket_id == socket_id)
//                                     .is_some()
//                             });

//                             if let Some(game_lobby) = game_lobby {
//                                 let game_log = GameLog::new(GameConfig {
//                                     num_players: game_lobby.players.len(),
//                                     hand_size: 4,
//                                     num_fuses: 3,
//                                     num_hints: 8,
//                                     starting_player: PlayerIndex(0),
//                                     seed: 0,
//                                 });

//                                 let game_state = game_log.current_game_state();
//                                 game_lobby.status = GameLobbyStatus::Playing(game_log);

//                                 let messages =
//                                     game_lobby.players.iter().enumerate().map(|(index, p)| {
//                                         SocketMessage {
//                                             message: ServerToClientMessage::GameStarted {
//                                                 player_index: PlayerIndex(index),
//                                                 game_state: game_state
//                                                     .clone()
//                                                     .into_client_game_state(PlayerIndex(index)),
//                                             },
//                                             socket_id: p.socket_id,
//                                         }
//                                     });
//                                 for message in messages.into_iter() {
//                                     broadcast_something.send(message).expect("Channel error");
//                                 }
//                             }
//                         }

//                         ClientToServerMessage::PlayerAction { action, .. } => {
//                             let game_lobby = game_lobby_map.values_mut().find(|game_log| {
//                                 game_log
//                                     .players
//                                     .iter()
//                                     .find(|p| p.socket_id == socket_id)
//                                     .is_some()
//                             });

//                             if let Some(GameLobby {
//                                 players,
//                                 status: GameLobbyStatus::Playing(game_log),
//                             }) = game_lobby
//                             {
//                                 let new_game_state = game_log.log(action);

//                                 if let Ok(game_state) = new_game_state {
//                                     let messages = players.iter().enumerate().map(|(index, p)| {
//                                         SocketMessage {
//                                             message: ServerToClientMessage::UpdatedGameState(
//                                                 game_state
//                                                     .clone()
//                                                     .into_client_game_state(PlayerIndex(index)),
//                                             ),
//                                             socket_id: p.socket_id,
//                                         }
//                                     });
//                                     for message in messages.into_iter() {
//                                         broadcast_something.send(message).expect("channel error");
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }

//             println!("new state: {:?}", game_lobby_map);
//         })
//     };

//     let socket_connection_manager_handler = {
//         let socket_clients = Arc::clone(&socket_clients);
//         let got_message_from_client = got_message_from_client.clone();

//         // Listening for connections thread
//         thread::spawn(move || {
//             let addr = "127.0.0.1:7879";

//             println!("Listening for Socket requests on {}", addr);

//             let socket_listener = websocket::sync::Server::bind(addr).unwrap();
//             let mut socket_counter = 0;

//             for connection in socket_listener.filter_map(Result::ok) {
//                 let socket_clients = Arc::clone(&socket_clients);

//                 let socket_id = socket_counter;
//                 socket_counter = socket_counter + 1;

//                 let got_message_from_client = got_message_from_client.clone();

//                 // Got a connection thread
//                 thread::spawn(move || {
//                     let ws_client = connection.accept().unwrap();
//                     let (socket_channel_for_writing, socket_channel_handler_for_writing) =
//                         mpsc::channel();

//                     println!("New socket connection {}", socket_id);

//                     let socket_client = SocketClient {
//                         id: socket_id,
//                         sender: Some(socket_channel_for_writing.clone()),
//                     };

//                     // Limit locking as much as possible.
//                     {
//                         let mut socket_clients = socket_clients.lock().unwrap();
//                         socket_clients.insert(socket_id, socket_client);
//                     }

//                     let (mut socket_stream_read, mut socket_stream_write) =
//                         ws_client.split().unwrap();

//                     // Thread for listening on sockets
//                     {
//                         thread::spawn(move || {
//                             println!("Socket channel {} listening", socket_id);
//                             while let Ok(message) = socket_channel_handler_for_writing.recv() {
//                                 println!(
//                                     "Socket channel {} received message: {:?}",
//                                     socket_id, message
//                                 );
//                                 let result = socket_stream_write.send_message(&message);

//                                 match result {
//                                     Ok(_) => {
//                                         println!("Sent message through socket: {:?}", message)
//                                     }
//                                     Err(error) => {
//                                         println!("Error sending message to client {}", error)
//                                     }
//                                 }
//                             }
//                         });
//                     }

//                     let socket_channel_sender = socket_channel_for_writing.clone();

//                     println!("Socket TCP {} listening", socket_id);
//                     for from_client_message in socket_stream_read.incoming_messages() {
//                         let message = from_client_message.unwrap();
//                         println!("Socket TCP {} received message: {:?}", socket_id, message);

//                         let result = match message {
//                             OwnedMessage::Close(_) => {
//                                 let message = OwnedMessage::Close(None);
//                                 socket_channel_sender.send(message)
//                             }
//                             OwnedMessage::Ping(ping) => {
//                                 let message = OwnedMessage::Pong(ping);
//                                 socket_channel_sender.send(message)
//                             }
//                             OwnedMessage::Text(text) => {
//                                 let message: ClientToServerMessage =
//                                     serde_json::from_str(&text).expect("Error parsing message");
//                                 got_message_from_client
//                                     .send(SocketMessage { message, socket_id })
//                                     .expect("Channel issue");
//                                 Ok(())
//                             }
//                             _ => socket_channel_sender.send(message),
//                         };

//                         match result {
//                             Err(err) => {
//                                 println!("Error from client: {}", err);
//                                 break;
//                             }
//                             _ => {}
//                         }
//                     }

//                     {
//                         println!("Client {} disconnected", socket_id);
//                         let mut socket_clients = socket_clients.lock().unwrap();
//                         socket_clients.remove(&socket_id);
//                     }
//                 });
//             }
//         })
//     };

//     message_from_client_handler.join().unwrap();
//     broadcasting_handler.join().unwrap();
//     socket_connection_manager_handler.join().unwrap();
// }
