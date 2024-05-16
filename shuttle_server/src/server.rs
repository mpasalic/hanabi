use std::collections::HashMap;

use shared::client_logic::*;
use shared::model::GameConfig;
use shared::model::PlayerIndex;
use std::hash::{Hash, Hasher};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct LobbyClient {
    pub client_id: ClientId,
    pub sender: mpsc::UnboundedSender<ServerToClientMessage>,
}

impl PartialEq for LobbyClient {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id
    }
}

#[derive(Debug, Clone)]
enum GameLobbyStatus {
    Waiting,
    Playing(GameLog),
}

#[derive(Debug, Clone)]
struct GameLobby {
    players: Vec<SocketPlayer>,
    status: GameLobbyStatus,
    log: Vec<String>,
}

impl GameLobby {
    fn new(players: Vec<SocketPlayer>) -> Self {
        GameLobby {
            players: players,
            status: GameLobbyStatus::Waiting,
            log: vec![],
        }
    }

    fn update_players(&self) {
        let players: Vec<OnlinePlayer> = self
            .players
            .iter()
            .map(|p| OnlinePlayer {
                name: p.name.clone(),
                connection_status: match p.connection {
                    ConnectionState::Connected(_) => ConnectionStatus::Connected,
                    ConnectionState::Disconnected => ConnectionStatus::Disconnected,
                },
                is_host: false,
            })
            .collect();

        for (index, p) in self.players.iter().enumerate() {
            p.send(ServerToClientMessage::UpdatedGameState(
                match &self.status {
                    GameLobbyStatus::Waiting => HanabiGame::Lobby {
                        log: self.log.clone(),
                        players: players.clone(),
                    },
                    GameLobbyStatus::Playing(game_log) => HanabiGame::Started {
                        players: players.clone(),
                        game_state: game_log.into_client_game_state(PlayerIndex(index)),
                    },
                },
            ));
        }
    }

    // fn get_client(&self, client_id: ClientId) -> Option<&SocketPlayer> {
    //     self.players.iter().find(|p| match p {
    //         SocketPlayer {
    //             connection: ConnectionState::Connected(LobbyClient { client_id: id, .. }),
    //             ..
    //         } => *id == client_id,
    //         _ => false,
    //     })
    // }

    fn get_mut_client(&mut self, client_id: ClientId) -> Option<&mut SocketPlayer> {
        self.players.iter_mut().find(|p| match p {
            SocketPlayer {
                connection: ConnectionState::Connected(LobbyClient { client_id: id, .. }),
                ..
            } => *id == client_id,
            _ => false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub struct ClientId(pub usize);

// #[derive(Debug, Clone, Eq, PartialEq, Hash)]
// struct PlayerName(String);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct SessionId(String);

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connected(LobbyClient),
    Disconnected,
}

#[derive(Debug, Clone)]
struct SocketPlayer {
    name: String,
    connection: ConnectionState,
}

impl SocketPlayer {
    fn send(&self, message: ServerToClientMessage) {
        if let ConnectionState::Connected(LobbyClient { sender, .. }) = &self.connection {
            // Doesn't matter if this fails, when the client reconnects they will get the updated game state.
            let _ = sender.send(message);
        }
    }
}

impl Hash for SocketPlayer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }

    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state)
        }
    }
}

pub struct LobbyServer {
    game_lobbies: HashMap<SessionId, GameLobby>,
}

#[derive(Debug)]
pub enum LobbyError {
    InvalidState(String),
    InvalidPlayerAction(String),
}

impl LobbyServer {
    pub fn new() -> Self {
        LobbyServer {
            game_lobbies: HashMap::new(),
        }
    }

    fn get_lobby_for_client(&mut self, client: ClientId) -> Option<&mut GameLobby> {
        self.game_lobbies.values_mut().find(|lobby| {
            lobby
                .players
                .iter()
                .find(|p| match p {
                    SocketPlayer {
                        connection: ConnectionState::Connected(LobbyClient { client_id: id, .. }),
                        ..
                    } => *id == client,
                    _ => false,
                })
                .is_some()
        })
    }

    pub fn disconnected(&mut self, client_id: ClientId) {
        let game_lobby = self.get_lobby_for_client(client_id);

        if let Some(game_lobby) = game_lobby {
            let player = game_lobby.get_mut_client(client_id);

            if let Some(player) = player {
                player.connection = ConnectionState::Disconnected;
            }
        }
    }

    pub fn message_received(
        &mut self,
        client: &LobbyClient,
        message: ClientToServerMessage,
    ) -> Result<(), LobbyError> {
        match message {
            ClientToServerMessage::Join {
                player_name,
                session_id,
            } => {
                let game_lobby = self
                    .game_lobbies
                    .entry(SessionId(session_id))
                    .or_insert(GameLobby::new(vec![]));

                let existing_player = game_lobby
                    .players
                    .iter_mut()
                    .find(|p| p.name == player_name);

                match existing_player {
                    Some(SocketPlayer { connection, .. }) => {
                        *connection = ConnectionState::Connected(client.clone());
                        game_lobby.log.push(format!("{} reconnected", player_name));
                    }
                    None => {
                        game_lobby.players.push(SocketPlayer {
                            name: player_name.clone(),
                            connection: ConnectionState::Connected(client.clone()),
                        });
                        game_lobby.log.push(format!("{} joined", player_name));
                    }
                }

                game_lobby.update_players();
            }
            ClientToServerMessage::StartGame => {
                let game_lobby = self.get_lobby_for_client(client.client_id);

                match game_lobby {
                    Some(GameLobby {
                        players,
                        status: status @ GameLobbyStatus::Waiting,
                        ..
                    }) => {
                        let new_game = GameLog::new(GameConfig {
                            num_players: players.len(),
                            hand_size: 4,
                            num_fuses: 3,
                            num_hints: 8,
                            starting_player: PlayerIndex(0),
                            seed: 0,
                        });
                        *status = GameLobbyStatus::Playing(new_game);
                    }
                    Some(GameLobby {
                        status: GameLobbyStatus::Playing(_),
                        ..
                    }) => {
                        return Err(LobbyError::InvalidState(
                            "Game is already in playing state".to_string(),
                        ));
                    }
                    None => {
                        return Err(LobbyError::InvalidState(
                            "Game is not in waiting state".to_string(),
                        ));
                    }
                };

                if let Some(game_lobby) = game_lobby {
                    game_lobby.update_players();
                }
            }

            ClientToServerMessage::PlayerAction { action, .. } => {
                if let Some(game_lobby) = self.get_lobby_for_client(client.client_id) {
                    if let GameLobbyStatus::Playing(ref mut game_log) = game_lobby.status {
                        game_log
                            .log(action)
                            .map_err(|e| LobbyError::InvalidPlayerAction(e))?;

                        game_lobby.update_players();
                    }
                }
            }
        }
        Ok(())
    }
}
