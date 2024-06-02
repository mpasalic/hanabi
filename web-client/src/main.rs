use std::sync::mpsc::{self, Sender};

use eframe::set_value;
use egui::FontFamily;
use egui::FontId;
use egui::OpenUrl;
use egui::Pos2;
use ratatui::layout::Position;
use ratatui::prelude::Terminal;
use ratatui_app::hanabi_app::*;
use ratatui_app::input_app::AppInput;
use ratatui_app::input_app::InputMode;
use ratatui_app::key_code::KeyCode;
use ratframe::NewCC;
use ratframe::RataguiBackend;
use shared::client_logic::*;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};
mod input;

use input::key_code_to_char;

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[cfg(not(target_arch = "wasm32"))]
use ratframe::native_setup;

#[cfg(target_arch = "wasm32")]
use ratframe::wasm_setup;

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_setup(HelloApp::default());
}
// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    native_setup(HelloApp::default())
}

pub struct HelloApp {
    terminal: Terminal<RataguiBackend>,
    tui_state: TuiState,
    send_to_server: mpsc::Sender<ClientToServerMessage>,
    send_to_server_queue: mpsc::Receiver<ClientToServerMessage>,
    read_from_server: mpsc::Receiver<ServerToClientMessage>,
    server_to_client_sender: Sender<ServerToClientMessage>,

    websocket: Option<WebSocket>,
    web_url: String,

    cursor: egui::CursorIcon,
}

pub enum TuiState {
    AppInput(AppInput),
    CreatingGame {
        player_name: String,
        server_address: String,
    },
    HanabiApp {
        hanabi_app: HanabiApp,
        player_name: String,
        session_id: String,
        server_address: String,
    },
    Test {
        hanabi_app: HanabiApp,
    },
}

