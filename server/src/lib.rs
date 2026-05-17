//! `server` crate library entry point. Lets integration tests (and any future
//! reusable consumers) reach `LobbyServer` and the `Database` trait without
//! going through `main.rs`. The binary in `src/main.rs` is the thin Axum
//! wiring layer on top.

pub mod app;
pub mod lobby;
pub mod model;
