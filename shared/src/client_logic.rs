use std::iter;

use itertools::Itertools;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::model::{
    CardFace, CardSuit, ClientPlayerView, GameConfig, GameEffect, GameEvent, GameOutcome,
    GameSnapshotEvent, GameState, GameStateSnapshot, HiddenSlot, HintAction, PlayerAction,
    PlayerIndex, SlotIndex,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnlinePlayer {
    pub name: String,
    pub connection_status: ConnectionStatus,
    pub is_host: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HanabiGame {
    Lobby {
        session_id: String,
        log: Vec<String>,
        players: Vec<OnlinePlayer>,
    },
    Started {
        session_id: String,
        players: Vec<OnlinePlayer>,
        game_state: GameStateSnapshot,
        log: Vec<GameSnapshotEvent>,
    },
    Ended {
        session_id: String,
        players: Vec<OnlinePlayer>,
        game_state: GameStateSnapshot,
        revealed_game_log: GameLog,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientToServerMessage {
    CreateGame {
        player_name: String,
    },
    Join {
        player_name: String,
        session_id: String,
    },
    StartGame,
    PlayerAction {
        action: PlayerAction,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Lobby {
    session_id: String,
    name: String,
    players: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]

pub enum ServerToClientMessage {
    CreatedGame { session_id: String },
    UpdatedGameState(HanabiGame),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum HintState {
    ChoosingPlayer,
    ChoosingHint { player_index: u8 },
    // ChoosingCard {
    //     player_index: u8,
    //     hint_type: HintBuilderType,
    // },
    // ChoosingSuit { player_index: u8 },
    // ChoosingFace { player_index: u8 },
}

#[derive(Debug, Clone)]
pub enum CardState {
    ChoosingCard { card_type: CardBuilderType },
}

#[derive(Debug, Clone, Copy)]
pub enum HintBuilderType {
    Suite,
    Face,
}

#[derive(Debug, Clone, Copy)]
pub enum CardBuilderType {
    Play,
    Discard,
}

#[derive(Debug, Clone, Copy)]
pub enum GameAction {
    Undo,
    StartGame,
    StartHint,
    StartPlay,
    StartDiscard,
    SelectPlayer { player_index: u8 },
    SelectSuit(CardSuit),
    SelectFace(CardFace),
    SelectCard(SlotIndex),
    Confirm(bool),
}

#[derive(Debug, Clone)]
pub enum CommandBuilder {
    Empty,
    Hinting(HintState),
    PlayingCard(CardState),
    DiscardingCard(CardState),
    ConfirmingAction(PlayerAction),
}

#[derive(Debug, Clone)]
pub struct CommandState {
    pub current_command: CommandBuilder,
}

pub fn process_app_action(
    state: CommandState,
    action: GameAction,
) -> (CommandState, Option<PlayerAction>) {
    use CommandBuilder as C;
    use GameAction as A;
    let builder = match (state.current_command, action) {
        (C::Empty, A::StartHint) => C::Hinting(HintState::ChoosingPlayer),
        (C::Empty, A::StartPlay) => C::PlayingCard(CardState::ChoosingCard {
            card_type: CardBuilderType::Play,
        }),
        (C::Empty, A::StartDiscard) => C::DiscardingCard(CardState::ChoosingCard {
            card_type: CardBuilderType::Discard,
        }),

        (C::PlayingCard(CardState::ChoosingCard { .. }), A::SelectCard(slot_index)) => {
            C::ConfirmingAction(PlayerAction::PlayCard(slot_index))
        }

        (C::DiscardingCard(CardState::ChoosingCard { .. }), A::SelectCard(slot_index)) => {
            C::ConfirmingAction(PlayerAction::DiscardCard(slot_index))
        }

        (C::Hinting(HintState::ChoosingPlayer), A::SelectPlayer { player_index }) => {
            C::Hinting(HintState::ChoosingHint { player_index })
        }

        (C::Hinting(HintState::ChoosingHint { player_index }), A::SelectSuit(suit)) => {
            C::ConfirmingAction(PlayerAction::GiveHint(
                PlayerIndex(player_index as usize),
                HintAction::SameSuit(suit),
            ))
        }

        (C::Hinting(HintState::ChoosingHint { player_index }), A::SelectFace(face)) => {
            C::ConfirmingAction(PlayerAction::GiveHint(
                PlayerIndex(player_index as usize),
                HintAction::SameFace(face),
            ))
        }

        // ----- Confirming the action -----
        (C::ConfirmingAction(_), A::Confirm(false)) => C::Empty,

        (C::ConfirmingAction(action), A::Confirm(true)) => {
            return (
                CommandState {
                    current_command: C::Empty,
                },
                Some(action),
            )
        }

        // ----- Undo -----
        (
            C::Hinting(HintState::ChoosingPlayer)
            | C::PlayingCard(CardState::ChoosingCard { .. })
            | C::DiscardingCard(CardState::ChoosingCard { .. })
            | C::ConfirmingAction(_),
            A::Undo,
        ) => C::Empty,

        (C::Hinting(HintState::ChoosingHint { .. }), A::Undo) => {
            C::Hinting(HintState::ChoosingPlayer)
        }

        // ------ other wise do nothing -------
        (builder, _) => builder,
    };

    (
        CommandState {
            current_command: builder,
        },
        None,
    )
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameLog {
    pub config: GameConfig,
    pub initial: GameState,
    pub log: Vec<(PlayerIndex, PlayerAction, Vec<GameEffect>, GameState)>,
}

impl GameLog {
    pub fn new<R: SeedableRng + Rng>(config: GameConfig) -> Self {
        GameLog {
            config: config.clone(),
            initial: GameState::start_with_seed::<R>(&config).unwrap(),
            log: vec![],
        }
    }

    pub fn log<'a>(&'a mut self, action: PlayerAction) -> Result<&'a GameState, String> {
        let mut new_game_state = self.current_game_state().clone();
        let current_player_index = new_game_state.current_player_index();

        let effects = new_game_state.play(action.clone())?;
        let logged_effects = effects.clone();

        new_game_state.run_effects(effects)?;

        self.log.push((
            current_player_index,
            action,
            logged_effects,
            new_game_state.clone(),
        ));

        Ok(&self.log.last().unwrap().3)
    }

    pub fn current_game_state(&self) -> GameState {
        match self.log.last() {
            Some((_index, _action, _effects, game)) => game.clone(),
            None => self.initial.clone(),
        }
    }

    pub fn undo(&mut self) {
        self.log.pop();
    }

    pub fn into_client_game_log(
        &self,
        client_player_index: PlayerIndex,
        name: Vec<String>,
    ) -> Vec<GameSnapshotEvent> {
        self.log
            .iter()
            .map(|(log_player_index, action, effects, game_state)| {
                let game_event = GameEvent::PlayerAction {
                    player_index: *log_player_index,
                    action: action.clone(),
                    effects: effects.clone(),
                };
                GameSnapshotEvent {
                    event: game_event,
                    snapshot: self.into_client_game_state(
                        game_state.clone(),
                        client_player_index,
                        name.clone(),
                    ),
                }
            })
            .chain(
                iter::once(self.current_game_state()).filter_map(|game_state| {
                    if game_state.outcome.is_some() {
                        Some(GameSnapshotEvent {
                            event: GameEvent::GameOver(game_state.outcome.clone().unwrap()),
                            snapshot: self.into_client_game_state(
                                game_state.clone(),
                                client_player_index,
                                name.clone(),
                            ),
                        })
                    } else {
                        None
                    }
                }),
            )
            .collect_vec()
    }

    pub fn into_client_game_state(
        &self,
        game_state: GameState,
        client_player_index: PlayerIndex,
        name: Vec<String>,
    ) -> GameStateSnapshot {
        GameStateSnapshot {
            this_client_player_index: client_player_index,
            draw_pile_count: game_state.draw_pile.len() as u8,
            played_cards: game_state.played_cards.clone(),
            discard_pile: game_state.discard_pile.clone(),
            players: game_state
                .players
                .iter()
                .enumerate()
                .map(|(index, p)| {
                    if PlayerIndex(index) == client_player_index {
                        ClientPlayerView::Me {
                            name: name[index].clone(),
                            hand: p
                                .hand
                                .iter()
                                .map(|h| {
                                    h.as_ref().map(|s| HiddenSlot {
                                        hints: s.hints.clone(),
                                    })
                                })
                                .collect(),
                        }
                    } else {
                        ClientPlayerView::Teammate {
                            name: name[index].clone(),
                            hand: p.hand.clone(),
                        }
                    }
                })
                .collect(),
            remaining_bomb_count: game_state.remaining_bomb_count,
            remaining_hint_count: game_state.remaining_hint_count,
            current_turn_player_index: game_state.current_player_index(),
            num_rounds: game_state.turn,
            last_turn: game_state.last_turn,
            outcome: game_state.outcome,
            game_config: self.config.clone(),
        }
    }
}

impl GameState {}
