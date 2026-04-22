# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

All common workflows are in `justfile` — `just --list` prints them.

- `just build` — cargo build the workspace + `trunk build` the WASM web client into `dist/`
- `just test` — `just build` then `cargo test` (all tests live in the workspace crates; `shared/` has the bulk of the game-logic tests)
- `cargo test -p shared` / `cargo test -p ratatui-app` — scope tests to one workspace crate
- `cargo test -p shared <test_name>` — run a single test by name
- `just run` — `trunk serve --open` the web client on `:8080`, proxying `/websocket` to a local Shuttle backend on `:8000` (start that separately with `just serve`)
- `just run-release` — same, but proxies to the deployed Shuttle server at `wss://hanabi.shuttleapp.rs`
- `just serve` — build + `cargo shuttle run` (serves the API on `:8000` and also the prebuilt `dist/` at `/`; the web-client WASM is NOT auto-rebuilt — rerun `just build` on changes)
- `just serve-external` — `cargo shuttle run --external` (bind to `0.0.0.0`)
- `just build-release` — release WASM build for the web client
- `just release` — `just build-release` then `cargo shuttle deploy`

Note: the README references command names (`run-shuttle-dev`, `run-web-dev`, `build-web-release`, `deploy-shuttle-release`) that do NOT exist in the justfile. Use the names above.

Prerequisites: `cargo install just trunk`, plus the Shuttle CLI (`cargo install cargo-shuttle` / `cargo install shuttle`). `wasm32-unknown-unknown` target is required for the web client.

## Architecture

This is a Cargo workspace for a multiplayer Hanabi card game. Authoritative game state lives on an Axum/Shuttle server; clients are thin views that render a Ratatui UI inside egui in the browser (WASM).

### Workspace crates (see root `Cargo.toml`)

- **`shared/`** — all cross-cutting game code. Pure, no IO.
  - `model.rs` — core domain types (`GameState`, `GameStateSnapshot`, `PlayerAction`, `CardSuit`/`CardFace`, `PlayerIndex`/`SlotIndex`, etc.). These are `Serialize`/`Deserialize` and form the wire protocol.
  - `logic.rs` — authoritative game engine (move validation, effects, outcome).
  - `client_logic.rs` — client-visible projections (`GameStateSnapshot`, `HanabiGame::{Lobby, Playing, Spectating, Ended}`) and the websocket message types `ClientToServerMessage` / `ServerToClientMessage`. The wire protocol is JSON-serialized versions of these enums.
- **`ratatui-app/`** — the UI, as a library. Uses `ratatui` for rendering plus a `taffy`-based flexbox layout engine (`nodes.rs`). Key entry points: `hanabi_app::HanabiApp` (in-game UI) and `input_app::AppInput` (the lobby/name/session-id entry screen). It is backend-agnostic — no IO, no direct terminal or websocket.
- **`web-client/`** — the egui/eframe/WASM shell that hosts `ratatui-app` via the `ratframe` bridge. `src/main.rs` owns the `HelloApp` state machine (`TuiState::{AppInput, CreatingGame, HanabiApp, Test}`) and the websocket lifecycle; `hanabi_backend.rs` is a custom `ratatui::Backend` that paints into egui. Built with `trunk`.
- **`shuttle-server/`** — Axum server deployed via `shuttle-runtime`. `main.rs` wires the `/websocket` upgrade and serves `dist/` as static files via `ServeDir`. `server.rs` holds `LobbyServer` (game/lobby state machine, persistence) and dispatches `ClientToServerMessage`s. `model.rs` and `migrations/*.sql` are the Postgres schema (Shuttle provisions a shared Postgres via `#[shuttle_shared_db::Postgres]`).

### Data flow

1. Browser loads the WASM bundle; `HelloApp::new` reads `session_id` from the URL query string and the persisted `player_name` from eframe storage.
2. Websocket opens to the hardcoded `wss://hanabi-ufgm.shuttle.app/websocket` (see `get_websocket_url` in `web-client/src/main.rs` — currently not using the relative host; change there to proxy through the running host instead).
3. Client sends `CreateGame` / `Join` / `Spectate` as the init message, then `PlayerAction` / `StartGame` while playing.
4. Server validates via `shared::logic`, persists, and broadcasts `UpdatedGameState(HanabiGame)` or `Error(String)` to affected clients. `UpdatedConnectionStatus` is separate.
5. Client applies an **optimistic local mutation** (`GameStateSnapshot::apply_local_mutation`) before round-tripping — the authoritative snapshot from the server then replaces it.

### Legacy directories (not in the workspace)

`client/` and `server/` contain older native-terminal and pre-Shuttle implementations. They are not in `[workspace] members` in the root `Cargo.toml` and do not build as part of `cargo build`. Do not modify them unless explicitly asked — new work goes in the four workspace crates above.

## Deployment

Two separate targets, both built from the same source:

- **GitHub Pages** (`.github/workflows/deploy-gh-pages.yml`, runs on push to `main`) — publishes the WASM web client to `https://mpasalic.github.io/hanabi`. It's a pure static build of `web-client/` → `dist/`; the page connects to the production Shuttle websocket URL.
- **Shuttle** (`.github/workflows/deploy-shuttle.yml`, `workflow_dispatch` only — manual because of the free-tier deploy quota) — deploys `shuttle-server` to `https://hanabi-ufgm.shuttle.app`. `Shuttle.toml` ships `dist/*` as an asset so the Shuttle host can also serve the web client at `/`.

`just release` performs the Shuttle deploy from a local checkout.

## Conventions

- The wire protocol is just `serde_json` on the `ClientToServerMessage` / `ServerToClientMessage` enums in `shared/client_logic.rs`. Changing a variant is a breaking change for any already-connected clients — there is no versioning.
- `PlayerIndex(usize)` and `SlotIndex(usize)` are newtypes; prefer them over raw `usize` at API boundaries.
- There is no auth — player identity is just the chosen display name, and the server treats same-name reconnections as the same player (see README caveat). Don't add impersonation checks ad-hoc without discussing; it's a known design limitation.
