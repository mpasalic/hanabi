//! Cross-checks `GameStateSnapshot::apply_local_mutation` (client-side
//! optimistic update) against the authoritative `shared::logic` engine.
//!
//! The client applies these mutations *before* round-tripping to the server so
//! the UI doesn't flicker. If the two ever drift, the player sees their card
//! snap back into a different position. Today only `PlayerAction::MoveSlot`
//! triggers a local mutation; this test is the template for verifying any new
//! variant we add.

use rand::rngs::StdRng;

use shared::client_logic::GameLog;
use shared::model::{ClientPlayerView, GameConfig, HiddenSlot, PlayerAction, PlayerIndex, SlotIndex};

/// Two-player fixture; deterministic deck so the test is hermetic.
fn fresh_log() -> GameLog {
    let config = GameConfig::new(2, /* seed */ 42);
    GameLog::new::<StdRng>(config)
}

fn me_hand(snapshot: &shared::model::GameStateSnapshot) -> &Vec<Option<HiddenSlot>> {
    snapshot
        .players
        .iter()
        .find_map(|p| match p {
            ClientPlayerView::Me { hand, .. } => Some(hand),
            _ => None,
        })
        .expect("snapshot must have a Me view")
}

fn draw_numbers(hand: &[Option<HiddenSlot>]) -> Vec<Option<usize>> {
    hand.iter()
        .map(|slot| slot.as_ref().map(|s| s.draw_number))
        .collect()
}

#[test]
fn move_slot_optimistic_matches_server_truth() {
    let names = vec!["alice".to_string(), "bob".to_string()];
    let me = PlayerIndex(0);

    // --- Build the shared starting state, project to a client snapshot ----
    let log = fresh_log();
    let initial_state = log.current_game_state();
    let mut client_snapshot = log.into_client_game_state(initial_state, me, names.clone());

    let before = draw_numbers(me_hand(&client_snapshot));

    // --- Client-side optimistic mutation -----------------------------------
    let action = PlayerAction::MoveSlot(me, SlotIndex(0), SlotIndex(3));
    client_snapshot.apply_local_mutation(action);
    let optimistic = draw_numbers(me_hand(&client_snapshot));

    // --- Server-side authoritative result ----------------------------------
    let mut server_log = fresh_log();
    server_log.log(me, action).expect("server move_slot");
    let server_snapshot =
        server_log.into_client_game_state(server_log.current_game_state(), me, names);
    let authoritative = draw_numbers(me_hand(&server_snapshot));

    // --- The two should agree on the new slot ordering ---------------------
    assert_ne!(before, optimistic, "MoveSlot should change the slot order");
    assert_eq!(
        optimistic, authoritative,
        "optimistic mutation diverged from server logic"
    );
}

#[test]
fn move_slot_optimistic_left_and_right_both_match_server() {
    // Verify the rotate-left and rotate-right branches inside
    // apply_local_mutation both stay in sync. Forward move (lower → higher)
    // hits rotate_left; backward move hits rotate_right.
    let names = vec!["alice".to_string(), "bob".to_string()];
    let me = PlayerIndex(0);

    for (from, to) in [(0usize, 4usize), (4, 0), (1, 2)] {
        let action = PlayerAction::MoveSlot(me, SlotIndex(from), SlotIndex(to));

        // Optimistic
        let log = fresh_log();
        let mut client_snapshot =
            log.into_client_game_state(log.current_game_state(), me, names.clone());
        client_snapshot.apply_local_mutation(action);
        let optimistic = draw_numbers(me_hand(&client_snapshot));

        // Authoritative
        let mut server_log = fresh_log();
        server_log.log(me, action).expect("server move_slot");
        let server_snapshot = server_log.into_client_game_state(
            server_log.current_game_state(),
            me,
            names.clone(),
        );
        let authoritative = draw_numbers(me_hand(&server_snapshot));

        assert_eq!(
            optimistic, authoritative,
            "drift for MoveSlot({from} -> {to})"
        );
    }
}
