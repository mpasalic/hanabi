mod hanabi_app;
// mod lobby_app;
mod input;

use std::{
    env,
    error::Error,
    io::{stdout, Stdout},
    process::exit,
};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt,
};
use futures_util::StreamExt;
use hanabi_app::{HanabiApp, HanabiClient};
use ratatui::prelude::*;
use shared::client_logic::{ClientToServerMessage, ServerToClientMessage};
use tokio::sync::broadcast::error::TryRecvError;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};

// These type aliases are used to make the code more readable by reducing repetition of the generic
// types. They are not necessary for the functionality of the code.
type HanabiTerminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
type BoxedResult<T> = std::result::Result<T, Box<dyn Error>>;

async fn spawn_server_connection(
    host: String,
) -> BoxedResult<(
    SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
    SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
)> {
    let url = url::Url::parse(&host).unwrap();

    let (ws_stream, _) = connect_async(url).await?;
    // println!("WebSocket handshake has been successfully completed");

    let (ws_write, ws_read) = ws_stream.split();

    return Ok((ws_write, ws_read));
}

#[tokio::main]
async fn main() -> BoxedResult<()> {
    eprintln!("Starting Hanabi client");

    let args: Vec<String> = env::args().collect();

    let (host, player_name, session_id) = match args.as_slice() {
        [_, host, player_name, session_id] => {
            (host.clone(), player_name.clone(), session_id.clone())
        }
        _ => {
            println!("Invalid format: hanabi [host] [username] [session_id]");
            exit(1);
        }
    };

    let result = run_online_game(host, player_name, session_id).await;

    match result {
        Ok(_) => {
            println!("Goodbye!");
        }
        Err(err) => {
            println!("An error occurred: {:?}", err);
        }
    }

    Ok(())
}

async fn connect_to_server(
    host: String,
    player_name: String,
    session_id: String,
) -> BoxedResult<(
    tokio::sync::broadcast::Sender<ClientToServerMessage>,
    tokio::sync::broadcast::Receiver<ServerToClientMessage>,
)> {
    let (client_to_server_sender, client_to_server_receiver) =
        tokio::sync::broadcast::channel::<ClientToServerMessage>(16);
    let (server_to_client_sender, server_to_client_receiver) =
        tokio::sync::broadcast::channel::<ServerToClientMessage>(16);

    {
        let read_tx = server_to_client_sender.clone();
        tokio::spawn(async move {
            loop {
                eprintln!("Reconnecting!!!");
                let (mut send_server, mut from_server): (_, _) =
                    spawn_server_connection(host.clone()).await.unwrap();

                send_server
                    .send(Message::Text(
                        serde_json::to_string(&ClientToServerMessage::Join {
                            player_name: player_name.clone(),
                            session_id: session_id.clone(),
                        })
                        .unwrap(),
                    ))
                    .await
                    .unwrap();

                let from_server_handle = {
                    let read_tx: tokio::sync::broadcast::Sender<ServerToClientMessage> =
                        read_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            let message = from_server.next().await.unwrap().unwrap();
                            match message {
                                Message::Text(text) => {
                                    read_tx.send(serde_json::from_str(&text).unwrap()).unwrap();
                                }
                                _ => {}
                            }
                        }
                    })
                };

                let to_server_handle = {
                    let mut write_rx = client_to_server_receiver.resubscribe();
                    tokio::spawn(async move {
                        loop {
                            let message = write_rx.recv().await.unwrap();
                            send_server
                                .send(Message::Text(serde_json::to_string(&message).unwrap()))
                                .await
                                .unwrap();

                            send_server.close().await.unwrap();
                        }
                    })
                };

                tokio::select! {
                    _ = from_server_handle => {
                        eprintln!("from_server_handle thread died!");
                    }
                    _ = to_server_handle => {
                        eprintln!("to_server_handle thread died!");
                    }
                }
            }
        });
    }

    Ok((client_to_server_sender, server_to_client_receiver))
}

async fn run_online_game(host: String, player_name: String, session_id: String) -> BoxedResult<()> {
    loop {
        let connection: (
            tokio::sync::broadcast::Sender<ClientToServerMessage>,
            tokio::sync::broadcast::Receiver<ServerToClientMessage>,
        ) = connect_to_server(host.clone(), player_name.clone(), session_id.clone()).await?;

        let mut terminal = setup_terminal()?;
        let result = run(&mut terminal, connection).await;
        restore_terminal(terminal)?;

        match result {
            Ok(_) => {
                break;
            }
            Err(err) => {
                println!("An error occurred: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

async fn run<T>(
    terminal: &mut Terminal<T>,
    connection: (
        tokio::sync::broadcast::Sender<ClientToServerMessage>,
        tokio::sync::broadcast::Receiver<ServerToClientMessage>,
    ),
) -> BoxedResult<()>
where
    T: ratatui::backend::Backend,
{
    let (send_to_server, mut receive_from_server) = connection;

    let mut current_game_state = HanabiClient::Connecting;

    let mut app = HanabiApp::new(current_game_state.clone());

    loop {
        app.draw(terminal)?;
        let result = app.handle_events()?;

        match result {
            hanabi_app::EventHandlerResult::PlayerAction(action) => {
                send_to_server.send(ClientToServerMessage::PlayerAction { action })?;
            }
            hanabi_app::EventHandlerResult::Start => {
                send_to_server.send(ClientToServerMessage::StartGame)?;
            }
            hanabi_app::EventHandlerResult::Quit => {
                break;
            }
            hanabi_app::EventHandlerResult::Continue => {}
        }

        let message = receive_from_server.try_recv();

        current_game_state = match message {
            Ok(message) => match message {
                ServerToClientMessage::UpdatedGameState(game_state) => {
                    HanabiClient::Loaded(game_state)
                }
            },
            Err(e @ TryRecvError::Closed) => {
                return Err(Box::new(e));
            }
            Err(TryRecvError::Empty) => current_game_state,
            _ => current_game_state,
        };

        app.update(current_game_state.clone());
    }
    Ok(())
}

fn setup_terminal() -> BoxedResult<HanabiTerminal> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: HanabiTerminal) -> BoxedResult<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
