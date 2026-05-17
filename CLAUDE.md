# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

All common workflows are in `justfile` — `just --list` prints them.

- `just build` — cargo build the workspace + `trunk build` the WASM web client into `dist/`
- `just test` — `just build` then `cargo test` (most tests live in `shared/`)
- `cargo test -p shared` / `cargo test -p ratatui-app` — scope tests to one workspace crate
- `cargo test -p shared <test_name>` — run a single test by name
- `just serve` — build + `cargo run -p server`. Needs `DATABASE_URL` in env (or a `.env` at repo root). Serves websocket at `/websocket` and the prebuilt `dist/` at `/` on `:8080`.
- `just run` — `trunk serve --open` the web client on `:8080`, proxying `/websocket` to a local server on `:8080`. Auto-recompiles on `web-client/` changes only (not `shared/` or `ratatui-app/`).
- `just run-release` — same, but proxies to the deployed Fly.io server (`wss://hanabi-tui.fly.dev/websocket`).
- `just build-release` — release WASM build for the web client.
- `just release` — `just build-release` then `flyctl deploy`.
- `just logs` — tail Fly.io logs.

## Architecture

This is a Cargo workspace for a multiplayer Hanabi card game. Authoritative game state lives on an Axum server deployed to Fly.io; game history is persisted to Postgres (Neon). Clients are thin views that render a Ratatui UI inside egui in the browser (WASM).

### Workspace crates (see root `Cargo.toml`)

- **`shared/`** — all cross-cutting game code. Pure, no IO.
  - `model.rs` — core domain types (`GameState`, `GameStateSnapshot`, `PlayerAction`, `CardSuit`/`CardFace`, `PlayerIndex`/`SlotIndex`, etc.). These are `Serialize`/`Deserialize` and form the wire protocol.
  - `logic.rs` — authoritative game engine (move validation, effects, outcome).
  - `client_logic.rs` — client-visible projections (`GameStateSnapshot`, `HanabiGame::{Lobby, Playing, Spectating, Ended}`) and the websocket message types `ClientToServerMessage` / `ServerToClientMessage`. The wire protocol is JSON-serialized versions of these enums.
- **`ratatui-app/`** — the UI, as a library. Uses `ratatui` for rendering plus a `taffy`-based flexbox layout engine (`nodes.rs`). Key entry points: `hanabi_app::HanabiApp` (in-game UI) and `input_app::AppInput` (the lobby/name/session-id entry screen). It is backend-agnostic — no IO, no direct terminal or websocket.
- **`web-client/`** — the egui/eframe/WASM shell that hosts `ratatui-app` via the `ratframe` bridge. `src/main.rs` owns the `HelloApp` state machine (`TuiState::{AppInput, CreatingGame, HanabiApp, Test}`) and the websocket lifecycle; `hanabi_backend.rs` is a custom `ratatui::Backend` that paints into egui. Built with `trunk`. The websocket URL is derived at runtime from the browser's own origin — the page always talks back to whatever host served it.
- **`server/`** — plain Axum + Tokio binary, with `lib.rs` exposing the modules so integration tests can reuse them. `main.rs` is thin bootstrapping; `app.rs` owns `build_router(Arc<dyn Database>) -> Router` and the websocket upgrade. `lobby.rs` holds `LobbyServer` (game/lobby state machine) and dispatches `ClientToServerMessage`s. `model.rs` defines `trait Database` + `PgDatabase` (sqlx + Postgres) — tests use an in-memory impl. `migrations/*.sql` is the schema. DB connection comes from `DATABASE_URL` env var.

### Data flow

1. Browser loads the WASM bundle; `HelloApp::new` reads `session_id` from the URL query string and the persisted `player_name` from eframe storage.
2. Websocket opens to `wss://{current_host}/websocket` (or `ws://` for http). In production this is the Fly app itself; locally it's `127.0.0.1:8080`.
3. Client sends `CreateGame` / `Join` / `Spectate` as the init message, then `PlayerAction` / `StartGame` while playing.
4. Server validates via `shared::logic`, persists the action to Postgres (`save_action`), and broadcasts `UpdatedGameState(HanabiGame)` or `Error(String)` to affected clients. `UpdatedConnectionStatus` is separate.
5. Client applies an **optimistic local mutation** (`GameStateSnapshot::apply_local_mutation`) before round-tripping — the authoritative snapshot from the server then replaces it.

