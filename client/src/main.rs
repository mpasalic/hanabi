mod hanabi_app;
// mod lobby_app;
mod input;

use std::{
    collections::HashMap,
    env,
    error::Error,
    io::{self, stdout, BufWriter, Read, Stdout, Write},
    iter,
    ops::ControlFlow,
    process::exit,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{self, Duration, SystemTime},
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt,
};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt};
use hanabi_app::{HanabiApp, HanabiClient};
use input::AppInput;
use mio::net::{SocketAddr, TcpListener};
use ratatui::prelude::*;
use shared::{
    client_logic::{ClientToServerMessage, GameLog, ServerToClientMessage},
    model::{ClientPlayerView, GameConfig, GameState, GameStateSnapshot, PlayerIndex},
};
use std::net::TcpStream;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::broadcast::error::TryRecvError,
};
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

    let (write, read) = ws_stream.split();

    return Ok((write, read));
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

// fn run(mut terminal: &Terminal) -> BoxedResult<()> {
//     let mut input_app = AppInput::default();
//     loop {
//         match input_app.run_app(terminal)? {
//             ControlFlow::Break(_) => break,
//             ControlFlow::Continue(_) => continue,
//         }
//     }
// }

async fn connect_to_server(
    host: String,
    player_name: String,
    session_id: String,
) -> BoxedResult<(
    tokio::sync::broadcast::Sender<ClientToServerMessage>,
    tokio::sync::broadcast::Receiver<ServerToClientMessage>,
)> {
    let (write_tx, mut write_rx) = tokio::sync::broadcast::channel::<ClientToServerMessage>(16);
    let (read_tx, read_rx) = tokio::sync::broadcast::channel::<ServerToClientMessage>(16);

    {
        let read_tx = read_tx.clone();
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
                    let mut write_rx = write_rx.resubscribe();
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

    Ok((write_tx, read_rx))
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
    let (mut write_tx, mut read_rx) = connection;

    let mut current_player = PlayerIndex(0);
    let mut current_game_state = HanabiClient::Connecting;

    let mut app = HanabiApp::new(current_game_state.clone());

    loop {
        app.draw(terminal)?;
        let result = app.handle_events()?;

        match result {
            hanabi_app::EventHandlerResult::PlayerAction(action) => {
                write_tx.send(ClientToServerMessage::PlayerAction {
                    player_index: current_player,
                    action,
                })?;
            }
            hanabi_app::EventHandlerResult::Start => {
                write_tx.send(ClientToServerMessage::StartGame)?;
            }
            hanabi_app::EventHandlerResult::Quit => {
                break;
            }
            hanabi_app::EventHandlerResult::Continue => {}
        }

        let message = read_rx.try_recv();

        current_game_state = match message {
            Ok(message) => match message {
                ServerToClientMessage::PlayerJoined { players } => current_game_state,
                ServerToClientMessage::GameStarted {
                    player_index,
                    game_state,
                } => current_game_state,
                ServerToClientMessage::UpdatedGameState(game_state) => {
                    HanabiClient::Loaded(game_state)
                }
                ServerToClientMessage::Pong(_) => current_game_state,
            },
            Err(e @ TryRecvError::Closed) => {
                return Err(Box::new(e));
            }
            Err(TryRecvError::Empty) => current_game_state,
            _ => current_game_state,
        };

        // current_game_state = match (current_game_state.clone(), message) {
        //     (
        //         HanabiGame::Lobby { players, .. },
        //         Ok(ServerToClientMessage::GameStarted {
        //             game_state,
        //             player_index,
        //         }),
        //     ) => {
        //         current_player = player_index;
        //         HanabiGame::Started {
        //             game_state,
        //             players,
        //         }
        //     }
        //     (
        //         HanabiGame::Started { players, .. },
        //         Ok(ServerToClientMessage::UpdatedGameState(game_state)),
        //     ) => HanabiGame::Started {
        //         game_state,
        //         players,
        //     },
        //     (
        //         HanabiGame::Lobby { log, .. } | HanabiGame::Connecting { log, .. },
        //         Ok(ServerToClientMessage::PlayerJoined { players }),
        //     ) => HanabiGame::Lobby {
        //         players: players.clone(),
        //         log: Vec::from_iter(log.into_iter().chain(iter::once(
        //             format!("Player {} joined", players.last().unwrap().clone()).to_string(),
        //         ))),
        //     },
        //     (game, Ok(ServerToClientMessage::Pong(time))) => {
        //         app.update_connection(SystemTime::now().duration_since(time).unwrap());
        //         game
        //     }

        //     (game, Err(TryRecvError::Disconnected)) => {
        //         // TODO: handle disconnect
        //         game
        //     }
        //     (game, Err(TryRecvError::Empty)) => game,

        //     (game, msg) => {
        //         // todo warn
        //         game
        //     }
        // };

        app.update(current_game_state.clone());

        // write_tx.send(ClientToServerMessage::Ping(SystemTime::now()))?;
    }

    // let result: Result<(), Box<dyn Error>> = app.run(&mut terminal);

    // if let Err(err) = result {
    //     eprintln!("{err:?}  ");
    // }
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
