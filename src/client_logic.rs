use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use strum::IntoEnumIterator;

use crate::model::{
    Card, CardFace, CardSuit, ClientGameState, ClientPlayerView, GameConfig, GameState, HiddenSlot,
    HintAction, PlayerAction, PlayerIndex, SlotIndex,
};

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

#[derive(Debug)]
pub enum AppAction {
    Undo,
    Quit,
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
    action: AppAction,
) -> (CommandState, Option<PlayerAction>) {
    use AppAction as A;
    use CommandBuilder as C;
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

        (C::Hint(HintState::ChoosingHintType { player_index }), A::Undo) => {
            C::Hint(HintState::ChoosingPlayer)
        }

        (
            C::Hint(HintState::ChoosingSuit { player_index })
            | C::Hint(HintState::ChoosingFace { player_index }),
            A::Undo,
        ) => C::Hint(HintState::ChoosingHintType { player_index }),

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
    pub initial: GameState,
    pub log: Vec<(PlayerAction, GameState)>,
}

impl GameLog {
    pub fn new(config: GameConfig) -> Self {
        GameLog {
            initial: GameState::start(&config).unwrap(),
            log: vec![],
        }
    }

    pub fn log<'a>(&'a mut self, action: PlayerAction) -> Result<&'a GameState, String> {
        let mut new_game_state = self.current_game_state().clone();
        let effects = new_game_state.play(action.clone())?;
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
}

impl GameState {
    pub fn into_client_game_state(self, player: PlayerIndex) -> ClientGameState {
        ClientGameState {
            draw_pile_count: self.draw_pile.len() as u8,
            played_cards: self.played_cards.clone(),
            discard_pile: self.discard_pile.clone(),
            players: self
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
            remaining_bomb_count: self.remaining_bomb_count,
            remaining_hint_count: self.remaining_hint_count,
            current_player_index: self.current_player_index(),
            turn: self.turn,
            last_turn: self.last_turn,
            outcome: self.outcome,
        }
    }
}

pub fn new_seeded_deck(seed: u64) -> Vec<Card> {
    let mut rand = ChaCha8Rng::seed_from_u64(seed);

    let mut deck: Vec<Card> = CardFace::iter()
        .flat_map(|face| {
            CardSuit::iter().flat_map(move |suit| {
                let num = match face {
                    CardFace::One => 3,
                    CardFace::Two | CardFace::Three | CardFace::Four => 2,
                    CardFace::Five => 1,
                };
                vec![Card { suit, face }; num]
            })
        })
        .collect();

    for index in 0..deck.len() {
        let swap = rand.gen_range(index..deck.len());
        deck.swap(index, swap);
    }
    return deck;
}
