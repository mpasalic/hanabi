use itertools::Itertools;
use rand::{Rng, SeedableRng};
use strum::IntoEnumIterator;

use crate::model::{
    Card, CardFace, CardSuit, GameConfig, GameEffect, GameOutcome, GameState, Hint, HintAction,
    PlayedCardResult, Player, PlayerAction, PlayerIndex, Slot, SlotIndex,
};

impl GameState {
    pub fn start<R: SeedableRng + Rng>(config: &GameConfig) -> Result<GameState, String> {
        let mut game = GameState {
            draw_pile: new_seeded_deck::<R>(config.seed),
            discard_pile: Vec::new(),
            last_turn: None,
            played_cards: Vec::new(),
            players: (0..config.num_players)
                .into_iter()
                .map(|_index| Player {
                    hand: (0..config.hand_size).map(|_slot_index| None).collect_vec(),
                })
                .collect(),
            remaining_bomb_count: config.num_fuses,
            remaining_hint_count: config.num_hints,
            turn: config.starting_player.0 as u8,
            outcome: None,
            history: Vec::new(),
            game_config: config.clone(),
        };

        use GameEffect::*;
        let init_effects = (0..config.hand_size)
            .flat_map(move |slot_index| {
                (0..config.num_players).map(move |player_index| {
                    DrawCard(PlayerIndex(player_index), SlotIndex(slot_index))
                })
            })
            .collect();

        game.run_effects(init_effects)?;
        return Ok(game);
    }

