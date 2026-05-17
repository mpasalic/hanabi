//! In-process integration tests for `LobbyServer`. Drives `message_received`
//! directly via mpsc channels and an in-memory `Database` impl — no websocket,
//! no Postgres, no Docker. This is the canonical pattern for testing new
//! server-side message flows.

mod support;

use std::sync::Arc;

use server::lobby::{ClientId, LobbyClient, LobbyServer};
use shared::client_logic::{ClientToServerMessage, HanabiGame, ServerToClientMessage};
use tokio::sync::mpsc;

use support::mem_db::MemDatabase;

struct FakeClient {
    handle: LobbyClient,
    rx: mpsc::UnboundedReceiver<ServerToClientMessage>,
}

fn fake_client(id: usize) -> FakeClient {
    let (tx, rx) = mpsc::unbounded_channel();
    FakeClient {
        handle: LobbyClient {
            client_id: ClientId(id),
            sender: tx,
        },
        rx,
    }
}

#[tokio::test]
async fn create_game_returns_session_id() {
    let db: Arc<dyn server::model::Database> = Arc::new(MemDatabase::new());
    let mut server = LobbyServer::new(db);

    let mut alice = fake_client(1);
    server
        .message_received(
            &alice.handle,
            ClientToServerMessage::CreateGame {
                player_name: "alice".into(),
            },
        )
        .await
        .expect("create game");

    let msg = alice.rx.recv().await.expect("alice receives CreatedGame");
    match msg {
        ServerToClientMessage::CreatedGame { session_id } => {
            assert!(!session_id.is_empty(), "session id should be non-empty");
        }
        other => panic!("expected CreatedGame, got {other:?}"),
    }
}

#[tokio::test]
async fn second_player_joining_broadcasts_lobby_to_both() {
    let db: Arc<dyn server::model::Database> = Arc::new(MemDatabase::new());
    let mut server = LobbyServer::new(db);

    // --- Alice creates the game ------------------------------------------
    let mut alice = fake_client(1);
    server
        .message_received(
            &alice.handle,
            ClientToServerMessage::CreateGame {
                player_name: "alice".into(),
            },
        )
        .await
        .expect("create");

    let session_id = match alice.rx.recv().await.unwrap() {
        ServerToClientMessage::CreatedGame { session_id } => session_id,
        other => panic!("expected CreatedGame, got {other:?}"),
    };

    // --- Bob joins -------------------------------------------------------
    let mut bob = fake_client(2);
    server
        .message_received(
            &bob.handle,
            ClientToServerMessage::Join {
                player_name: "bob".into(),
                session_id: session_id.clone(),
            },
        )
        .await
        .expect("join");

    // --- Both see an UpdatedGameState(Lobby) with two players ------------
    let alice_msg = alice.rx.recv().await.expect("alice sees join");
    let bob_msg = bob.rx.recv().await.expect("bob sees join");

    for (who, msg) in [("alice", &alice_msg), ("bob", &bob_msg)] {
        match msg {
            ServerToClientMessage::UpdatedGameState(HanabiGame::Lobby { players, .. }) => {
                assert_eq!(
                    players.len(),
                    2,
                    "{who} should see 2 players in the lobby"
                );
                assert!(players.iter().any(|p| p.name == "alice"));
                assert!(players.iter().any(|p| p.name == "bob"));
            }
            other => panic!("{who} got unexpected message: {other:?}"),
        }
    }
}
