use std::collections::HashMap;

use shared::client_logic::ClientToServerMessage;
use shared::client_logic::GameLog;
use shared::client_logic::ServerToClientMessage;
use shared::model::GameConfig;
use shared::model::PlayerIndex;

#[derive(Debug, Clone)]
enum GameLobbyStatus {
    Waiting,
    Playing(GameLog),
}

#[derive(Debug, Clone)]
struct GameLobby {
    players: Vec<SocketPlayer>,
    status: GameLobbyStatus,
}

pub struct LobbyServer {
    game_lobby_map: HashMap<String, GameLobby>,
    // socket_clients: HashMap<i32, SocketClient>,
}

#[derive(Debug, Clone)]
struct SocketPlayer {
    name: String,
    socket_id: usize,
}

#[derive(Debug, Clone)]
pub struct SocketMessage<T> {
    pub message: T,
    pub socket_id: usize,
}

impl LobbyServer {
    pub fn new() -> Self {
        LobbyServer {
            game_lobby_map: HashMap::new(),
            // socket_clients: HashMap::new(),
        }
    }

    pub fn message_received(
        &mut self,
        socket_id: usize,
        message: ClientToServerMessage,
    ) -> Vec<SocketMessage<ServerToClientMessage>> {
        match message {
            ClientToServerMessage::Join {
                player_name,
                session_id,
            } => {
                let game_lobby = self.game_lobby_map.get_mut(&session_id);
                if let Some(game_lobby) = game_lobby {
                    println!("Player {} joined {:?}", player_name, game_lobby);

                    game_lobby.players.push(SocketPlayer {
                        name: player_name.clone(),
                        socket_id: socket_id.clone(),
                    });

                    let messages = game_lobby.players.iter().map(|p| SocketMessage {
                        message: ServerToClientMessage::PlayerJoined {
                            players: game_lobby.players.iter().map(|p| p.name.clone()).collect(),
                        },
                        socket_id: p.socket_id,
                    });

                    return messages.collect();
                } else {
                    println!("Player {} joined new lobby {}", player_name, session_id);
                    let players = vec![SocketPlayer {
                        name: player_name.clone(),
                        socket_id: socket_id,
                    }];

                    self.game_lobby_map.insert(
                        session_id,
                        GameLobby {
                            players: players.clone(),
                            status: GameLobbyStatus::Waiting,
                        },
                    );

                    let messages = SocketMessage {
                        message: ServerToClientMessage::PlayerJoined {
                            players: vec![player_name.clone()],
                        },
                        socket_id: socket_id,
                    };

                    return vec![messages];
                }
            }
            ClientToServerMessage::StartGame => {
                let game_lobby = self.game_lobby_map.values_mut().find(|game_log| {
                    game_log
                        .players
                        .iter()
                        .find(|p| p.socket_id == socket_id)
                        .is_some()
                });

                if let Some(GameLobby { players, status }) = game_lobby {
                    let game_log = GameLog::new(GameConfig {
                        num_players: players.len(),
                        hand_size: 4,
                        num_fuses: 3,
                        num_hints: 8,
                        starting_player: PlayerIndex(0),
                        seed: 0,
                    });
                    *status = GameLobbyStatus::Playing(game_log.clone());

                    let messages = players.iter().enumerate().map(|(index, p)| SocketMessage {
                        message: ServerToClientMessage::GameStarted {
                            player_index: PlayerIndex(index),
                            game_state: game_log.into_client_game_state(PlayerIndex(index)),
                        },
                        socket_id: p.socket_id,
                    });

                    return messages.collect();
                } else {
                    return vec![];
                }
            }

            ClientToServerMessage::PlayerAction { action, .. } => {
                let game_lobby = self.game_lobby_map.values_mut().find(|game_log| {
                    game_log
                        .players
                        .iter()
                        .find(|p| p.socket_id == socket_id)
                        .is_some()
                });

                if let Some(GameLobby {
                    players,
                    status: GameLobbyStatus::Playing(game_log),
                }) = game_lobby
                {
                    let new_game_state = game_log.log(action);

                    if let Ok(_) = new_game_state {
                        let messages = players.iter().enumerate().map(|(index, p)| SocketMessage {
                            message: ServerToClientMessage::UpdatedGameState(
                                game_log.into_client_game_state(PlayerIndex(index)),
                            ),
                            socket_id: p.socket_id,
                        });
                        return messages.collect();
                    } else {
                        return vec![];
                    }
                }
            }
        }
        return vec![];
    }
}
