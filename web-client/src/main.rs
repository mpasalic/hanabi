use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use egui::{
    style::{Selection, WidgetVisuals, Widgets},
    Event, Visuals,
};
use ratatui::{
    prelude::{Stylize, Terminal},
    widgets::Paragraph,
};
use ratatui_app::hanabi_app::*;
use ratframe::NewCC;
use ratframe::RataguiBackend;
use shared::client_logic::*;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};
use web_time::{Duration, Instant};
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
    hanabi_app: HanabiApp,
    send_to_server: mpsc::Sender<ClientToServerMessage>,
    send_to_server_queue: mpsc::Receiver<ClientToServerMessage>,
    read_from_server: mpsc::Receiver<ServerToClientMessage>,
    server_to_client_sender: Sender<ServerToClientMessage>,

    websocket: Option<WebSocket>,

    player_name: String,
    session_id: String,
    url: String,
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
        let mut terminal = Terminal::new(backend).unwrap();
        Self {
            terminal: terminal,
            hanabi_app: HanabiApp::new(HanabiClient::Connecting),
            send_to_server: client_to_server_sender,
            read_from_server: server_to_client_receiver,
            send_to_server_queue: client_to_server_receiver,
            server_to_client_sender: server_to_client_sender,
            websocket: None,
            player_name: "Player".to_string(),
            session_id: "Session".to_string(),
            url: "ws://localhost:8080".to_string(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn get_params(cc: &eframe::CreationContext<'_>) -> Option<(String, String, String)> {
    let session = cc
        .integration_info
        .web_info
        .location
        .query_map
        .get("session_id")
        .unwrap()
        .join("");

    let name = cc
        .integration_info
        .web_info
        .location
        .query_map
        .get("name")
        .unwrap()
        .join("");

    // const proto = location.protocol.startsWith("https") ? "wss" : "ws";
    // const websocket = new WebSocket(
    //   `${proto}://${window.location.host}/websocket`
    // );
    let proto = &cc.integration_info.web_info.location.protocol;
    let host = &cc.integration_info.web_info.location.host;

    let url = match proto.as_str() {
        "https" => format!("wss://{}/websocket", host),
        _ => format!("ws://{}/websocket", host),
    };

    Some((session, name, url))
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn get_params(cc: &eframe::CreationContext<'_>) -> Option<(String, String, String)> {
    None
}

impl NewCC for HelloApp {
    /// Called once before the first frame.
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (session_id, player_name, url) = get_params(cc).unwrap();
        console_log!("Session ID: {:?}", session_id);
        console_log!("Player Name: {:?}", player_name);
        console_log!("URL: {:?}", url);

        let (client_to_server_sender, client_to_server_receiver) =
            mpsc::channel::<ClientToServerMessage>();
        let (server_to_client_sender, server_to_client_receiver) =
            mpsc::channel::<ServerToClientMessage>();

        console_log!("Hello from wasm");

        let result = setup_websocket(
            url.clone(),
            player_name.clone(),
            session_id.clone(),
            server_to_client_sender.clone(),
        );

        console_log!("Websocket setup result: {:?}", result);

        let ws = result.unwrap();

        setup_custom_fonts(&cc.egui_ctx);
        //Creating the Ratatui backend/ Egui widget here
        let backend = RataguiBackend::new_with_fonts(
            100,
            100,
            "Regular".into(),
            "Bold".into(),
            "Oblique".into(),
            "BoldOblique".into(),
        );

        let mut terminal = Terminal::new(backend).unwrap();
        Self {
            terminal: terminal,
            hanabi_app: HanabiApp::new(HanabiClient::Connecting),
            send_to_server: client_to_server_sender,
            send_to_server_queue: client_to_server_receiver,
            read_from_server: server_to_client_receiver,
            server_to_client_sender: server_to_client_sender.clone(),
            websocket: Some(ws),
            player_name: player_name,
            session_id: session_id,
            url: url,
        }
    }

    //matches index.html
    fn canvas_id() -> String {
        "the_canvas_id".into()
    }
}

impl eframe::App for HelloApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //call repaint here so that app runs continuously, remove if you dont need that
        ctx.request_repaint();

        self.hanabi_app.draw(&mut self.terminal);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(self.terminal.backend_mut());

            ui.input(|i| {
                i.events.iter().for_each(|e| {
                    let key = key_code_to_char(e);
                    if let Some(key) = key {
                        println!("Event: {:?} -> {:?}", e, key);
                        let result = self.hanabi_app.handle_event(key).unwrap();

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
                })
            });
        });

        if let Some(websocket) = &self.websocket {
            if websocket.ready_state() == 1 {
                let message = self.send_to_server_queue.try_recv();
                if let Ok(message) = message {
                    console_log!("Sending... {:?}", message);

                    let send_result =
                        websocket.send_with_str(serde_json::to_string(&message).unwrap().as_str());

                    console_log!("Send result: {:?}", send_result);
                }
            } else if websocket.ready_state() > 1 {
                console_log!("Websocket was closed, reconnecting...");
                self.websocket = None;
            }
        }

        if let None = self.websocket {
            let result = setup_websocket(
                self.url.clone(),
                self.player_name.clone(),
                self.session_id.clone(),
                self.server_to_client_sender.clone(),
            );
            console_log!("Websocket setup result: {:?}", result);
            self.websocket = Some(result.unwrap());
        }

        let message = self.read_from_server.try_recv();

        match message {
            Ok(message) => match message {
                ServerToClientMessage::UpdatedGameState(game_state) => {
                    console_log!("Got Updated Game State... {:?}", game_state);

                    let new_state = HanabiClient::Loaded(game_state);
                    self.hanabi_app.update(new_state);
                }
                _ => {}
            },
            _ => {}
        };
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "Regular".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/JetBrainsMonoNerdFont-Regular.ttf"
        )),
    );
    fonts.families.insert(
        egui::FontFamily::Name("Regular".into()),
        vec!["Regular".to_owned()],
    );
    fonts.font_data.insert(
        "Bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/JetBrainsMonoNerdFont-Bold.ttf")),
    );
    fonts.families.insert(
        egui::FontFamily::Name("Bold".into()),
        vec!["Bold".to_owned()],
    );

    fonts.font_data.insert(
        "Oblique".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/JetBrainsMonoNerdFont-Italic.ttf")),
    );
    fonts.families.insert(
        egui::FontFamily::Name("Oblique".into()),
        vec!["Oblique".to_owned()],
    );

    fonts.font_data.insert(
        "BoldOblique".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/JetBrainsMonoNerdFont-BoldItalic.ttf"
        )),
    );
    fonts.families.insert(
        egui::FontFamily::Name("BoldOblique".into()),
        vec!["BoldOblique".to_owned()],
    );

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

fn setup_websocket(
    url: String,
    player_name: String,
    session_id: String,
    server_to_client_sender: Sender<ServerToClientMessage>,
) -> Result<WebSocket, JsValue> {
    console_log!("Connecting to websocket: {:?}", url);

    // Connect to an echo server
    let ws = WebSocket::new(&url)?;
    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    // create callback
    let cloned_ws = ws.clone();
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

    let join_msg = serde_json::to_string(&ClientToServerMessage::Join {
        player_name: player_name.clone(),
        session_id: session_id.clone(),
    })
    .unwrap();

    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        console_log!("socket opened");

        match cloned_ws.send_with_str(&join_msg) {
            Ok(_) => console_log!("join successfully sent"),
            Err(err) => console_log!("error sending join message: {:?}", err),
        }
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    Ok(ws.clone())
}
