mod hanabi_app;
// mod lobby_app;
mod input;

use std::{
    env,
    error::Error,
    io::{self, stdout, BufWriter, Read, Stdout, Write},
    iter,
    ops::ControlFlow,
    process::exit,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{self, Duration},
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
use hanabi_app::{HanabiApp, HanabiGame};
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
    sync::mpsc::error::TryRecvError,
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
    let mut terminal = setup_terminal()?;

    let result = run(&mut terminal).await;

    restore_terminal(terminal)?;

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

async fn run<T>(terminal: &mut Terminal<T>) -> BoxedResult<()>
where
    T: ratatui::backend::Backend,
{
    // let mut name: Option<String> = None;
    // let mut server: Option<String> = None;
    // let mut game_id: Option<String> = None;

    // // TODO input example
    // let mut input_app = AppInput::default();
    // input_app.messages.push("Enter your name:".to_string());

    // loop {
    //     match input_app.run_app(terminal)? {
    //         ControlFlow::Break(Some(result)) => {
    //             if name.is_none() {
    //                 name = Some(result);
    //             }
    //         }
    //         ControlFlow::Continue(_) => continue,
    //         ControlFlow::Break(None) => break,
    //     }
    // }

    let args: Vec<String> = env::args().collect();

    let (mut send_server, mut from_server) = match args.as_slice() {
        [_, host, player_name, session_id] => {
            // println!(
            //     "Connect to {} (session: {}) as {}...",
            //     host, session_id, player_name
            // );
            let (mut send_server, from_server): (_, _) =
                spawn_server_connection(host.clone()).await?;

            send_server
                .send(Message::Text(serde_json::to_string(
                    &ClientToServerMessage::Join {
                        player_name: player_name.clone(),
                        session_id: session_id.clone(),
                    },
                )?))
                .await?;

            (send_server, from_server)
        }
        _ => {
            return Err("Invalid format: hanabi [host] [username] [session_id]".into());
        }
    };

    // println!("Connected! Waiting for players... press 's' to start the game!");

    // loop {
    //     let message = rx.recv();

    //     match message {
    //         Ok(Messages::ServerMessage(ServerToClientMessage::GameStarted {
    //             game_state, ..
    //         })) => {
    //             initial_game_state = Some(game_state);
    //             break;
    //         }
    //         Ok(Messages::ServerMessage(ServerToClientMessage::PlayerJoined { players })) => {
    //             println!("Current players: {}", players.join(", "));
    //         }
    //         Ok(Messages::UserInput(msg)) => {
    //             if msg == "start\n" {
    //                 server.send_message(&OwnedMessage::Text(serde_json::to_string(
    //                     &ClientToServerMessage::StartGame,
    //                 )?))?;
    //             }
    //         }
    //         Err(_) => todo!(),
    //         _ => {}
    //     }
    // }

    let mut current_player = PlayerIndex(0);
    let mut current_game_state = HanabiGame::Connecting {
        log: vec!["Connecting...".to_string()],
    };

    let mut app = HanabiApp::new(current_game_state.clone());

    let (write_tx, mut write_rx) = tokio::sync::mpsc::unbounded_channel::<ClientToServerMessage>();
    tokio::spawn(async move {
        loop {
            let message = write_rx.recv().await.unwrap();
            send_server
                .send(Message::Text(serde_json::to_string(&message).unwrap()))
                .await
                .unwrap();
        }
    });

    let (read_tx, mut read_rx) = tokio::sync::mpsc::unbounded_channel::<ServerToClientMessage>();
    tokio::spawn(async move {
        loop {
            let message = from_server.next().await;
            match message {
                Some(Ok(Message::Text(text))) => {
                    read_tx.send(serde_json::from_str(&text).unwrap()).unwrap();
                }
                _ => {}
            }
        }
    });

    loop {
        app.run(terminal)?;
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
        current_game_state = match (current_game_state.clone(), message) {
            (
                HanabiGame::Lobby { players, .. },
                Ok(ServerToClientMessage::GameStarted {
                    game_state,
                    player_index,
                }),
            ) => {
                current_player = player_index;
                HanabiGame::Started {
                    game_state,
                    players,
                }
            }
            (
                HanabiGame::Started { players, .. },
                Ok(ServerToClientMessage::UpdatedGameState(game_state)),
            ) => HanabiGame::Started {
                game_state,
                players,
            },
            (
                HanabiGame::Lobby { log, .. } | HanabiGame::Connecting { log, .. },
                Ok(ServerToClientMessage::PlayerJoined { players }),
            ) => HanabiGame::Lobby {
                players: players.clone(),
                log: Vec::from_iter(log.into_iter().chain(iter::once(
                    format!("Player {} joined", players.last().unwrap().clone()).to_string(),
                ))),
            },

            (game, Err(TryRecvError::Disconnected)) => {
                // TODO: handle disconnect
                game
            }
            (game, Err(TryRecvError::Empty)) => game,

            (game, msg) => {
                // todo warn
                game
            }
        };

        app.update(current_game_state.clone());
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
