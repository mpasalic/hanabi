use std::iter;

use itertools::Itertools;
use rand::{Rng, SeedableRng};
use strum::IntoEnumIterator;

use crate::model::{
    Card, CardFace, CardSuit, GameConfig, GameEffect, GameOutcome, GameState, Hint, HintAction,
    PlayedCardResult, Player, PlayerAction, PlayerIndex, Slot, SlotIndex,
};

impl GameState {
    pub fn start_with_seed<R: SeedableRng + Rng>(config: &GameConfig) -> Result<GameState, String> {
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

    pub fn start_with_deck<R: SeedableRng + Rng>(
        config: &GameConfig,
        deck: Vec<Card>,
    ) -> Result<GameState, String> {
        let mut game = GameState {
            draw_pile: deck,
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

        fn draw_card_effect(
            game_state: &GameState,
            player_index: PlayerIndex,
            slot_index: SlotIndex,
        ) -> impl Iterator<Item = GameEffect> {
            match game_state {
                GameState { draw_pile, .. } if draw_pile.len() > 1 => {
                    vec![GameEffect::DrawCard(player_index, slot_index)].into_iter()
                }

                GameState {
                    draw_pile, turn, ..
                } if draw_pile.len() == 1 => vec![
                    GameEffect::DrawCard(player_index, slot_index),
                    GameEffect::MarkLastTurn(*turn + game_state.players.len() as u8),
                ]
                .into_iter(),

                _ => vec![].into_iter(),
            }
        }

        fn next_turn_effect(game_state: &GameState) -> GameEffect {
            match (game_state.turn, game_state.last_turn) {
                (current_turn, Some(last_turn)) if current_turn >= last_turn => {
                    GameEffect::LastTurn
                }
                _ => GameEffect::NextTurn(game_state.turn + 1),
            }
        }

        match action {
            PlayerAction::PlayCard(SlotIndex(slot_index)) => {
                let slot = current_player
                    .hand
                    .get(slot_index)
                    .and_then(|s| s.as_ref().map(|s| s))
                    .ok_or_else(|| "Invalid slot index".to_string())?;
                let play_result = self.check_play(&slot.card);

                return Ok(match play_result {
                    PlayedCardResult::Accepted => [
                        RemoveCard(player_index, SlotIndex(slot_index)),
                        PlaceOnBoard(slot.card),
                    ]
                    .into_iter()
                    .chain(draw_card_effect(self, player_index, SlotIndex(slot_index)))
                    .chain(iter::once(next_turn_effect(self)))
                    .collect_vec(),
                    PlayedCardResult::CompletedSet => [
                        RemoveCard(player_index, SlotIndex(slot_index)),
                        PlaceOnBoard(slot.card),
                        IncHint,
                    ]
                    .into_iter()
                    .chain(draw_card_effect(self, player_index, SlotIndex(slot_index)))
                    .chain(iter::once(next_turn_effect(self)))
                    .collect_vec(),
                    PlayedCardResult::Rejected => [
                        RemoveCard(player_index, SlotIndex(slot_index)),
                        AddToDiscard(slot.card),
                        BurnFuse,
                    ]
                    .into_iter()
                    .chain(draw_card_effect(self, player_index, SlotIndex(slot_index)))
                    .chain(iter::once(next_turn_effect(self)))
                    .collect_vec(),
                });
            }
            PlayerAction::DiscardCard(SlotIndex(slot_index)) => {
                let slot = current_player
                    .hand
                    .get(slot_index)
                    .and_then(|s| s.as_ref().map(|s| s))
                    .ok_or_else(|| "Invalid slot index".to_string())?;

                return Ok([
                    RemoveCard(player_index, SlotIndex(slot_index)),
                    AddToDiscard(slot.card),
                    IncHint,
                ]
                .into_iter()
                .chain(draw_card_effect(self, player_index, SlotIndex(slot_index)))
                .chain(iter::once(next_turn_effect(self)))
                .collect_vec());
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
                let hinted_effects = vec![DecHint, next_turn_effect(self)];

                return Ok(hints
                    .into_iter()
                    .chain(hinted_effects.into_iter())
                    .collect());
            }
            PlayerAction::MoveSlot(
                PlayerIndex(player_index),
                SlotIndex(from_slot_index),
                SlotIndex(to_slot_index),
            ) => {
                let player = self
                    .players
                    .get(player_index)
                    .ok_or_else(|| "Invalid player index".to_string())?;

                let from_slot = player
                    .hand
                    .get(from_slot_index)
                    .and_then(|s| s.as_ref().map(|s| s))
                    .ok_or_else(|| "Invalid from slot index".to_string())?;

                let to_slot = player
                    .hand
                    .get(to_slot_index)
                    .and_then(|s| s.as_ref().map(|s| s))
                    .ok_or_else(|| "Invalid to slot index".to_string())?;

                if from_slot_index == to_slot_index {
                    return Err("Cannot move slot to itself".to_string());
                }

                return Ok(vec![GameEffect::MoveSlot(
                    PlayerIndex(player_index),
                    SlotIndex(from_slot_index),
                    SlotIndex(to_slot_index),
                )]);
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
                assert!(
                    self.players[player_index].hand[slot_index].is_none(),
                    "Slot is not empty"
                );
                self.players[player_index].hand[slot_index] = Some(Slot {
                    card: self
                        .draw_pile
                        .pop()
                        .ok_or_else(|| "Logic error: No more cards to draw")?,
                    hints: vec![],
                });
            }
            GameEffect::MarkLastTurn(turn_count) => {
                self.last_turn = Some(turn_count);
            }
            GameEffect::RemoveCard(PlayerIndex(player_index), SlotIndex(slot_index)) => {
                self.players[player_index].hand[slot_index] = None;
            }
            GameEffect::AddToDiscard(card) => {
                self.discard_pile.push(card);
            }
            GameEffect::PlaceOnBoard(card) => {
                self.played_cards.push(card);
            }
            GameEffect::HintCard(PlayerIndex(player_index), SlotIndex(slot_index), hint) => {
                self.players[player_index].hand[slot_index]
                    .as_mut()
                    .ok_or_else(|| "No card to hint in slot index")?
                    .hints
                    .push(hint);
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
            GameEffect::LastTurn => {
                self.turn = self.turn + 1;
                self.outcome = self.check_game_outcome();
                // TODO implement (noop for now)
            }
            GameEffect::MoveSlot(
                PlayerIndex(player_index),
                SlotIndex(from_slot_index),
                SlotIndex(new_slot_index),
            ) => {
                if from_slot_index < new_slot_index {
                    self.players[player_index].hand[from_slot_index..=new_slot_index]
                        .rotate_left(1);
                } else {
                    self.players[player_index].hand[new_slot_index..=from_slot_index]
                        .rotate_right(1);
                }
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
    use rand::rngs::StdRng;

    use super::*;

    use CardFace::*;
    use CardSuit::*;

    fn card(face: CardFace, suit: CardSuit) -> Card {
        Card { face, suit }
    }

    fn card_slot(face: CardFace, suit: CardSuit) -> Option<Slot> {
        Some(Slot {
            card: card(face, suit),
            hints: vec![],
        })
    }

    fn hand(cards: &[Option<Card>]) -> Vec<Option<Slot>> {
        cards
            .iter()
            .map(|card| {
                card.map(|card| Slot {
                    card,
                    hints: vec![],
                })
            })
            .collect()
    }

    fn player(hand: &[Option<Slot>]) -> Player {
        Player {
            hand: hand.to_vec(),
        }
    }

    #[test]
    fn test_game_state_start_2_players() {
        let game_state = GameState::start_with_seed::<StdRng>(&GameConfig::new(2, 0)).unwrap();

        assert_matches!(
            &game_state.players.as_slice(),
            &[Player {
                        hand: player_1_hand
                    },
                    Player {
                        hand: player_2_hand
                    },
                ] if player_1_hand.len() == 5 && player_2_hand.len() == 5
        );
    }

    #[test]
    fn test_game_state_start_4_players() {
        let game_state = GameState::start_with_seed::<StdRng>(&GameConfig::new(4, 0)).unwrap();

        assert_matches!(
            &game_state.players.as_slice(),
            &[
                Player {
                    hand: player_1_hand
                },
                Player {
                    hand: player_2_hand
                },
                Player {
                    hand: player_3_hand
                },
                Player {
                    hand: player_4_hand
                },
            ] => {
                assert_matches!(player_1_hand.len(), 4);
                assert_matches!(player_2_hand.len(), 4);
                assert_matches!(player_3_hand.len(), 4);
                assert_matches!(player_4_hand.len(), 4);
            }
        );
    }

    #[test]
    fn test_remove_card_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(Four, Blue), card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects(vec![GameEffect::RemoveCard(PlayerIndex(0), SlotIndex(0))])
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                players: vec![
                    player(&[None, card_slot(Five, Blue)]),
                    player(&[card_slot(Four, Green), card_slot(Five, Green)]),
                ],
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_add_to_discard_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(Four, Blue), card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects(vec![GameEffect::AddToDiscard(card(One, Red))])
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                discard_pile: vec![card(One, Red)],
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_draw_card_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(Two, Red), card(Three, Red), card(One, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects(vec![GameEffect::DrawCard(PlayerIndex(0), SlotIndex(0))])
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                draw_pile: vec![card(Two, Red), card(Three, Red)],
                players: vec![
                    player(&[card_slot(One, Red), card_slot(Five, Blue)]),
                    player(&[card_slot(Four, Green), card_slot(Five, Green)]),
                ],
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_dec_hint_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
            outcome: None,
        };

        game_state.run_effects(vec![GameEffect::DecHint]).unwrap();

        assert_eq!(
            game_state,
            GameState {
                remaining_hint_count: 7,
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_inc_hint_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
            outcome: None,
        };

        game_state.run_effects(vec![GameEffect::IncHint]).unwrap();

        assert_eq!(
            game_state,
            GameState {
                remaining_hint_count: 9,
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_burn_fuse_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects([GameEffect::BurnFuse].to_vec())
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                remaining_bomb_count: 2,
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_place_on_board_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects([GameEffect::PlaceOnBoard(card(One, Yellow))].to_vec())
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                played_cards: vec![card(One, Yellow)],
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_mark_last_turn_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects([GameEffect::MarkLastTurn(12)].to_vec())
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                last_turn: Some(12),
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_mark_next_turn_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects([GameEffect::NextTurn(11)].to_vec())
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                turn: 11,
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_mark_hint_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[None, card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects(
                [GameEffect::HintCard(
                    PlayerIndex(1),
                    SlotIndex(1),
                    Hint::IsFace(Five),
                )]
                .to_vec(),
            )
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                players: [
                    player(&[None, card_slot(Five, Blue)]),
                    Player {
                        hand: [
                            card_slot(Four, Green),
                            Some(Slot {
                                card: card(Five, Green),
                                hints: [Hint::IsFace(Five)].to_vec()
                            })
                        ]
                        .to_vec(),
                    }
                ]
                .to_vec(),
                ..game_state.clone()
            }
        );
    }

    #[test]
    fn test_move_slot_effect() {
        let mut game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(Five, Green)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        game_state
            .run_effects(
                [GameEffect::MoveSlot(
                    PlayerIndex(0),
                    SlotIndex(0),
                    SlotIndex(1),
                )]
                .to_vec(),
            )
            .unwrap();

        assert_eq!(
            game_state,
            GameState {
                players: [
                    player(&[card_slot(Five, Blue), card_slot(One, Blue)]),
                    player(&[card_slot(Four, Green), card_slot(Five, Green)]),
                ]
                .to_vec(),
                ..game_state.clone()
            }
        );
    }

    fn assert_vector_contains_eq(actual: Vec<GameEffect>, expected: Vec<GameEffect>) {
        assert_eq!(
            expected.len(),
            actual.len(),
            "Different length of effects,\nexpected:\n{:#?}\n actual:\n{:#?}",
            expected,
            actual
        );
        for effect in expected {
            assert!(actual.contains(&effect), "{:?} not found", effect);
        }
    }

    #[test]
    fn test_plays_normal_card_action() {
        let game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 1,
            last_turn: None,
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::PlayCard(SlotIndex(1)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(1), SlotIndex(1)),
                GameEffect::PlaceOnBoard(card(One, Yellow)),
                GameEffect::DrawCard(PlayerIndex(1), SlotIndex(1)),
                GameEffect::NextTurn(2),
            ],
        );
    }

    #[test]
    fn test_plays_card_last_turn_action() {
        let game_state = GameState {
            draw_pile: vec![],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 12,
            last_turn: Some(12),
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::PlayCard(SlotIndex(0)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(0), SlotIndex(0)),
                GameEffect::PlaceOnBoard(card(One, Blue)),
                GameEffect::LastTurn,
            ],
        );
    }

    #[test]
    fn test_plays_rejected_card_action() {
        let game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Blue)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 1,
            last_turn: None,
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::PlayCard(SlotIndex(0)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(1), SlotIndex(0)),
                GameEffect::AddToDiscard(card(Four, Green)),
                GameEffect::DrawCard(PlayerIndex(1), SlotIndex(0)),
                GameEffect::NextTurn(2),
                GameEffect::BurnFuse,
            ],
        );
    }

    #[test]
    fn test_plays_completing_card_action() {
        let game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Red)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::PlayCard(SlotIndex(1)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::PlaceOnBoard(card(Five, Red)),
                GameEffect::DrawCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::NextTurn(11),
                GameEffect::IncHint,
            ],
        );
    }

    #[test]
    fn test_plays_completing_card_with_last_card_draw_action() {
        let game_state = GameState {
            draw_pile: vec![card(One, Red)],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Red)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::PlayCard(SlotIndex(1)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::PlaceOnBoard(card(Five, Red)),
                GameEffect::DrawCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::NextTurn(11),
                GameEffect::IncHint,
                GameEffect::MarkLastTurn(12),
            ],
        );
    }

    #[test]
    fn test_discards_card_action() {
        let game_state = GameState {
            draw_pile: vec![card(One, Red), card(Two, Red), card(Three, Red)],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Red)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::DiscardCard(SlotIndex(1)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::AddToDiscard(card(Five, Red)),
                GameEffect::DrawCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::NextTurn(11),
                GameEffect::IncHint,
            ],
        );
    }

    #[test]
    fn test_discards_card_no_draw_action() {
        let game_state = GameState {
            draw_pile: vec![],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Red)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: Some(12),
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::DiscardCard(SlotIndex(1)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::AddToDiscard(card(Five, Red)),
                GameEffect::NextTurn(11),
                GameEffect::IncHint,
            ],
        );
    }

    #[test]
    fn test_discards_card_with_last_card_draw_action() {
        let game_state = GameState {
            draw_pile: vec![card(One, Red)],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[card_slot(One, Blue), card_slot(Five, Red)]),
                player(&[card_slot(Four, Green), card_slot(One, Yellow)]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: None,
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::DiscardCard(SlotIndex(1)));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::RemoveCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::AddToDiscard(card(Five, Red)),
                GameEffect::DrawCard(PlayerIndex(0), SlotIndex(1)),
                GameEffect::NextTurn(11),
                GameEffect::IncHint,
                GameEffect::MarkLastTurn(12),
            ],
        );
    }

    #[test]
    fn test_gives_face_hint_action() {
        let game_state = GameState {
            draw_pile: vec![],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[
                    card_slot(One, Blue),
                    card_slot(Five, Red),
                    card_slot(Five, Blue),
                    card_slot(Three, Blue),
                ]),
                player(&[
                    card_slot(Four, Green),
                    card_slot(Five, White),
                    card_slot(Five, Green),
                    card_slot(Three, Blue),
                ]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: Some(12),
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::GiveHint(
            PlayerIndex(1),
            HintAction::SameFace(Five),
        ));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(0), Hint::IsNotFace(Five)),
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(1), Hint::IsFace(Five)),
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(2), Hint::IsFace(Five)),
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(3), Hint::IsNotFace(Five)),
                GameEffect::NextTurn(11),
                GameEffect::DecHint,
            ],
        );
    }

    #[test]
    fn test_gives_suit_hint_action() {
        let game_state = GameState {
            draw_pile: vec![],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[
                    card_slot(One, Blue),
                    card_slot(Five, Red),
                    card_slot(Five, Blue),
                    card_slot(Three, Blue),
                ]),
                player(&[
                    card_slot(Four, Green),
                    card_slot(Five, White),
                    card_slot(Five, Green),
                    card_slot(Three, Blue),
                ]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: Some(12),
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::GiveHint(
            PlayerIndex(1),
            HintAction::SameSuit(Green),
        ));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(0), Hint::IsSuit(Green)),
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(1), Hint::IsNotSuit(Green)),
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(2), Hint::IsSuit(Green)),
                GameEffect::HintCard(PlayerIndex(1), SlotIndex(3), Hint::IsNotSuit(Green)),
                GameEffect::NextTurn(11),
                GameEffect::DecHint,
            ],
        );
    }

    #[test]
    fn test_move_slot_action() {
        let game_state = GameState {
            draw_pile: vec![],
            played_cards: vec![
                card(One, Red),
                card(Two, Red),
                card(Three, Red),
                card(Four, Red),
            ],
            discard_pile: vec![],
            players: vec![
                player(&[
                    card_slot(One, Blue),
                    card_slot(Five, Red),
                    card_slot(Five, Blue),
                    card_slot(Three, Blue),
                ]),
                player(&[
                    card_slot(Four, Green),
                    card_slot(Five, White),
                    card_slot(Five, Green),
                    card_slot(Three, Blue),
                ]),
            ],
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 10,
            last_turn: Some(12),
            outcome: None,
        };

        let effects = game_state.play(PlayerAction::MoveSlot(
            PlayerIndex(0),
            SlotIndex(0),
            SlotIndex(3),
        ));

        assert_vector_contains_eq(
            effects.unwrap(),
            vec![GameEffect::MoveSlot(
                PlayerIndex(0),
                SlotIndex(0),
                SlotIndex(3),
            )],
        );
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

    // Deck [1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5]
    // Each player dealt [1, 2, 3, 4]
    // Each player plays slot 0, then slot 1, then ..
    #[test]
    fn test_full_game_todo() {
        // use PlayerAction::*;

        // fn lucky_deck() -> Vec<Card> {
        //     [One, Two, Three, Four, Five].into_iter().map((|face| [Red, Blue, Green, White, Yellow].into_iter().map(|suit| Card { face, suit }))).flatten().collect()

        // }

        // fn S(slot_index: usize) -> SlotIndex {
        //     SlotIndex(slot_index)
        // }

        // fn P(player_index: usize) -> PlayerIndex {
        //     PlayerIndex(player_index)
        // }

        // let lucky_game = GameState::start_with_deck(&GameConfig {
        //     num_players: 5,
        //     hand_size: 5,
        //     num_fuses: 3,
        //     num_hints: 8,
        //     starting_player:  P(0),
        //     seed: 0,
        // }, lucky_deck());

        // let actions = [
        //     PlayCard(S(0)),
        //     PlayCard(S(0)),
        //     PlayCard(S(0)),
        //     PlayCard(S(0)),
        //     PlayCard(S(0)),
        //     PlayCard(S(1)),
        //     PlayCard(S(1)),
        //     PlayCard(S(1)),
        //     PlayCard(S(1)),
        //     PlayCard(S(1)),

        // ]

        // assert!(result.is_ok());
        // assert_matches!(
        //     &game_state,
        //     GameState {
        //         last_turn: Some(12),
        //         outcome: None,
        //         draw_pile,
        //         ..
        //     } if draw_pile.is_empty()
        // );

        // let action = PlayerAction::PlayCard(SlotIndex(0));
        // let effects = game_state.play(action).unwrap();
        // let result = game_state.run_effects(effects);

        // assert!(result.is_ok());
        // assert_matches!(
        //     &game_state,
        //     GameState {
        //         last_turn: Some(12),
        //         outcome: None,
        //         players,
        //         draw_pile,
        //         ..
        //     } if draw_pile.is_empty() && players[1].hand[0].is_none()
        // );

        // // last turn!
        // let action = PlayerAction::PlayCard(SlotIndex(1));
        // let effects = game_state.play(action).unwrap();
        // let result = game_state.run_effects(effects);

        // assert!(result.is_ok());
        // assert_matches!(
        //     &game_state,
        //     GameState {
        //         last_turn: Some(12),
        //         outcome: Some(GameOutcome::Fail { score: _ }),
        //         players,
        //         draw_pile,
        //         ..
        //     } if draw_pile.is_empty() && players[0].hand[1].is_none()
        // );
    }
}
