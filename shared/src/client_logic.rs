use std::iter;

use itertools::Itertools;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::model::{
    CardFace, CardSuit, ClientPlayerView, GameConfig, GameEffect, GameOutcome, GameSnapshotEvent,
    GameState, GameStateSnapshot, HiddenSlot, HintAction, Player, PlayerAction, PlayerIndex,
    SlotIndex,
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
    Spectate {
        session_id: String,
        players: Vec<OnlinePlayer>,
        game_state: GameStateSnapshot,
        revealed_game_log: GameLog,
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
    Spectate {
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

#[derive(Debug, Clone)]
pub enum MovingCardState {
    ChoosingCard {
        card_type: CardBuilderType,
    },
    ChangeSlot {
        from_slot_index: SlotIndex,
        new_slot_index: SlotIndex,
    },
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
    Move,
}

#[derive(Debug, Clone, Copy)]
pub enum MoveCardDirection {
    Left,
    Right,
    Start,
    End,
}

#[derive(Debug, Clone, Copy)]
pub enum GameAction {
    Undo,
    StartGame,
    StartHint,
    StartPlay,
    StartDiscard,
    StartMove,
    SelectPlayer { player_index: u8 },
    SelectSuit(CardSuit),
    SelectFace(CardFace),
    SelectCard(SlotIndex),
    Confirm(bool),
    SelectSlot(SlotIndex),
}

#[derive(Debug, Clone)]
pub enum CommandBuilder {
    Empty,
    Hinting(HintState),
    PlayingCard(CardState),
    DiscardingCard(CardState),
    MovingCard(MovingCardState),
    ConfirmingAction(PlayerAction),
}

#[derive(Debug, Clone)]
pub struct CommandState {
    pub current_player: PlayerIndex,
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
        (C::Empty, A::StartMove) => C::MovingCard(MovingCardState::ChoosingCard {
            card_type: CardBuilderType::Move,
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

        (
            C::MovingCard(MovingCardState::ChoosingCard {
                card_type: CardBuilderType::Move,
            }),
            A::SelectCard(slot_index),
        ) => C::MovingCard(MovingCardState::ChangeSlot {
            from_slot_index: slot_index,
            new_slot_index: slot_index,
        }),

        (
            C::MovingCard(MovingCardState::ChangeSlot {
                from_slot_index, ..
            }),
            A::SelectSlot(new_slot_index),
        ) => C::MovingCard(MovingCardState::ChangeSlot {
            from_slot_index,
            new_slot_index: new_slot_index,
        }),

        (
            C::MovingCard(MovingCardState::ChangeSlot {
                from_slot_index,
                new_slot_index,
            }),
            A::Confirm(true),
        ) => {
            return (
                CommandState {
                    current_player: state.current_player,
                    current_command: C::Empty,
                },
                Some(PlayerAction::MoveSlot(
                    state.current_player,
                    from_slot_index,
                    new_slot_index,
                )),
            )
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
                    current_player: state.current_player,
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
            | C::ConfirmingAction(_)
            | C::MovingCard(_),
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
            current_player: state.current_player,
            current_command: builder,
        },
        None,
    )
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameLogEvent {
    pub current_turn_count: u8,
    pub current_turn_player_index: PlayerIndex,
    pub event_player_index: PlayerIndex,
    pub event_action: PlayerAction,
    pub event_effects: Vec<GameEffect>,
    pub post_event_game_state: GameState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameLog {
    pub config: GameConfig,
    pub initial: GameState,
    pub log: Vec<GameLogEvent>,
}

impl GameLog {
    pub fn new<R: SeedableRng + Rng>(config: GameConfig) -> Self {
        GameLog {
            config: config.clone(),
            initial: GameState::start_with_seed::<R>(&config).unwrap(),
            log: vec![],
        }
    }

    pub fn log<'a>(
        &'a mut self,
        actor: PlayerIndex,
        action: PlayerAction,
    ) -> Result<&'a GameLogEvent, String> {
        let mut current_game_state = self.current_game_state();
        let current_turn_count = current_game_state.turn;
        let current_turn_player_index = current_game_state.current_player_index();

        let effects = current_game_state.play(action.clone())?;
        let logged_effects = effects.clone();

        current_game_state.run_effects(effects)?;
        let new_game_state = current_game_state;

        let new_log_event = GameLogEvent {
            current_turn_count,
            current_turn_player_index,
            event_player_index: actor,
            event_action: action,
            event_effects: logged_effects,
            post_event_game_state: new_game_state.clone(),
        };

        self.log.push(new_log_event);

        Ok(&self.log.last().unwrap())
    }

    pub fn current_game_state(&self) -> GameState {
        self.log
            .last()
            .map(|e| e.post_event_game_state.clone())
            .unwrap_or(self.initial.clone())
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
            .map(
                |GameLogEvent {
                     current_turn_count,
                     current_turn_player_index,
                     event_player_index,
                     event_action,
                     event_effects,
                     post_event_game_state,
                 }| {
                    GameSnapshotEvent {
                        current_turn_count: *current_turn_count,
                        current_turn_player_index: *current_turn_player_index,
                        event_player_index: *event_player_index,
                        event_action: event_action.clone(),
                        effects: event_effects.clone(),
                        post_event_game_snapshot: self.into_client_game_state(
                            post_event_game_state.clone(),
                            client_player_index,
                            name.clone(),
                        ),
                    }
                },
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
                                        draw_number: s.draw_number,
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

impl GameStateSnapshot {
    pub fn apply_local_mutation(&mut self, action: PlayerAction) {
        match action {
            PlayerAction::MoveSlot(
                PlayerIndex(player_index),
                SlotIndex(from_slot_index),
                SlotIndex(new_slot_index),
            ) => {
                match self.players[player_index] {
                    ClientPlayerView::Me { ref mut hand, .. } => {
                        if from_slot_index < new_slot_index {
                            hand[from_slot_index..=new_slot_index].rotate_left(1);
                        } else {
                            hand[new_slot_index..=from_slot_index].rotate_right(1);
                        }
                    }
                    _ => unreachable!("Only the current player can move a card"),
                    // shared::model::ClientPlayerView::Teammate { name, hand } => todo!(),
                }
            }
            _ => {}
        }
    }
}
