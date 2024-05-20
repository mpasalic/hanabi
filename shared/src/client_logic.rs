use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::model::{
    Card, CardFace, CardSuit, ClientPlayerView, GameConfig, GameEvent, GameState,
    GameStateSnapshot, HiddenSlot, HintAction, PlayerAction, PlayerIndex, SlotIndex,
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
        log: Vec<String>,
        players: Vec<OnlinePlayer>,
    },
    Started {
        players: Vec<OnlinePlayer>,
        game_state: GameStateSnapshot,
    },
    Ended {
        players: Vec<OnlinePlayer>,
        game_state: GameStateSnapshot,
        revealed_game_state: GameState,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientToServerMessage {
    // CreateGame
    Join {
        player_name: String,
        session_id: String,
        //
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
    UpdatedGameState(HanabiGame),
}

#[derive(Debug, Clone)]
pub enum HintState {
    ChoosingPlayer,
    ChoosingHintType { player_index: u8 },
    // ChoosingCard {
    //     player_index: u8,
    //     hint_type: HintBuilderType,
    // },
    ChoosingSuit { player_index: u8 },
    ChoosingFace { player_index: u8 },
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

#[derive(Debug, Clone)]
pub enum GameAction {
    Undo,
    StartGame,
    StartHint,
    StartPlay,
    StartDiscard,
    SelectPlayer { player_index: u8 },
    SelectHintType { hint_type: HintBuilderType },
    SelectSuit(CardSuit),
    SelectFace(CardFace),
    SelectCard(SlotIndex),
}

#[derive(Debug, Clone)]
pub enum CommandBuilder {
    Empty,
    Hint(HintState),
    Play(CardState),
    Discard(CardState),
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
        (C::Empty, A::StartHint) => C::Hint(HintState::ChoosingPlayer),
        (C::Empty, A::StartPlay) => C::Play(CardState::ChoosingCard {
            card_type: CardBuilderType::Play,
        }),
        (C::Empty, A::StartDiscard) => C::Discard(CardState::ChoosingCard {
            card_type: CardBuilderType::Discard,
        }),

        (C::Play(CardState::ChoosingCard { .. }), A::SelectCard(slot_index)) => {
            return (
                CommandState {
                    current_command: C::Empty,
                },
                Some(PlayerAction::PlayCard(slot_index)),
            )
        }

        (C::Discard(CardState::ChoosingCard { .. }), A::SelectCard(slot_index)) => {
            return (
                CommandState {
                    current_command: C::Empty,
                },
                Some(PlayerAction::DiscardCard(slot_index)),
            )
        }

        (C::Hint(HintState::ChoosingPlayer), A::SelectPlayer { player_index }) => {
            C::Hint(HintState::ChoosingHintType { player_index })
        }

        (
            C::Hint(HintState::ChoosingHintType { player_index }),
            A::SelectHintType { hint_type },
        ) => C::Hint(match hint_type {
            HintBuilderType::Suite => HintState::ChoosingSuit { player_index },
            HintBuilderType::Face => HintState::ChoosingFace { player_index },
        }),

        // TODO produce a command
        (C::Hint(HintState::ChoosingSuit { player_index }), A::SelectSuit(suit)) => {
            return (
                CommandState {
                    current_command: C::Empty,
                },
                Some(PlayerAction::GiveHint(
                    PlayerIndex(player_index as usize),
                    HintAction::SameSuit(suit),
                )),
            )
        }

        // TODO produce a command
        (C::Hint(HintState::ChoosingFace { player_index }), A::SelectFace(face)) => {
            return (
                CommandState {
                    current_command: C::Empty,
                },
                Some(PlayerAction::GiveHint(
                    PlayerIndex(player_index as usize),
                    HintAction::SameFace(face),
                )),
            )
        }

        // ----- Undo -----
        (C::Hint(HintState::ChoosingPlayer), A::Undo) => C::Empty,

        (C::Hint(HintState::ChoosingHintType { .. }), A::Undo) => {
            C::Hint(HintState::ChoosingPlayer)
        }

        (
            C::Hint(HintState::ChoosingSuit { player_index })
            | C::Hint(HintState::ChoosingFace { player_index }),
            A::Undo,
        ) => C::Hint(HintState::ChoosingHintType { player_index }),

        (C::Play(CardState::ChoosingCard { .. }), A::Undo) => C::Empty,
        (C::Discard(CardState::ChoosingCard { .. }), A::Undo) => C::Empty,

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

#[derive(Debug, Clone)]
pub struct GameLog {
    pub config: GameConfig,
    pub initial: GameState,
    pub log: Vec<(PlayerAction, GameState)>,
    pub history: Vec<GameEvent>,
}

impl GameLog {
    pub fn new(config: GameConfig) -> Self {
        GameLog {
            config: config.clone(),
            initial: GameState::start(&config).unwrap(),
            log: vec![],
            history: vec![],
        }
    }

    pub fn log<'a>(&'a mut self, action: PlayerAction) -> Result<&'a GameState, String> {
        let mut new_game_state = self.current_game_state().clone();
        self.history.push(GameEvent::PlayerAction(
            new_game_state.current_player_index(),
            action.clone(),
        ));

        let effects = new_game_state.play(action.clone())?;
        self.history.extend(
            effects
                .clone()
                .into_iter()
                .map(|effect| GameEvent::GameEffect(effect)),
        );

        new_game_state.run_effects(effects)?;
        self.log.push((action, new_game_state));

        Ok(&self.log.last().unwrap().1)
    }

    pub fn current_game_state(&self) -> GameState {
        match self.log.last() {
            Some((_index, game)) => game.clone(),
            None => self.initial.clone(),
        }
    }

    pub fn undo(&mut self) {
        self.log.pop();
    }

    pub fn into_client_game_state(&self, player: PlayerIndex) -> GameStateSnapshot {
        let game_state = self.current_game_state();
        GameStateSnapshot {
            log: self.history.clone(),
            player_snapshot: player,
            draw_pile_count: game_state.draw_pile.len() as u8,
            played_cards: game_state.played_cards.clone(),
            discard_pile: game_state.discard_pile.clone(),
            players: game_state
                .players
                .iter()
                .enumerate()
                .map(|(index, p)| match (index, player) {
                    (index, PlayerIndex(player)) if index == player => ClientPlayerView::Me {
                        hand: p
                            .hand
                            .iter()
                            .map(|h| {
                                h.as_ref().map(|s| HiddenSlot {
                                    hints: s.hints.clone(),
                                })
                            })
                            .collect(),
                    },
                    _ => ClientPlayerView::Teammate {
                        hand: p.hand.clone(),
                    },
                })
                .collect(),
            remaining_bomb_count: game_state.remaining_bomb_count,
            remaining_hint_count: game_state.remaining_hint_count,
            turn: game_state.current_player_index(),
            num_rounds: game_state.turn,
            last_turn: game_state.last_turn,
            outcome: game_state.outcome,
            game_config: game_state.game_config,
        }
    }
}

impl GameState {}
