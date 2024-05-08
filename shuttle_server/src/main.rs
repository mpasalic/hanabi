use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use futures::{SinkExt, StreamExt};
use shared::client_logic::ClientToServerMessage;
use shuttle_axum::ShuttleAxum;
use tokio::sync::{broadcast, Mutex};
use tower_http::services::ServeDir;
mod server;

use server::{LobbyServer, SocketMessage};

struct State {
    clients_count: usize,
    lobby_server: LobbyServer,
    tx: broadcast::Sender<SocketMessage<Message>>,
}

#[shuttle_runtime::main]
async fn axum() -> ShuttleAxum {
    let (tx, _) = broadcast::channel::<SocketMessage<Message>>(16);

    let state = Arc::new(Mutex::new(State {
        clients_count: 0,
        lobby_server: LobbyServer::new(),
        tx,
    }));

    let router = Router::new()
        .route("/websocket", get(websocket_handler))
        .nest_service("/", ServeDir::new("static"))
        .layer(Extension(state));

    Ok(router.into())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<Mutex<State>>) {
    // By splitting we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    let (client_id, mut rx) = {
        let mut state = state.lock().await;
        let client_id = state.clients_count;

        state.clients_count += 1;
        (client_id, state.tx.subscribe())
    };

    // This task will receive watch messages and forward it to this connected client.
    let mut send_task = tokio::spawn(async move {
        loop {
            let message = rx.recv().await;
            if let Ok(message) = message {
                if message.socket_id == client_id {
                    if sender.send(message.message).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    {
        let state = state.clone();
        // This task will receive messages from this client.
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                let client_to_server_msg: Result<ClientToServerMessage, _> =
                    serde_json::from_str(&text);

                if let Ok(client_to_server_msg) = client_to_server_msg {
                    let mut state = state.lock().await;

                    let messages = state
                        .lobby_server
                        .message_received(client_id, client_to_server_msg);

                    for socket_msg in messages {
                        let msg = Message::Text(
                            serde_json::to_string(&socket_msg.message).expect("json"),
                        );
                        state
                            .tx
                            .send(SocketMessage {
                                message: msg,
                                socket_id: socket_msg.socket_id,
                            })
                            .expect("channel");
                    }
                }
            }
        });
        // If any one of the tasks exit, abort the other.
        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        };
    }
}
