//! Axum app wiring extracted from `main.rs` so integration tests can spawn the
//! exact same router (against any `Database` impl) without re-implementing the
//! websocket plumbing.

use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{
        ws::{Message, Utf8Bytes, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use futures::{FutureExt, StreamExt};
use shared::client_logic::{ClientToServerMessage, ServerToClientMessage};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::services::ServeDir;

use crate::lobby::{ClientId, LobbyClient, LobbyError, LobbyServer};
use crate::model::Database;

pub struct ServerStateSchema {
    pub clients_count: usize,
    pub client_map: HashMap<ClientId, LobbyClient>,
    pub lobby_server: LobbyServer,
}

pub type ServerState = Arc<Mutex<ServerStateSchema>>;

/// Build the Axum router used by both production (`main.rs`) and integration
/// tests. The `Database` impl is the only seam — pass a Postgres-backed one
/// for prod, a `MemDatabase` for tests.
pub fn build_router(db: Arc<dyn Database>) -> Router {
    let state: ServerState = Arc::new(Mutex::new(ServerStateSchema {
        clients_count: 0,
        client_map: HashMap::new(),
        lobby_server: LobbyServer::new(db),
    }));

    Router::new()
        .route("/websocket", get(websocket_handler))
        .fallback_service(ServeDir::new("dist"))
        .layer(Extension(state))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: ServerState) {
    let (client_ws_sender, mut client_ws_rcv) = stream.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel::<ServerToClientMessage>();
    let client_rcv = UnboundedReceiverStream::new(client_rcv);

    let client_id = {
        let mut state = state.lock().await;
        let client_id = ClientId(state.clients_count);
        let new_client = LobbyClient {
            client_id,
            sender: client_sender,
        };
        state.client_map.insert(client_id, new_client);
        state.clients_count += 1;
        client_id
    };

    let client_id_clone = client_id;
    tokio::task::spawn(
        client_rcv
            .map(move |m| {
                let message = serde_json::to_string(&m).expect("json");
                tracing::debug!(client = ?client_id_clone, "sending message: {}", message);
                Ok(Message::Text(Utf8Bytes::from(message)))
            })
            .forward(client_ws_sender)
            .map(|result| {
                if let Err(e) = result {
                    tracing::error!("error sending websocket msg: {}", e);
                }
            }),
    );

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("error receiving for id {client_id:?}: {e}");
                break;
            }
        };
        client_msg(client_id, msg, &state).await;
    }

    let mut s = state.lock().await;
    s.client_map.remove(&client_id);
    s.lobby_server.disconnected(client_id);
    tracing::info!("{client_id:?} disconnected");
}

async fn client_msg(client_id: ClientId, msg: Message, state: &ServerState) {
    let Message::Text(text) = msg else {
        return;
    };

    let parsed: Result<ClientToServerMessage, _> = serde_json::from_str(&text);
    let Ok(message) = parsed else {
        tracing::warn!("error parsing message: {:?}", parsed);
        return;
    };

    let mut state = state.lock().await;
    let client = state.client_map.get(&client_id).unwrap().clone();
    let result = state.lobby_server.message_received(&client, message).await;

    match result {
        Err(LobbyError::InvalidState(err) | LobbyError::InvalidPlayerAction(err)) => {
            tracing::warn!("error handling message: {err}");
            let _ = client.sender.send(ServerToClientMessage::Error(err));
        }
        Err(LobbyError::Database(err)) => tracing::error!("database error: {err:?}"),
        _ => {}
    }
}
