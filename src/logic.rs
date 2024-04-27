use itertools::Itertools;
use rand::Rng;
use strum::IntoEnumIterator;
mod logic_tests;

use crate::model::{
    Card, CardFace, CardSuit, GameEffect, GameLog, GameOutcome, GameState, Hint, HintAction,
    PlayedCardResult, Player, PlayerAction, PlayerIndex, Slot, SlotIndex,
};

impl GameLog {
    pub fn new(num_players: usize, hand_size: usize) -> Self {
        GameLog {
            actions: Vec::new(),
            num_players,
            hand_size,
        }
    }

    pub fn log(&mut self, action: PlayerAction) {
        self.actions.push(action);
    }

    pub fn undo(&mut self) {
        self.actions.pop();
    }

    pub fn generate_state(&self) -> Result<GameState, String> {
        let mut game = GameState::start(self.num_players, self.hand_size)?;
        for action in self.actions.iter() {
            let effects = game.play(action.clone()).unwrap();
            game.run_effects(effects).unwrap();
        }
        return Ok(game);
    }
}

impl GameState {
    pub fn start(num_players: usize, num_cards: usize) -> Result<GameState, String> {
        let mut game = GameState {
            draw_pile: new_ordered_deck(),
            discard_pile: Vec::new(),
            last_turn: None,
            played_cards: Vec::new(),
            players: (0..num_players)
                .into_iter()
                .map(|_index| Player {
                    hand: (0..num_cards).map(|_slot_index| None).collect_vec(),
                })
                .collect(),
            remaining_bomb_count: 3,
            remaining_hint_count: 8,
            turn: 0,
        };

        use GameEffect::*;
        let init_effects = (0..4)
            .flat_map(move |slot_index| {
                (0..num_players).map(move |player_index| {
                    DrawCard(PlayerIndex(player_index), SlotIndex(slot_index))
                })
            })
            .collect();
        game.run_effects(init_effects)?;
        return Ok(game);
    }

    // precondition: assumes the the action was taken by the current player
    pub fn play(&self, action: PlayerAction) -> Result<Vec<GameEffect>, String> {
        use GameEffect::*;
        let player_index = PlayerIndex(self.turn as usize % self.players.len());

        match action {
            PlayerAction::PlayCard(SlotIndex(index), card) => {
                let play_result = self.check_play(&card);

                match play_result {
                    PlayedCardResult::Accepted => {
                        return Ok(vec![
                            RemoveCard(player_index, SlotIndex(index)),
                            PlaceOnBoard(card),
                            DrawCard(player_index, SlotIndex(index)),
                            NextTurn,
                        ]);
                    }
                    PlayedCardResult::CompletedSet => {
                        return Ok(vec![
                            RemoveCard(player_index, SlotIndex(index)),
                            PlaceOnBoard(card),
                            DrawCard(player_index, SlotIndex(index)),
                            IncHint,
                            NextTurn,
                        ]);
                    }
                    PlayedCardResult::Rejected => {
                        return Ok(vec![
                            RemoveCard(player_index, SlotIndex(index)),
                            AddToDiscrard(card),
                            DrawCard(player_index, SlotIndex(index)),
                            BurnFuse,
                            NextTurn,
                        ]);
                    }
                }
            }
            PlayerAction::DiscardCard(SlotIndex(index), card) => {
                return Ok(vec![
                    RemoveCard(player_index, SlotIndex(index)),
                    AddToDiscrard(card),
                    DrawCard(player_index, SlotIndex(index)),
                    IncHint,
                    NextTurn,
                ]);
            }
            PlayerAction::GiveHint(PlayerIndex(hinted_player_index), slots, hint_action) => {
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
                        if let (index, Some(_)) = value {
                            let slot_index = SlotIndex(index);
                            let slot_hinted = slots.contains(&slot_index);

                            // TODO probably should check for conflicts?
                            match (slot_hinted, hint_action) {
                                (true, SameFace(face_hint)) => Some(HintCard(
                                    PlayerIndex(hinted_player_index),
                                    slot_index,
                                    Hint::IsFace(face_hint),
                                )),
                                (true, SameSuit(suit_hint)) => Some(HintCard(
                                    PlayerIndex(hinted_player_index),
                                    slot_index,
                                    Hint::IsSuit(suit_hint),
                                )),
                                (false, SameFace(face_hint)) => Some(HintCard(
                                    PlayerIndex(hinted_player_index),
                                    slot_index,
                                    Hint::IsNotFace(face_hint),
                                )),
                                (false, SameSuit(suit_hint)) => Some(HintCard(
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
                let hinted_effects = vec![DecHint, NextTurn];

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
            (current_turn, Some(last_turn), _, _) if current_turn == last_turn => {
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

    pub fn run_effect(&mut self, effect: GameEffect) -> Result<Option<GameOutcome>, String> {
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
                            card,
                            hints: Vec::new(),
                        });
                } else {
                    self.last_turn = Some(self.turn);
                }
            }
            GameEffect::RemoveCard(PlayerIndex(player_index), SlotIndex(slot_index)) => {
                let player = self
                    .players
                    .get_mut(player_index)
                    .ok_or_else(|| "Invalid current player")?;
                player.hand.get(slot_index).take();
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
            GameEffect::NextTurn => {
                self.turn = self.turn + 1;
            }
        }
        return Ok(self.check_game_outcome());
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

pub fn new_standard_deck() -> Vec<Card> {
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
        let swap = rand::thread_rng().gen_range(index..deck.len());
        deck.swap(index, swap);
    }
    return deck;
}

pub fn new_ordered_deck() -> Vec<usize> {
    let standard_deck = new_standard_deck();
    let mut ordered_deck = vec![];
    for index in 0..standard_deck.len() {
        ordered_deck.insert(0, index)
    }
    return ordered_deck;
}
