//! Golden-file snapshot tests for the ratatui UI.
//!
//! These render `HanabiApp` against a `TestBackend` (no egui, no WASM) and
//! compare the resulting buffer to a snapshot in `tests/snapshots/`. To accept
//! intentional UI changes, run:
//!
//! ```
//! cargo insta review
//! # or to blanket-accept:
//! cargo insta accept
//! ```
//!
//! The `buffer_to_string` helper renders each Cell's `symbol()` row-by-row so
//! snapshots are diffable plain text.

use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use ratatui_app::hanabi_app::{HanabiApp, HanabiClient};
use shared::client_logic::{ConnectionStatus, HanabiGame, OnlinePlayer};
use shared::test_data::generate_example_panic_case_1;

fn buffer_to_string(buf: &Buffer) -> String {
    let mut out = String::with_capacity((buf.area.width as usize + 1) * buf.area.height as usize);
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf.get(x, y).symbol());
        }
        out.push('\n');
    }
    out
}

fn render(state: HanabiClient, width: u16, height: u16) -> String {
    let mut app = HanabiApp::new(state);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal init");
    app.draw(&mut terminal).expect("draw");
    buffer_to_string(terminal.backend().buffer())
}

#[test]
fn renders_connecting_state() {
    let rendered = render(HanabiClient::Connecting, 80, 24);
    insta::assert_snapshot!(rendered);
}

#[test]
fn renders_lobby_state() {
    let lobby = HanabiGame::Lobby {
        session_id: "test-session".to_string(),
        log: vec![],
        players: vec![
            OnlinePlayer {
                name: "alice".into(),
                connection_status: ConnectionStatus::Connected,
                is_host: true,
            },
            OnlinePlayer {
                name: "bob".into(),
                connection_status: ConnectionStatus::Connected,
                is_host: false,
            },
        ],
    };
    let rendered = render(HanabiClient::Loaded(lobby), 80, 24);
    insta::assert_snapshot!(rendered);
}

#[test]
fn renders_mid_game_playing_state() {
    let rendered = render(HanabiClient::Loaded(generate_example_panic_case_1()), 156, 38);
    insta::assert_snapshot!(rendered);
}
