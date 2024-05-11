use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use futures::{FutureExt, SinkExt, StreamExt};
use shared::client_logic::{ClientToServerMessage, ServerToClientMessage};
use shuttle_axum::ShuttleAxum;
use tokio::sync::{broadcast, mpsc, Mutex};
use tower_http::services::ServeDir;
mod server;
use server::{LobbyClient, LobbyServer, SocketMessage};
use tokio_stream::wrappers::UnboundedReceiverStream;

struct State {
    clients_count: usize,
    client_map: HashMap<usize, LobbyClient>,
    lobby_server: LobbyServer,
    tx: broadcast::Sender<SocketMessage<Message>>,
}
type ServerState = Arc<Mutex<State>>;

#[shuttle_runtime::main]
async fn axum() -> ShuttleAxum {
    let (tx, _) = broadcast::channel::<SocketMessage<Message>>(16);

    let state = Arc::new(Mutex::new(State {
        clients_count: 0,
        client_map: HashMap::new(),
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

async fn websocket(stream: WebSocket, state: ServerState) {
    // By splitting we can send and receive at the same time.
    let (client_ws_sender, mut client_ws_rcv) = stream.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel::<ServerToClientMessage>();

    let client_rcv = UnboundedReceiverStream::new(client_rcv);

    let client_id = {
        let mut state = state.lock().await;
        let client_id = state.clients_count;
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

                println!("Sending message to {}: {}", client_id_clone, message);
                Ok(Message::Text(message))
            })
            .forward(client_ws_sender)
            .map(|result| {
                if let Err(e) = result {
                    println!("error sending websocket msg: {}", e);
                }
            }),
    );

    while let Some(result) = client_ws_rcv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                println!("error receiving message for id {}): {}", client_id, e);
                break;
            }
        };
        client_msg(client_id, msg, &state).await;
    }

    // state.lock().await.su
    // clients.lock().await.insert(uuid.clone(), new_client);

    // let (client_id, mut rx) = {
    //     let mut state = state.lock().await;
    //     let client_id = state.clients_count;
    //     let new_client = Client {
    //         client_id,
    //         sender: Some(client_sender),
    //     };

    //     (client_id, state.tx.subscribe())
    // };

    // let (client_id, mut rx) = {
    //     let mut state = state.lock().await;
    //     let client_id = state.clients_count;

    //     state.clients_count += 1;
    //     (client_id, state.tx.subscribe())
    // };

    // while let Some(result) = client_ws_rcv.next().await {
    //     let msg = match result {
    //         Ok(msg) => msg,
    //         Err(e) => {
    //             println!("error receiving message for id {}): {}", client_id, e);
    //             break;
    //         }
    //     };
    //     client_msg(client_id, msg, &state).await;
    // }

    state.lock().await.client_map.remove(&client_id);
    state.lock().await.lobby_server.disconnected(client_id);
    println!("{} disconnected", client_id);

    // This task will receive watch messages and forward it to this connected client.
    // let mut send_task = tokio::spawn(async move {
    //     loop {
    //         let message = rx.recv().await;
    //         if let Ok(message) = message {
    //             if message.socket_id == client_id {
    //                 if client_ws_sender.send(message.message).await.is_err() {
    //                     break;
    //                 }
    //             }
    //         }
    //     }
    // });

    // {
    //     let state = state.clone();
    //     // This task will receive messages from this client.
    //     let mut recv_task = tokio::spawn(async move {
    //         while let Some(Ok(Message::Text(text))) = client_ws_rcv.next().await {
    //             let client_to_server_msg: Result<ClientToServerMessage, _> =
    //                 serde_json::from_str(&text);

    //             if let Ok(client_to_server_msg) = client_to_server_msg {
    //                 let mut state = state.lock().await;

    //                 let messages = state
    //                     .lobby_server
    //                     .message_received(client_id, client_to_server_msg);

    //                 for socket_msg in messages {
    //                     let msg = Message::Text(
    //                         serde_json::to_string(&socket_msg.message).expect("json"),
    //                     );
    //                     state
    //                         .tx
    //                         .send(SocketMessage {
    //                             message: msg,
    //                             socket_id: socket_msg.socket_id,
    //                         })
    //                         .expect("channel");
    //                 }
    //             }
    //         }
    //     });
    //     // If any one of the tasks exit, abort the other.
    //     tokio::select! {
    //         _ = (&mut send_task) => recv_task.abort(),
    //         _ = (&mut recv_task) => send_task.abort(),
    //     };
}

async fn client_msg(client_id: usize, msg: Message, state: &ServerState) {
    match msg {
        Message::Text(text) => {
            println!("Got message from client {}: {}", client_id, text);

            let client_to_server_msg: Result<ClientToServerMessage, _> =
                serde_json::from_str(&text);

            if let Ok(client_to_server_msg) = client_to_server_msg {
                let mut state = state.lock().await;
                let client = state.client_map.get(&client_id).unwrap().clone();

                // handle result
                let messages = state
                    .lobby_server
                    .message_received(&client, client_to_server_msg);
            } else {
                println!("error parsing message: {:?}", client_to_server_msg);
            }
        }
        _ => {}
    }
}
