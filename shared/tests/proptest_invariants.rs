//! Property-based tests for the game engine. Generates random valid-ish
//! action sequences and asserts that core invariants hold at every step.
//! Skips invalid actions (the engine returns `Err` and we move on), so the
//! generator can be coarse — proptest will still find action sequences that
//! reach interesting states.
//!
//! Add invariants here as we find them — this is the place to encode "this
//! must never happen" rules about game state.

use proptest::prelude::*;
use rand::rngs::StdRng;

use shared::client_logic::GameLog;
use shared::model::{
    CardFace, CardSuit, GameConfig, HintAction, PlayerAction, PlayerIndex, SlotIndex,
};

const HAND_SIZE: usize = 5;
const NUM_PLAYERS: usize = 2;

fn arb_slot() -> impl Strategy<Value = SlotIndex> {
    (0..HAND_SIZE).prop_map(SlotIndex)
}

fn arb_player() -> impl Strategy<Value = PlayerIndex> {
    (0..NUM_PLAYERS).prop_map(PlayerIndex)
}

fn arb_suit() -> impl Strategy<Value = CardSuit> {
    prop_oneof![
        Just(CardSuit::Red),
        Just(CardSuit::Green),
        Just(CardSuit::Yellow),
        Just(CardSuit::White),
        Just(CardSuit::Blue),
    ]
}

fn arb_face() -> impl Strategy<Value = CardFace> {
    prop_oneof![
        Just(CardFace::One),
        Just(CardFace::Two),
        Just(CardFace::Three),
        Just(CardFace::Four),
        Just(CardFace::Five),
    ]
}

fn arb_hint() -> impl Strategy<Value = HintAction> {
    prop_oneof![
        arb_suit().prop_map(HintAction::SameSuit),
        arb_face().prop_map(HintAction::SameFace),
    ]
}

fn arb_action() -> impl Strategy<Value = PlayerAction> {
    prop_oneof![
        arb_slot().prop_map(PlayerAction::PlayCard),
        arb_slot().prop_map(PlayerAction::DiscardCard),
        (arb_player(), arb_hint()).prop_map(|(p, h)| PlayerAction::GiveHint(p, h)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn invariants_hold_through_random_action_sequence(
        seed in any::<u64>(),
        actions in prop::collection::vec(arb_action(), 0..40),
    ) {
        let config = GameConfig::new(NUM_PLAYERS, seed);
        let max_fuses = config.num_fuses;
        let mut log = GameLog::new::<StdRng>(config);

        for action in actions {
            let state_before = log.current_game_state();
            // If the game has ended, there's nothing more to test.
            if state_before.outcome.is_some() {
                break;
            }

            let actor = state_before.current_player_index();
            // Invalid actions are expected (random generator) — just skip them.
            let _ = log.log(actor, action);

            let s = log.current_game_state();

            // Bombs only ever decrement — never above the initial fuse count.
            prop_assert!(
                s.remaining_bomb_count <= max_fuses,
                "fuse count {} exceeded initial {}", s.remaining_bomb_count, max_fuses
            );

            // At most 5 suits × 5 faces = 25 plays.
            prop_assert!(
                s.played_cards.len() <= 25,
                "played pile grew past 25: {}", s.played_cards.len()
            );

            // Per suit, the played pile must form a strict 1..=N progression
            // (each face appears at most once, no gaps).
            for suit in [CardSuit::Red, CardSuit::Green, CardSuit::Yellow, CardSuit::White, CardSuit::Blue] {
                let mut faces: Vec<_> = s
                    .played_cards
                    .iter()
                    .filter(|c| c.suit == suit)
                    .map(|c| c.face)
                    .collect();
                faces.sort();
                for (i, face) in faces.iter().enumerate() {
                    let expected = match i {
                        0 => CardFace::One,
                        1 => CardFace::Two,
                        2 => CardFace::Three,
                        3 => CardFace::Four,
                        4 => CardFace::Five,
                        _ => unreachable!(),
                    };
                    prop_assert_eq!(
                        *face,
                        expected,
                        "suit {:?} played out of order: {:?}", suit, faces
                    );
                }
            }

            // Win outcome implies a complete board.
            if let Some(shared::model::GameOutcome::Win) = s.outcome {
                prop_assert_eq!(s.played_cards.len(), 25, "Win outcome must mean 25 cards played");
            }
        }
    }
}