//l
impl Default for HelloApp {
    fn default() -> Self {
        let (client_to_server_sender, client_to_server_receiver) =
            mpsc::channel::<ClientToServerMessage>();
        let (server_to_client_sender, server_to_client_receiver) =
            mpsc::channel::<ServerToClientMessage>();

        //Creating the Ratatui backend/ Egui widget here
        let backend = RataguiBackend::new(100, 100);
        let terminal = Terminal::new(backend).unwrap();
        Self {
            terminal: terminal,
            tui_state: TuiState::AppInput(AppInput::default()),
            send_to_server: client_to_server_sender,
            read_from_server: server_to_client_receiver,
            send_to_server_queue: client_to_server_receiver,
            server_to_client_sender: server_to_client_sender,
            websocket: None,
            web_url: "".to_string(),
            // player_name: "Player".to_string(),
            // session_id: None,
            // url: "ws://localhost:8080".to_string(),
            cursor: egui::CursorIcon::Default,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn get_websocket_url(cc: &eframe::CreationContext<'_>) -> String {
    let proto = &cc.integration_info.web_info.location.protocol;
    let host = &cc.integration_info.web_info.location.host;

    console_log!("Protocol: '{:?}'", proto);
    console_log!("Host: '{:?}'", host);

    let url = match proto.as_str() {
        "https:" => format!("wss://{}/websocket", host),
        _ => format!("ws://{}/websocket", host),
    };

    url
}

#[cfg(target_arch = "wasm32")]
fn get_web_url(cc: &eframe::CreationContext<'_>) -> String {
    let origin = &cc.integration_info.web_info.location.origin;
    return origin.clone();
}

#[cfg(target_arch = "wasm32")]
fn get_session_id(cc: &eframe::CreationContext<'_>) -> Option<String> {
    let session = cc
        .integration_info
        .web_info
        .location
        .query_map
        .get("session_id")?
        .join("");

    Some(session)
}

static PLAYER_NAME: &str = "player_name";

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn get_websocket_url(_cc: &eframe::CreationContext<'_>) -> String {
    "ws://127.0.0.1:8000/websocket".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn get_web_url(_cc: &eframe::CreationContext<'_>) -> String {
    "http://127.0.0.1:8000".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn get_session_id(_cc: &eframe::CreationContext<'_>) -> Option<String> {
    None
}

impl NewCC for HelloApp {
    /// Called once before the first frame.
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let websocket_url = get_websocket_url(cc);
        // let url = "ws://127.0.0.1:8000/websocket".to_string();
        let player_name = eframe::get_value::<String>(cc.storage.unwrap(), PLAYER_NAME);
        let session_id = get_session_id(cc);
        let web_url = get_web_url(cc);
        // let (session_id, player_name, url) = get_params(cc).unwrap();
        // console_log!("Session ID: {:?}", session_id);
        // console_log!("Player Name: {:?}", player_name);
        // console_log!("URL: {:?}", url);

        let (client_to_server_sender, client_to_server_receiver) =
            mpsc::channel::<ClientToServerMessage>();
        let (server_to_client_sender, server_to_client_receiver) =
            mpsc::channel::<ServerToClientMessage>();

        console_log!("Hello from wasm");
        console_log!("Player Name: {:?}", player_name);
        console_log!("Session ID: {:?}", session_id);
        console_log!("URL: {:?}", websocket_url);
        console_log!("Web URL: {:?}", web_url);

        // let result = setup_websocket(
        //     url.clone(),
        //     player_name.clone(),
        //     session_id.clone(),
        //     server_to_client_sender.clone(),
        // );

        // console_log!("Websocket setup result: {:?}", result);

        // let ws = result.unwrap();

        setup_custom_fonts(&cc.egui_ctx);

        //Creating the Ratatui backend/ Egui widget here
        let mut backend = RataguiBackend::new_with_fonts(
            100,
            100,
            "JetBrainsMonoNerdFont-Regular".into(),
            "JetBrainsMonoNerdFont-Bold".into(),
            "JetBrainsMonoNerdFont-Oblique".into(),
            "JetBrainsMonoNerdFont-BoldOblique".into(),
        );
        // let mut backend = RataguiBackend::new(200, 100);
        // backend.set_font_size(16);

        let session_join_url =
            session_id.and_then(|s| Some(format!("{}/?session_id={}", web_url.clone(), s)));

        let terminal = Terminal::new(backend).unwrap();
        Self {
            terminal: terminal,
            tui_state: TuiState::AppInput(AppInput::new(
                websocket_url.clone(),
                session_join_url,
                player_name.unwrap_or("".to_string()),
            )),
            // tui_state: TuiState::Test {
            //     hanabi_app: HanabiApp::new(HanabiClient::Connecting),
            // },
            send_to_server: client_to_server_sender,
            send_to_server_queue: client_to_server_receiver,
            read_from_server: server_to_client_receiver,
            server_to_client_sender: server_to_client_sender.clone(),
            websocket: None,
            web_url: web_url,
            cursor: egui::CursorIcon::Default,
        }
    }

    //matches index.html
    fn canvas_id() -> String {
        "the_canvas_id".into()
    }
}

impl HelloApp {
    fn get_player_name(&mut self, eframe: &mut eframe::Frame) -> Option<String> {
        eframe::get_value::<String>(eframe.storage_mut()?, PLAYER_NAME)
    }

    fn set_player_name(
        &mut self,
        eframe: &mut eframe::Frame,
        player_name: String,
    ) -> Option<String> {
        set_value::<String>(eframe.storage_mut()?, PLAYER_NAME, &player_name);
        Some(player_name)
    }
}

impl eframe::App for HelloApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //call repaint here so that app runs continuously, remove if you dont need that
        ctx.request_repaint();

        let main_font = FontId::new(
            self.terminal.backend().get_font_size() as f32,
            FontFamily::Name("JetBrainsMonoNerdFont-Regular".to_owned().into()),
        );

        let screen_rect = ctx.screen_rect();
        match self.tui_state {
            TuiState::Test { ref mut hanabi_app } => {
                hanabi_app.draw(&mut self.terminal).unwrap();
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.add(self.terminal.backend_mut());
                });
            }
            TuiState::AppInput(ref mut app_input) => {
                let copy_url = app_input.session_id.clone().unwrap_or("".to_string());
                app_input.draw(&mut self.terminal);

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.add(self.terminal.backend_mut());

                    ui.input(|i| {
                        i.events.iter().for_each(|e| {
                            match e {
                                egui::Event::Key {
                                    key: egui::Key::Space,
                                    ..
                                } => {
                                    console_log!("Coppied URL! {}", copy_url);
                                    console_log!("UI: {:?}", ui.available_width());
                                    // Doesn't work :(
                                    // ctx.output_mut(|o| {
                                    //     o.copied_text = format!("{}", copy_url);
                                    // });
                                    // return;
                                }
                                _ => {}
                            }
                            let key = key_code_to_char(e);
                            if let Some(key) = key {
                                println!("Event: {:?} -> {:?}", e, key);
                                let result = app_input.handle_event(key).unwrap();

                                match result {
                                    std::ops::ControlFlow::Continue(_) => {}
                                    std::ops::ControlFlow::Break(_) => {
                                        console_log!("Got name! {:?}", app_input.display_name);
                                    }
                                }
                            }
                        })
                    });
                });

                if let InputMode::Done = app_input.input_mode {
                    let player_name = app_input.display_name.clone();
                    let session_id = app_input.session_id.clone();

                    match session_id {
                        Some(session_id) => {
                            self.tui_state = TuiState::HanabiApp {
                                hanabi_app: HanabiApp::new(HanabiClient::Connecting),
                                player_name: player_name.clone(),
                                session_id: session_id.clone(),
                                server_address: app_input.server_address.clone(),
                            };
                        }
                        None => {
                            self.tui_state = TuiState::CreatingGame {
                                player_name: player_name.clone(),
                                server_address: app_input.server_address.clone(),
                            };
                        }
                    }

                    self.set_player_name(_frame, player_name);
                }
            }
            TuiState::CreatingGame {
                ref player_name,
                ref server_address,
            } => {
                if let None = self.websocket {
                    let result = setup_websocket(
                        server_address.clone(),
                        player_name.clone(),
                        None,
                        self.server_to_client_sender.clone(),
                        ctx.clone(),
                    );
                    console_log!("Websocket setup result: {:?}", result);
                    self.websocket = Some(result.unwrap());
                }

                let message = self.read_from_server.try_recv();

                match message {
                    Ok(message) => match message {
                        ServerToClientMessage::CreatedGame { session_id } => {
                            console_log!("Got Created Game... {:?}", session_id);

                            ctx.output_mut(|o| {
                                // doesn't work :(
                                // o.copied_text =
                                //     format!("{}/?session_id={}", self.web_url, session_id);
                                o.open_url = Some(OpenUrl {
                                    url: format!("/?session_id={}", session_id),
                                    new_tab: false,
                                })
                            });
                        }
                        _ => {}
                    },
                    _ => {}
                };
            }
            TuiState::HanabiApp {
                ref mut hanabi_app,
                ref player_name,
                ref session_id,
                ref server_address,
            } => {
                let bindings: Vec<Binding> = hanabi_app.draw(&mut self.terminal).unwrap();

                egui::CentralPanel::default().show(ctx, |ui| {
                    let char_height = ui.fonts(|fx| fx.row_height(&main_font));
                    let char_width = ui.fonts(|fx| self.terminal.backend().get_font_width(fx));

                    let point_to_char = |pos: &Pos2| Position {
                        x: (pos.x / char_width) as u16,
                        y: (pos.y / char_height) as u16,
                    };

                    let term = ui.add(self.terminal.backend_mut());

                    // let hover_binding =
                    //     term.hover_pos()
                    //         .map(|pos| point_to_char(&pos))
                    //         .and_then(|pos| {
                    //             bindings
                    //                 .iter()
                    //                 .find(|binding| binding.click_rect.contains(pos))
                    //         });

                    ui.output_mut(|o| {
                        o.cursor_icon = self.cursor;
                    });

                    // term.on_hover_and_drag_cursor(match hover_binding {
                    //     Some(_) => {
                    //         console_log!("Hovered binding: {:?}", hover_binding);
                    //         egui::CursorIcon::PointingHand
                    //     }
                    //     None => egui::CursorIcon::Default,
                    // });

                    // let hovered_binding = term.on_hover_text_at_pointer(text)
                    //     .map(|h| {
                    //         bindings
                    //             .iter()
                    //             .find(|binding| binding.click_rect.contains(point_to_char(&h)))
                    //     })
                    //     .flatten();

                    // hovered_binding.and_then(|binding| {
                    //     ui.output_mut(|o| {
                    //         // o.cursor_icon = match hovered_binding {
                    //         //     Some(_) => egui::CursorIcon::PointingHand,
                    //         //     None => egui::CursorIcon::Default,
                    //         // }
                    //         o.cursor_icon = egui::CursorIcon::PointingHand;
                    //     });
                    //     Some(binding)
                    // });

                    ui.input(|i| {
                        i.events.iter().for_each(|e| {
                            use egui::Event;

                            let binding_matched = match e {
                                Event::Copy => None,
                                Event::Cut => None,
                                Event::Paste(_) => None,
                                Event::Text(char) => {
                                    if char.chars().count() == 1 {
                                        let key = char.chars().next().unwrap();

                                        console_log!("Key pressed: ({})", key);

                                        bindings
                                            .iter()
                                            .find(|binding| KeyCode::Char(key) == binding.key_code)
                                    } else {
                                        None
                                    }
                                }

                                Event::PointerMoved(pos2) => {
                                    let hovered_binding = bindings.iter().find(|binding| {
                                        binding.click_rect.contains(point_to_char(pos2))
                                    });

                                    if hovered_binding.is_some() {
                                        self.cursor = egui::CursorIcon::PointingHand;
                                    } else {
                                        self.cursor = egui::CursorIcon::Default;
                                    }
                                    None
                                }
                                Event::MouseMoved(_) => None,
                                Event::PointerButton {
                                    pos, pressed: true, ..
                                } => {
                                    console_log!("Bindings {:?}", bindings);
                                    let x = (pos.x / char_width) as u16;
                                    let y = (pos.y / char_height) as u16;

                                    console_log!(
                                        "Click at: ({}, {}) size=({},{}) ",
                                        x,
                                        y,
                                        char_width,
                                        char_height
                                    );

                                    bindings.iter().find(|binding| {
                                        binding.click_rect.contains(point_to_char(pos))
                                    })
                                }

                                Event::Scroll(_) => None,
                                Event::Zoom(_) => None,

                                Event::Touch { .. } => None,
                                Event::MouseWheel { .. } => None,

                                _ => None,
                            };

                            if let Some(binding) = binding_matched {
                                console_log!("Binding: {:?}", binding);
                                let result = hanabi_app.handle_action(binding.action).unwrap();

                                match result {
                                    EventHandlerResult::PlayerAction(action) => {
                                        self.send_to_server
                                            .send(ClientToServerMessage::PlayerAction { action })
                                            .unwrap();
                                    }
                                    EventHandlerResult::Start => {
                                        self.send_to_server
                                            .send(ClientToServerMessage::StartGame)
                                            .unwrap();
                                    }
                                    EventHandlerResult::Quit => {}
                                    EventHandlerResult::Continue => {}
                                }
                            }

                            // let key = key_code_to_char(e);
                            // if let Some(key) = key {
                            //     println!("Event: {:?} -> {:?}", e, key);

                            //     if key == KeyCode::Char(' ') {
                            //         console_log!("DEBUG: {}", ui.available_width());
                            //     }

                            //     if let Some(binding) = bindings.iter().find(|b| b.key == key) {
                            //         console_log!("Binding: {:?}", binding);
                            //         let result = hanabi_app.handle_event(binding.action).unwrap();

                            //         match result {
                            //             EventHandlerResult::PlayerAction(action) => {
                            //                 self.send_to_server
                            //                     .send(ClientToServerMessage::PlayerAction {
                            //                         action,
                            //                     })
                            //                     .unwrap();
                            //             }
                            //             EventHandlerResult::Start => {
                            //                 self.send_to_server
                            //                     .send(ClientToServerMessage::StartGame)
                            //                     .unwrap();
                            //             }
                            //             EventHandlerResult::Quit => {}
                            //             EventHandlerResult::Continue => {}
                            //         }
                            //     }

                            // let result = hanabi_app.handle_event(key).unwrap();

                            // match result {
                            //     EventHandlerResult::PlayerAction(action) => {
                            //         self.send_to_server
                            //             .send(ClientToServerMessage::PlayerAction { action })
                            //             .unwrap();
                            //     }
                            //     EventHandlerResult::Start => {
                            //         self.send_to_server
                            //             .send(ClientToServerMessage::StartGame)
                            //             .unwrap();
                            //     }
                            //     EventHandlerResult::Quit => {}
                            //     EventHandlerResult::Continue => {}
                            // }
                            // }
                        })
                    });
                });

                if let Some(websocket) = &self.websocket {
                    if websocket.ready_state() == 1 {
                        let message = self.send_to_server_queue.try_recv();
                        if let Ok(message) = message {
                            console_log!("Sending... {:?}", message);

                            let send_result = websocket
                                .send_with_str(serde_json::to_string(&message).unwrap().as_str());

                            console_log!("Send result: {:?}", send_result);
                        }
                    } else if websocket.ready_state() > 1 {
                        console_log!("Websocket was closed, reconnecting...");
                        self.websocket = None;
                    }
                }

                if let None = self.websocket {
                    let result = setup_websocket(
                        server_address.clone(),
                        player_name.clone(),
                        Some(session_id.clone()),
                        self.server_to_client_sender.clone(),
                        ctx.clone(),
                    );
                    console_log!("Websocket setup result: {:?}", result);
                    self.websocket = Some(result.unwrap());
                }

                let message = self.read_from_server.try_recv();

                match message {
                    Ok(message) => match message {
                        ServerToClientMessage::CreatedGame { session_id } => {}
                        ServerToClientMessage::UpdatedGameState(game_state) => {
                            console_log!("Got Updated Game State... {:?}", game_state);

                            let new_state = HanabiClient::Loaded(game_state);
                            hanabi_app.update(new_state);
                        }
                        ServerToClientMessage::Error(error) => {
                            console_log!("Got Error... {:?}", error);
                        }
                    },
                    _ => {}
                };
            }
        }
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "JetBrainsMonoNerdFont-Regular".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/JetBrainsMonoNerdFont-Regular.ttf"
        )),
    );
    fonts.families.insert(
        egui::FontFamily::Name("JetBrainsMonoNerdFont-Regular".into()),
        vec!["JetBrainsMonoNerdFont-Regular".to_owned()],
    );
    fonts.font_data.insert(
        "JetBrainsMonoNerdFont-Bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/JetBrainsMonoNerdFont-Bold.ttf")),
    );
    fonts.families.insert(
        egui::FontFamily::Name("JetBrainsMonoNerdFont-Bold".into()),
        vec!["JetBrainsMonoNerdFont-Bold".to_owned()],
    );

    fonts.font_data.insert(
        "JetBrainsMonoNerdFont-Oblique".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/JetBrainsMonoNerdFont-Italic.ttf")),
    );
    fonts.families.insert(
        egui::FontFamily::Name("JetBrainsMonoNerdFont-Oblique".into()),
        vec!["JetBrainsMonoNerdFont-Oblique".to_owned()],
    );

    fonts.font_data.insert(
        "JetBrainsMonoNerdFont-BoldOblique".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/JetBrainsMonoNerdFont-BoldItalic.ttf"
        )),
    );
    fonts.families.insert(
        egui::FontFamily::Name("JetBrainsMonoNerdFont-BoldOblique".into()),
        vec!["JetBrainsMonoNerdFont-BoldOblique".to_owned()],
    );

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

