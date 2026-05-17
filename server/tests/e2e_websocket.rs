//! True end-to-end test: spawns the real Axum router on a random localhost
//! port, connects via `tokio-tungstenite`, and drives the full JSON wire
//! protocol. Uses the in-memory `MemDatabase` so the test stays hermetic.
//!
//! This is the slow / high-confidence test layer. It catches anything that
//! the in-process `LobbyServer` test in `tests/lobby.rs` would miss: JSON
//! serialization of `ClientToServerMessage` / `ServerToClientMessage`, the
//! websocket upgrade handshake, async stream forwarding, and the full
//! connection lifecycle. Add scenarios by copying this one.

mod support;

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use server::app::build_router;
use server::model::Database;
use shared::client_logic::{ClientToServerMessage, HanabiGame, ServerToClientMessage};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use support::mem_db::MemDatabase;

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Spawn the production router on an ephemeral port. Returns the bound port
/// and a `JoinHandle` so the test can abort the server when finished.
async fn spawn_server() -> (u16, tokio::task::JoinHandle<()>) {
    let db: Arc<dyn Database> = Arc::new(MemDatabase::new());
    let app = build_router(db);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (port, handle)
}

async fn connect(port: u16) -> WsStream {
    let url = format!("ws://127.0.0.1:{port}/websocket");
    let (ws, _resp) = connect_async(&url).await.expect("ws connect");
    ws
}

async fn send_msg(ws: &mut WsStream, msg: ClientToServerMessage) {
    let json = serde_json::to_string(&msg).unwrap();
    ws.send(Message::Text(json.into())).await.expect("ws send");
}

/// Pull the next decoded `ServerToClientMessage` off the socket, with a short
/// timeout so we get a useful failure instead of hanging forever.
async fn recv_msg(ws: &mut WsStream) -> ServerToClientMessage {
    let frame = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("timed out waiting for ws message")
        .expect("ws closed")
        .expect("ws frame error");
    match frame {
        Message::Text(text) => serde_json::from_str(&text).expect("decode S2C"),
        other => panic!("unexpected non-text frame: {other:?}"),
    }
}

#[tokio::test]
async fn two_players_create_join_and_start() {
    let (port, server) = spawn_server().await;

    // --- Alice creates the game ------------------------------------------
    let mut alice = connect(port).await;
    send_msg(
        &mut alice,
        ClientToServerMessage::CreateGame {
            player_name: "alice".into(),
        },
    )
    .await;

    let session_id = match recv_msg(&mut alice).await {
        ServerToClientMessage::CreatedGame { session_id } => session_id,
        other => panic!("expected CreatedGame, got {other:?}"),
    };
    assert!(!session_id.is_empty());

    // --- Bob joins via the same session id -------------------------------
    let mut bob = connect(port).await;
    send_msg(
        &mut bob,
        ClientToServerMessage::Join {
            player_name: "bob".into(),
            session_id: session_id.clone(),
        },
    )
    .await;

    // Both clients should see a Lobby broadcast containing both players.
    // The order of game-state vs. connection-status broadcasts is not fixed;
    // pull until we find the game-state one.
    let alice_lobby = drain_until_game_state(&mut alice).await;
    let bob_lobby = drain_until_game_state(&mut bob).await;

    for (who, game) in [("alice", &alice_lobby), ("bob", &bob_lobby)] {
        match game {
            HanabiGame::Lobby { players, .. } => {
                assert_eq!(players.len(), 2, "{who} sees both players");
                assert!(players.iter().any(|p| p.name == "alice"));
                assert!(players.iter().any(|p| p.name == "bob"));
            }
            other => panic!("{who} got unexpected game state: {other:?}"),
        }
    }

    // --- Alice starts the game -------------------------------------------
    send_msg(&mut alice, ClientToServerMessage::StartGame).await;

    let alice_playing = drain_until_game_state(&mut alice).await;
    let bob_playing = drain_until_game_state(&mut bob).await;
    assert!(
        matches!(alice_playing, HanabiGame::Playing { .. }),
        "alice should see Playing after StartGame"
    );
    assert!(
        matches!(bob_playing, HanabiGame::Playing { .. }),
        "bob should see Playing after StartGame"
    );

    server.abort();
}

/// Skip over `UpdatedConnectionStatus`/`Error` frames until we find the next
/// `UpdatedGameState`. Keeps the assertion code free of broadcast-order
/// brittleness.
async fn drain_until_game_state(ws: &mut WsStream) -> HanabiGame {
    loop {
        match recv_msg(ws).await {
            ServerToClientMessage::UpdatedGameState(g) => return g,
            ServerToClientMessage::UpdatedConnectionStatus { .. } => continue,
            ServerToClientMessage::CreatedGame { .. } => continue,
            ServerToClientMessage::Error(e) => panic!("unexpected server Error: {e}"),
        }
    }
}
