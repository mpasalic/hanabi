//! Postgres-backed hydration test. Skipped locally unless `TEST_DATABASE_URL`
//! is set; CI provides one via the Postgres service. Verifies that
//! `LobbyServer::hydrate` replays a persisted game log and produces a live
//! lobby that a reconnecting client can join.
//!
//! This is the only test in the suite that exercises real SQL — the rest run
//! against `MemDatabase`. It catches: migration breakage, sqlx row mapping,
//! the JSON encoding of `PlayerAction`, and the replay loop itself.

use std::sync::Arc;

use rand::Rng;

use server::lobby::{ClientId, LobbyClient, LobbyServer};
use server::model::{Database, PgDatabase};
use shared::client_logic::{ClientToServerMessage, HanabiGame, ServerToClientMessage};
use shared::model::{GameConfig, PlayerAction, SlotIndex};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

fn test_db_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn unique_game_id() -> String {
    // Unique per run so concurrent tests don't collide on PK.
    let suffix: u32 = rand::thread_rng().gen();
    format!("test-hydrate-{suffix:08x}")
}

#[tokio::test]
async fn hydrates_persisted_game_when_client_rejoins() {
    let Some(url) = test_db_url() else {
        eprintln!("TEST_DATABASE_URL not set; skipping Postgres hydration test");
        return;
    };

    // --- Connect and migrate ---------------------------------------------
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to test postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let game_id = unique_game_id();
    let players = vec!["alice".to_string(), "bob".to_string()];
    let config = GameConfig::new(2, /* seed */ 12345);

    // --- Seed the DB directly with a known game + a few moves ------------
    let pg: Arc<dyn Database> = Arc::new(PgDatabase::new(pool.clone()));
    pg.create_game(&game_id, &config, &players)
        .await
        .expect("seed: create_game");

    // Two valid moves: alice discards slot 0, bob discards slot 0.
    pg.save_action(
        &game_id,
        /* turn */ 0,
        PlayerAction::DiscardCard(SlotIndex(0)),
        /* player */ 0,
    )
    .await
    .expect("seed: action 0");
    pg.save_action(
        &game_id,
        /* turn */ 1,
        PlayerAction::DiscardCard(SlotIndex(0)),
        /* player */ 1,
    )
    .await
    .expect("seed: action 1");

    // --- Fresh LobbyServer pointing at the same DB -----------------------
    // The in-memory lobby map is empty, so `Join` will trigger hydrate().
    let mut server = LobbyServer::new(pg.clone());

    let (alice_tx, mut alice_rx) = mpsc::unbounded_channel();
    let alice = LobbyClient {
        client_id: ClientId(1),
        sender: alice_tx,
    };

    server
        .message_received(
            &alice,
            ClientToServerMessage::Join {
                player_name: "alice".into(),
                session_id: game_id.clone(),
            },
        )
        .await
        .expect("alice rejoins hydrated game");

    // First broadcast is the game state. There may be an `UpdatedConnectionStatus`
    // queued after it; we only care about the game state message.
    let msg = alice
        .recv_game_state(&mut alice_rx)
        .await
        .expect("alice sees hydrated game state");

    match msg {
        HanabiGame::Playing { game_state, players: lobby_players, .. } => {
            assert_eq!(lobby_players.len(), 2, "both players should be present");
            assert!(
                lobby_players.iter().any(|p| p.name == "alice"),
                "alice should be in the roster"
            );
            assert!(
                lobby_players.iter().any(|p| p.name == "bob"),
                "bob should be in the roster"
            );
            // Two discards happened, so the discard pile should have two cards.
            assert_eq!(
                game_state.discard_pile.len(),
                2,
                "discard pile should reflect 2 persisted actions"
            );
        }
        other => panic!("expected Playing after hydrate, got {other:?}"),
    }
}

trait Receive {
    async fn recv_game_state(
        &self,
        rx: &mut mpsc::UnboundedReceiver<ServerToClientMessage>,
    ) -> Option<HanabiGame>;
}

impl Receive for LobbyClient {
    /// Pull messages off the receiver until we see an `UpdatedGameState` —
    /// connection-status broadcasts are noise for this assertion.
    async fn recv_game_state(
        &self,
        rx: &mut mpsc::UnboundedReceiver<ServerToClientMessage>,
    ) -> Option<HanabiGame> {
        while let Some(msg) = rx.recv().await {
            if let ServerToClientMessage::UpdatedGameState(g) = msg {
                return Some(g);
            }
        }
        None
    }
}