### Persistence & recovery

Every player action is written to the `game_log` table in Postgres as it happens (see `save_action` in `server/src/model.rs`). Tables: `game_config` (setup), `player` (roster), `game_log` (action history with timestamps).

When a client sends `Join { session_id }` for a game that is NOT in the server's in-memory `HashMap`, `LobbyServer::hydrate` (in `server/src/lobby.rs`) reads the config, players, and full action log from Postgres and replays every action through `GameLog::log()` to reconstruct the live state. This means:

- A restart (e.g. Fly Machine auto-stopping when idle, then waking on the next connection) loses in-memory lobbies but **no game data** — any reconnecting player triggers hydration and the game resumes exactly where it left off.
- All played games are durably browsable from the DB (no UI for this yet; read-only replay would be a future addition).

## Testing

Full reference: [`TESTING.md`](TESTING.md). Quick decision tree for "where does my test go":

| Touched | Add a test in |
|---|---|
| Game rules in `shared/src/logic.rs` | `shared/src/logic.rs` `#[test]` (unit) |
| A new "this can never happen" invariant | `shared/tests/proptest_invariants.rs` (proptest) |
| `apply_local_mutation` (optimistic client preview) | `shared/tests/client_logic_parity.rs` (parity vs server) |
| One UI component in `ratatui-app/src/components.rs` | `ratatui-app/tests/component_snapshots.rs` (insta) |
| Top-level screens / `HanabiApp::ui` dispatch | `ratatui-app/tests/render_snapshots.rs` (insta) |
| `LobbyServer::message_received` behavior | `server/tests/lobby.rs` (in-memory `MemDatabase`) |
| SQL / migrations / hydration replay | `server/tests/hydration_pg.rs` (gated on `TEST_DATABASE_URL`) |
| Wire protocol, websocket upgrade, Axum router | `server/tests/e2e_websocket.rs` (real `tokio-tungstenite`) |
| `web-client/` (WASM shell, egui adapter) | No automated coverage — verify manually in browser |

Default to the cheapest layer that catches the regression. Snapshot tests use `insta` — update them with `cargo insta accept` (or `cargo insta review` for interactive). All test fixtures and the `GameStateSnapshotBuilder` live in `shared/src/test_data.rs` behind the `test-helpers` Cargo feature; consume them from any crate's `[dev-dependencies]` via `shared = { path = "../shared", features = ["test-helpers"] }`. The Postgres-backed test reads `TEST_DATABASE_URL` and silently skips if unset — for local runs, `just db-init` then export it pointing at `hanabi_$WORKTREE_DB`.

## Deployment

**Fly.io** (single Machine, auto-stop / auto-start) serves both the WASM web client (from `dist/`) and the websocket (`/websocket`) from the same origin. Configuration lives in `fly.toml` + `Dockerfile` at the repo root.

**Neon** provides the Postgres. Connection string is set via `flyctl secrets set DATABASE_URL=...`.

CI: `.github/workflows/integration.yml` runs three jobs on push/PR — `lint` (fmt + clippy, currently advisory), `test` (with a Postgres service so `TEST_DATABASE_URL`-gated tests run), and `coverage` (uploads an `lcov.info` artifact via `cargo-llvm-cov`). `.github/workflows/deploy-fly.yml` is `workflow_dispatch` only (manual) — requires `FLY_API_TOKEN` repo secret.

Note: there is no staging environment. Any `flyctl deploy` goes straight to production.

## Conventions

- The wire protocol is just `serde_json` on the `ClientToServerMessage` / `ServerToClientMessage` enums in `shared/client_logic.rs`. Changing a variant is a breaking change for any already-connected clients — there is no versioning.
- `PlayerIndex(usize)` and `SlotIndex(usize)` are newtypes; prefer them over raw `usize` at API boundaries.
- There is no auth — player identity is just the chosen display name, and the server treats same-name reconnections as the same player (see README caveat). Don't add impersonation checks ad-hoc without discussing; it's a known design limitation.
- DB queries use `sqlx::query_as::<_, Row>("SELECT ...")` with runtime SQL strings, not the compile-time `query!` macro. This means you don't need a live `DATABASE_URL` at build time and don't need a `.sqlx/` offline cache — but you also don't get compile-time schema checking. Preserve this pattern when adding queries.