    // precondition: assumes the the action was taken by the current player
    pub fn play(&self, action: PlayerAction) -> Result<Vec<GameEffect>, String> {
        if let Some(outcome) = &self.outcome {
            return Err(format!("Game is already over: {:?}", outcome));
        }

        use GameEffect::*;
        let player_index = PlayerIndex(self.turn as usize % self.players.len());
        let current_player = self
            .players
            .get(self.turn as usize % self.players.len())
            .ok_or_else(|| "Invalid player index".to_string())?;
        let next_turn = PlayerIndex((self.turn as usize + 1) % self.players.len());

        match action {
            PlayerAction::PlayCard(SlotIndex(index)) => {
                let slot = current_player
                    .hand
                    .get(index)
                    .and_then(|s| s.as_ref().map(|s| s))
                    .ok_or_else(|| "Invalid slot index".to_string())?;
                let play_result = self.check_play(&slot.card);

                match play_result {
                    PlayedCardResult::Accepted => {
                        return Ok(vec![
                            RemoveCard(player_index, SlotIndex(index)),
                            PlaceOnBoard(slot.card),
                            DrawCard(player_index, SlotIndex(index)),
                            NextTurn(next_turn),
                        ]);
                    }
                    PlayedCardResult::CompletedSet => {
                        return Ok(vec![
                            RemoveCard(player_index, SlotIndex(index)),
                            PlaceOnBoard(slot.card),
                            DrawCard(player_index, SlotIndex(index)),
                            IncHint,
                            NextTurn(next_turn),
                        ]);
                    }
                    PlayedCardResult::Rejected => {
                        return Ok(vec![
                            RemoveCard(player_index, SlotIndex(index)),
                            AddToDiscrard(slot.card),
                            DrawCard(player_index, SlotIndex(index)),
                            BurnFuse,
                            NextTurn(next_turn),
                        ]);
                    }
                }
            }
            PlayerAction::DiscardCard(SlotIndex(index)) => {
                let slot = current_player
                    .hand
                    .get(index)
                    .and_then(|s| s.as_ref().map(|s| s))
                    .ok_or_else(|| "Invalid slot index".to_string())?;

                return Ok(vec![
                    RemoveCard(player_index, SlotIndex(index)),
                    AddToDiscrard(slot.card),
                    DrawCard(player_index, SlotIndex(index)),
                    IncHint,
                    NextTurn(next_turn),
                ]);
            }
            PlayerAction::GiveHint(PlayerIndex(hinted_player_index), hint_type) => {
                use HintAction::*;

                if self.remaining_hint_count <= 0 {
                    return Err("Not enough hints".to_string());
                }

                let hinted_player = self
                    .players
                    .get(hinted_player_index)
                    .ok_or_else(|| "Invalid player index".to_string())?;

                let hints: Vec<GameEffect> = hinted_player
                    .hand
                    .iter()
                    .enumerate()
                    .filter_map(|value| {
                        if let (index, Some(slot)) = value {
                            let slot_index = SlotIndex(index);
                            match (slot.card.face, slot.card.suit, hint_type) {
                                (face, _, SameFace(face_hint)) if face == face_hint => {
                                    Some(HintCard(
                                        PlayerIndex(hinted_player_index),
                                        slot_index,
                                        Hint::IsFace(face_hint),
                                    ))
                                }
                                (_, suit, SameSuit(suit_hint)) if suit == suit_hint => {
                                    Some(HintCard(
                                        PlayerIndex(hinted_player_index),
                                        slot_index,
                                        Hint::IsSuit(suit_hint),
                                    ))
                                }
                                (_, _, SameFace(face_hint)) => Some(HintCard(
                                    PlayerIndex(hinted_player_index),
                                    slot_index,
                                    Hint::IsNotFace(face_hint),
                                )),
                                (_, _, SameSuit(suit_hint)) => Some(HintCard(
                                    PlayerIndex(hinted_player_index),
                                    slot_index,
                                    Hint::IsNotSuit(suit_hint),
                                )),
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                let hinted_effects = vec![DecHint, NextTurn(next_turn)];

                return Ok(hints
                    .into_iter()
                    .chain(hinted_effects.into_iter())
                    .collect());
            }
        }
    }

    fn check_play(&self, card_played: &Card) -> PlayedCardResult {
        // Is the previous required card already played? Good!
        if let Some(required_card) = card_played.prev_card() {
            // Note: If prev_card is None, then we skip this check.
            if !self.played_cards.contains(&required_card) {
                return PlayedCardResult::Rejected;
            }
        }

        // Has this card already been played? Bad!
        let has_same_card = self.played_cards.contains(card_played);
        if has_same_card {
            return PlayedCardResult::Rejected;
        }

        if card_played.is_final_set_card() {
            PlayedCardResult::CompletedSet
        } else {
            PlayedCardResult::Accepted
        }
    }

    pub fn check_game_outcome(&self) -> Option<GameOutcome> {
        match (
            self.turn,
            self.last_turn,
            self.remaining_bomb_count,
            self.is_all_sets_complete(),
        ) {
            (_, _, _, true) => Some(GameOutcome::Win),
            (_, _, 0, false) => Some(GameOutcome::Fail {
                score: self.played_cards.len(),
            }),
            (current_turn, Some(last_turn), _, _) if current_turn > last_turn => {
                Some(GameOutcome::Fail {
                    score: self.played_cards.len(),
                })
            }
            (_, _, _, _) => None,
        }
    }

    fn is_all_sets_complete(&self) -> bool {
        for face in CardFace::iter() {
            for suit in CardSuit::iter() {
                let card = Card { face, suit };
                if !self.played_cards.contains(&card) {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn current_round(&self) -> u8 {
        self.turn / self.players.len() as u8
    }

    pub fn current_player_index(&self) -> PlayerIndex {
        PlayerIndex(self.turn as usize % self.players.len())
    }

    pub fn current_player(&self) -> Option<&Player> {
        match self.current_player_index() {
            PlayerIndex(player_index) => self.players.get(player_index),
        }
    }

    pub fn run_effects(&mut self, effects: Vec<GameEffect>) -> Result<(), String> {
        for effect in effects {
            self.run_effect(effect)?;
        }
        Ok(())
    }

    pub fn run_effect(&mut self, effect: GameEffect) -> Result<(), String> {
        match effect {
            GameEffect::DrawCard(PlayerIndex(player_index), SlotIndex(slot_index)) => {
                let player = self
                    .players
                    .get_mut(player_index)
                    .ok_or_else(|| "Invalid current player")?;
                let drawed_card = self.draw_pile.pop();

                if let Some(card) = drawed_card {
                    player
                        .hand
                        .get_mut(slot_index)
                        .ok_or_else(|| "Invalid slot index")?
                        .replace(Slot {
                            card: card,
                            hints: Vec::new(),
                        });
                }

                if self.draw_pile.len() == 0 && self.last_turn.is_none() {
                    self.last_turn = Some(self.turn + self.players.len() as u8);
                }
            }
            GameEffect::RemoveCard(PlayerIndex(player_index), SlotIndex(slot_index)) => {
                let player = self
                    .players
                    .get_mut(player_index)
                    .ok_or_else(|| "Invalid current player")?;
                player.hand[slot_index] = None;
            }
            GameEffect::AddToDiscrard(card) => {
                self.discard_pile.push(card);
            }
            GameEffect::PlaceOnBoard(card) => {
                self.played_cards.push(card);
            }
            GameEffect::HintCard(PlayerIndex(player_index), SlotIndex(slot_index), hint) => {
                let player = self
                    .players
                    .get_mut(player_index)
                    .ok_or_else(|| "Invalid current player")?;
                let slot = player
                    .hand
                    .get_mut(slot_index)
                    .ok_or_else(|| "Invalid slot index")?;
                if let Some(slot) = slot {
                    slot.hints.push(hint);
                }
            }
            GameEffect::DecHint => {
                self.remaining_hint_count = self.remaining_hint_count - 1;
            }
            GameEffect::IncHint => {
                self.remaining_hint_count = self.remaining_hint_count + 1;
            }
            GameEffect::BurnFuse => {
                self.remaining_bomb_count = self.remaining_bomb_count - 1;
            }
            GameEffect::NextTurn(_) => {
                self.turn = self.turn + 1;
            }
        }

        self.outcome = self.check_game_outcome();

        return Ok(());
    }
}

impl Card {
    fn prev_face(&self) -> Option<CardFace> {
        use CardFace::*;
        match self.face {
            One => None,
            Two => Some(One),
            Three => Some(Two),
            Four => Some(Three),
            Five => Some(Four),
        }
    }

    fn prev_card(&self) -> Option<Card> {
        if let Some(face) = self.prev_face() {
            Some(Card {
                face,
                suit: self.suit,
            })
        } else {
            return None;
        }
    }

    fn is_final_set_card(&self) -> bool {
        self.face == CardFace::Five
    }
}

pub fn num_cards() -> i32 {
    CardFace::iter()
        .flat_map(|face| {
            CardSuit::iter().map(move |_suit| match face {
                CardFace::One => 3,
                CardFace::Two | CardFace::Three | CardFace::Four => 2,
                CardFace::Five => 1,
            })
        })
        .sum()
}

pub fn new_standard_deck() -> Vec<Card> {
    let deck: Vec<Card> = CardFace::iter()
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

    return deck;
}

pub fn new_seeded_deck<R: SeedableRng + Rng>(seed: u64) -> Vec<Card> {
    let mut rand = R::seed_from_u64(seed);

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

#[cfg(test)]

mod tests {

    use assert_matches::assert_matches;

    use super::*;

    use CardFace::*;
    use CardSuit::*;

    fn card(face: CardFace, suit: CardSuit) -> Card {
        Card { face, suit }
    }

    #[test]
    fn test_drawing_last_card() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Green)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                Player {
                    hand: vec![
                        Some(Slot {
                            card: card(One, Red),
                            hints: vec![],
                        }),
                        Some(Slot {
                            card: card(Two, Red),
                            hints: vec![],
                        }),
                    ],
                },
                Player {
                    hand: vec![
                        Some(Slot {
                            card: card(One, Blue),
                            hints: vec![],
                        }),
                        Some(Slot {
                            card: card(Two, Blue),
                            hints: vec![],
                        }),
                    ],
                },
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
            history: vec![],
            game_config: GameConfig {
                num_players: 2,
                hand_size: 2,
                num_fuses: 3,
                num_hints: 8,
                starting_player: PlayerIndex(0),
                seed: 0,
            },
        };

        let action = PlayerAction::PlayCard(SlotIndex(0));
        let effects = game_state.play(action).unwrap();
        let result = game_state.run_effects(effects);

        assert!(result.is_ok());
        assert_matches!(
            &game_state,
            GameState {
                last_turn: Some(12),
                outcome: None,
                draw_pile,
                ..
            } if draw_pile.is_empty()
        );

        let action = PlayerAction::PlayCard(SlotIndex(0));
        let effects = game_state.play(action).unwrap();
        let result = game_state.run_effects(effects);

        assert!(result.is_ok());
        assert_matches!(
            &game_state,
            GameState {
                last_turn: Some(12),
                outcome: None,
                players,
                draw_pile,
                ..
            } if draw_pile.is_empty() && players[1].hand[0].is_none()
        );

        // last turn!
        let action = PlayerAction::PlayCard(SlotIndex(1));
        let effects = game_state.play(action).unwrap();
        let result = game_state.run_effects(effects);

        assert!(result.is_ok());
        assert_matches!(
            &game_state,
            GameState {
                last_turn: Some(12),
                outcome: Some(GameOutcome::Fail { score: _ }),
                players,
                draw_pile,
                ..
            } if draw_pile.is_empty() && players[0].hand[1].is_none()
        );
    }
}