fn setup_websocket(
    url: String,
    player_name: String,
    session_id: Option<String>,
    server_to_client_sender: Sender<ServerToClientMessage>,
    ctx: egui::Context,
) -> Result<WebSocket, JsValue> {
    console_log!("Connecting to websocket: {:?}", url);

    // Connect to an echo server
    let ws = WebSocket::new(&url)?;
    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    // create callback
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        // Handle difference Text/Binary,...
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            console_log!("message event, received arraybuffer: {:?}", abuf);
            let array = js_sys::Uint8Array::new(&abuf);
            let len = array.byte_length() as usize;
            console_log!("Arraybuffer received {}bytes: {:?}", len, array.to_vec());
        } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
            console_log!("message event, received blob: {:?}", blob);
            // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
            let fr = web_sys::FileReader::new().unwrap();
            let fr_c = fr.clone();
            // create onLoadEnd callback
            let onloadend_cb = Closure::<dyn FnMut(_)>::new(move |_e: web_sys::ProgressEvent| {
                let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                let len = array.byte_length() as usize;
                console_log!("Blob received {}bytes: {:?}", len, array.to_vec());
                // here you can for example use the received image/png data
            });
            fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
            fr.read_as_array_buffer(&blob).expect("blob not readable");
            onloadend_cb.forget();
        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            console_log!("message event, received Text: {:?}", txt);
            let txt: String = txt.into();

            let msg: ServerToClientMessage = serde_json::from_str(&txt).unwrap();

            console_log!("Message received: {:?}", msg);
            server_to_client_sender.send(msg).unwrap();
            ctx.request_repaint();
        } else {
            console_log!("message event, received Unknown: {:?}", e.data());
        }
    });
    // set message event handler on WebSocket
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    // forget the callback to keep it alive
    onmessage_callback.forget();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        console_log!("error event: {:?}", e);
    });

    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let cloned_ws = ws.clone();

    let init_message = serde_json::to_string(&match session_id {
        None => ClientToServerMessage::CreateGame {
            player_name: player_name.clone(),
        },
        Some(session_id) => ClientToServerMessage::Join {
            player_name: player_name.clone(),
            session_id: session_id,
        },
    })
    .unwrap();

    console_log!("Sending init message: {:?}", init_message);

    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        console_log!("socket opened");

        match cloned_ws.send_with_str(&init_message) {
            Ok(_) => console_log!("successfully sent message"),
            Err(err) => console_log!("error sending join message: {:?}", err),
        }
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    Ok(ws.clone())
}
