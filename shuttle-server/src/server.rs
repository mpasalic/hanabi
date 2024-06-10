use std::collections::hash_map::Entry;
use std::collections::HashMap;

use rand::rngs::StdRng;
use shared::client_logic::*;
use shared::model::GameConfig;
use shared::model::PlayerIndex;
use sqlx::PgPool;
use std::hash::{Hash, Hasher};
use tokio::sync::mpsc;

use crate::model::create_game;
use crate::model::generate_unique_game_id;
use crate::model::get_game_actions;
use crate::model::get_game_config;
use crate::model::get_players;
use crate::model::save_action;

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
    Ended(GameLog),
}

#[derive(Debug, Clone)]
struct GameLobby {
    session_id: SessionId,
    players: Vec<SocketPlayer>,
    status: GameLobbyStatus,
    log: Vec<String>,
}

impl GameLobby {
    fn new(session: SessionId, players: Vec<SocketPlayer>) -> Self {
        GameLobby {
            session_id: session,
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
                        session_id: self.session_id.0.clone(),
                        log: self.log.clone(),
                        players: players.clone(),
                    },
                    GameLobbyStatus::Playing(game_log) => HanabiGame::Started {
                        session_id: self.session_id.0.clone(),
                        players: players.clone(),
                        game_state: game_log.into_client_game_state(
                            game_log.current_game_state(),
                            PlayerIndex(index),
                            self.players.iter().map(|p| p.name.clone()).collect(),
                        ),
                        log: game_log
                            .into_client_game_log(
                                PlayerIndex(index),
                                self.players.iter().map(|p| p.name.clone()).collect(),
                            )
                            .clone(),
                    },
                    GameLobbyStatus::Ended(game_log) => HanabiGame::Ended {
                        session_id: self.session_id.0.clone(),
                        players: players.clone(),
                        game_state: game_log.into_client_game_state(
                            game_log.current_game_state(),
                            PlayerIndex(index),
                            self.players.iter().map(|p| p.name.clone()).collect(),
                        ),
                        revealed_game_log: game_log.clone(),
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
    pool: PgPool,
}

#[derive(Debug)]
pub enum LobbyError {
    InvalidState(String),
    InvalidPlayerAction(String),
    SqlError(sqlx::Error),
}

impl From<sqlx::Error> for LobbyError {
    fn from(e: sqlx::Error) -> Self {
        LobbyError::SqlError(e)
    }
}

// pub enum Result {
//     GameCreated(GameConfig, Vec<String>),
//     PlayedAction(PlayerAction, PlayerIndex, TurnCount),
// }

impl LobbyServer {
    pub fn new(pool: PgPool) -> Self {
        LobbyServer {
            game_lobbies: HashMap::new(),
            pool,
        }
    }

    pub async fn hydrate(&mut self, game_id: &String) -> Result<(), LobbyError> {
        let game_config = get_game_config(&self.pool, game_id.clone()).await?;

        let game_actions = get_game_actions(&self.pool, game_id.clone()).await?;

        let players = get_players(&self.pool, game_id.clone()).await?;

        let mut game_log = GameLog::new::<StdRng>(game_config.clone());

        for action in game_actions {
            game_log
                .log(action)
                .map_err(|e| LobbyError::InvalidState(e))?;
        }

        let current_state = game_log.current_game_state();

        let game_lobby = GameLobby {
            session_id: SessionId(game_id.clone()),
            players: players
                .iter()
                .map(|p| SocketPlayer {
                    name: p.clone(),
                    connection: ConnectionState::Disconnected,
                })
                .collect(),
            status: match current_state.outcome {
                Some(_) => GameLobbyStatus::Ended(game_log),
                None => GameLobbyStatus::Playing(game_log),
            },
            log: vec![],
        };

        self.game_lobbies
            .insert(SessionId(game_id.clone()), game_lobby);

        Ok(())
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

    fn get_lobby_session_for_client(&self, client: ClientId) -> Option<SessionId> {
        let lobby = self.game_lobbies.values().find(|lobby| {
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
        });

        lobby.and_then(|l| Some(l.session_id.clone()))
    }

    pub fn disconnected(&mut self, client_id: ClientId) {
        let game_lobby_session = self.get_lobby_session_for_client(client_id);

        if let Some(game_lobby_session) = game_lobby_session.clone() {
            let game_lobby = self
                .game_lobbies
                .entry(game_lobby_session.clone())
                .and_modify(|game_lobby| {
                    let player = game_lobby.get_mut_client(client_id);

                    if let Some(player) = player {
                        player.connection = ConnectionState::Disconnected;
                    }
                    game_lobby.update_players();
                });
        }
    }

    pub async fn message_received(
        &mut self,
        client: &LobbyClient,
        message: ClientToServerMessage,
    ) -> Result<(), LobbyError> {
        match message {
            ClientToServerMessage::CreateGame { player_name } => {
                let session_id = generate_unique_game_id(&self.pool).await?;

                let game_lobby = self
                    .game_lobbies
                    .entry(SessionId(session_id.clone()))
                    .or_insert(GameLobby::new(
                        SessionId(session_id.clone()),
                        vec![SocketPlayer {
                            name: player_name.clone(),
                            connection: ConnectionState::Connected(client.clone()),
                        }],
                    ));
                let _ = client.sender.send(ServerToClientMessage::CreatedGame {
                    session_id: session_id.clone(),
                });
            }
            ClientToServerMessage::Join {
                player_name,
                session_id,
            } => {
                if !self
                    .game_lobbies
                    .contains_key(&SessionId(session_id.clone()))
                {
                    // Should prob have better logic here
                    // This will be simpler when we have an actual "Create Game" message
                    let result = self.hydrate(&session_id.clone()).await;

                    match result {
                        Ok(_) => {
                            println!("Hydrated game");
                        }
                        Err(e) => {
                            println!("Error hydrating game: {:?}", e);
                        }
                    }
                }

                let game_lobby = self
                    .game_lobbies
                    .entry(SessionId(session_id.clone()))
                    .or_insert(GameLobby::new(SessionId(session_id.clone()), vec![]));

                let existing_player = game_lobby
                    .players
                    .iter_mut()
                    .find(|p| p.name == player_name);

                match (existing_player, &game_lobby.status) {
                    (Some(SocketPlayer { connection, .. }), _) => {
                        *connection = ConnectionState::Connected(client.clone());
                        game_lobby.log.push(format!("{} reconnected", player_name));
                    }
                    (None, GameLobbyStatus::Waiting) => {
                        game_lobby.players.push(SocketPlayer {
                            name: player_name.clone(),
                            connection: ConnectionState::Connected(client.clone()),
                        });
                        game_lobby.log.push(format!("{} joined", player_name));
                    }
                    (None, _) => {
                        return Err(LobbyError::InvalidState(
                            "Game is already in progress".to_string(),
                        ));
                    }
                }

                game_lobby.update_players();
            }
            ClientToServerMessage::StartGame => {
                let session_id = self.get_lobby_session_for_client(client.client_id);

                if let Some(session_id) = session_id {
                    let game_lobby =
                        self.game_lobbies
                            .entry(session_id.clone())
                            .and_modify(|game_lobby| {
                                let num_players = game_lobby.players.len();
                                let config = GameConfig {
                                    num_players: num_players,
                                    hand_size: match num_players {
                                        2 | 3 => 5,
                                        4 | 5 => 4,
                                        _ => 4, // error?
                                    },
                                    num_fuses: 3,
                                    num_hints: 8,
                                    starting_player: PlayerIndex(0),
                                    seed: rand::random::<u64>(),
                                };
                                game_lobby.status = GameLobbyStatus::Playing(
                                    GameLog::new::<StdRng>(config.clone()),
                                );
                            });

                    match game_lobby {
                        Entry::Occupied(game_lobby_entry) => {
                            let game_lobby = game_lobby_entry.get();

                            match game_lobby {
                                GameLobby {
                                    session_id: SessionId(session_id),
                                    players,
                                    status: GameLobbyStatus::Playing(game_log),
                                    ..
                                } => {
                                    create_game(
                                        &self.pool,
                                        session_id.clone(),
                                        &game_log.config,
                                        &players
                                            .iter()
                                            .map(|p| p.name.clone())
                                            .collect::<Vec<String>>(),
                                    )
                                    .await
                                    .map_err(|e| LobbyError::InvalidState(e.to_string()))?;
                                }
                                _ => {}
                            }

                            game_lobby.update_players();
                        }
                        Entry::Vacant(_) => {}
                    }
                }

                // let game_lobby = self.get_lobby_for_client(client.client_id);

                // match game_lobby {
                //     Some(GameLobby {
                //         session_id: SessionId(session_id),
                //         players,
                //         status: status @ GameLobbyStatus::Waiting,
                //         ..
                //     }) => {
                //         let num_players = players.len();
                //         let config = GameConfig {
                //             num_players: num_players,
                //             hand_size: match num_players {
                //                 2 | 3 => 5,
                //                 4 | 5 => 4,
                //                 np => {
                //                     return Err(LobbyError::InvalidState(format!(
                //                         "Invalid number of players: {np}"
                //                     )))
                //                 }
                //             },
                //             num_fuses: 3,
                //             num_hints: 8,
                //             starting_player: PlayerIndex(0),
                //             seed: 0,
                //         };
                //         let new_game = GameLog::new(config.clone());
                //         *status = GameLobbyStatus::Playing(new_game);
                //     }
                //     Some(GameLobby {
                //         status: GameLobbyStatus::Playing(_) | GameLobbyStatus::Ended(_),
                //         ..
                //     }) => {
                //         return Err(LobbyError::InvalidState(
                //             "Game is already in playing state".to_string(),
                //         ));
                //     }
                //     None => {
                //         return Err(LobbyError::InvalidState(
                //             "Game is not in waiting state".to_string(),
                //         ));
                //     }
                // };
                // if let Some(game_lobby) = game_lobby {
                //     game_lobby.update_players();
                // }
            }

            ClientToServerMessage::PlayerAction { action, .. } => {
                if let Some(game_lobby) = self.get_lobby_for_client(client.client_id) {
                    if let GameLobbyStatus::Playing(ref mut game_log) = game_lobby.status {
                        let SessionId(session_id) = game_lobby.session_id.clone();
                        let current_game_state = game_log.current_game_state();
                        let turn_index = current_game_state.turn;
                        let PlayerIndex(player_index) = current_game_state.current_player_index();

                        let result = game_log
                            .log(action)
                            .map_err(|e| LobbyError::InvalidPlayerAction(e))?
                            .clone();

                        if let Some(_) = result.outcome {
                            game_lobby.status = GameLobbyStatus::Ended(game_log.clone());
                        }
                        game_lobby.update_players();

                        save_action(&self.pool, &session_id, turn_index, action, player_index)
                            .await
                            .map_err(|e| LobbyError::InvalidState(e.to_string()))?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_seeded_deck() {
        let config = GameConfig {
            num_players: 2,
            hand_size: 5,
            num_fuses: 3,
            num_hints: 8,
            starting_player: PlayerIndex(0),
            seed: 0,
        };

        let deck = GameLog::new::<StdRng>(config.clone());

        let deck_same_seed = GameLog::new::<StdRng>(config.clone());

        assert_eq!(
            deck.current_game_state().draw_pile,
            deck_same_seed.current_game_state().draw_pile
        );
    }

    #[test]
    fn test_different_seeded_deck() {
        let config = GameConfig {
            num_players: 2,
            hand_size: 5,
            num_fuses: 3,
            num_hints: 8,
            starting_player: PlayerIndex(0),
            seed: 0,
        };

        let deck = GameLog::new::<StdRng>(config.clone());

        let config = GameConfig {
            num_players: 2,
            hand_size: 5,
            num_fuses: 3,
            num_hints: 8,
            starting_player: PlayerIndex(0),
            seed: 1,
        };

        let deck_same_seed = GameLog::new::<StdRng>(config.clone());

        assert_ne!(
            deck.current_game_state().draw_pile,
            deck_same_seed.current_game_state().draw_pile
        );
    }
}
