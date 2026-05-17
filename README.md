# Hanabi
[Hanabi](https://en.wikipedia.org/wiki/Hanabi_(card_game)) is a cooperative card game where players are only aware of other players cards and can only communicate with each other through a system of hints that reveal limited information. The objective is to play a series of cards in the correct sequential order in order to complete all sets, resulting in fireworks (Hanabi is a japenese word for "fireworks"). Think multiplayer solataire!

This version of Hanabi is a multiplayer web app, built in Rust using the [ratatui](https://ratatui.rs/) and [egui](https://github.com/gold-silver-copper/egui_ratatui) for a terminal-like experience in the browser, connected to an axum websocket server hosted on [Fly.io](https://fly.io) with a [Neon](https://neon.tech) Postgres backend.

## How to play

To start a game with friends, open the deployed URL and press [Enter] to create a game. This will redirect to a new session. Share the URL in your browser with others. Once everyone has joined, press [S] to start the game. Enjoy!

<img width="1485" height="764" alt="hanabi-screenshot" src="https://github.com/user-attachments/assets/6ac1fb4e-ebe9-4b3c-8daa-ecbe367e4288" />

**Important notes**
* There is no authentication or any security that prevents impersonation. If anyone gets disconnected, simply open the session URL again and choose the same name (it is persisted to make it easier). This also means everyone needs a unique name, or else the game will consider them the same person!

# Development
## Workspaces

- `web-client/` — egui wrapper that serves the app through the browser via WebAssembly
  - `ratatui-app/` — library crate implementing the actual Ratatui UI
- `server/` — Axum websocket server (lobby + game engine). Runs on Fly.io.
- `shared/` — shared models, game logic, and wire-protocol types

## Dependencies

- `cargo install just`
- `cargo install trunk`
- `rustup target add wasm32-unknown-unknown`
- [`flyctl`](https://fly.io/docs/flyctl/install/) (for deployment)

## Local environment

1. Start a local Postgres 17 in Docker:
   ```
   just db
   ```
   `just db` is idempotent — re-runs use the existing container; data lives in a named Docker volume so it survives `db-stop`/restart. Use `just db-reset` to wipe.

   The justfile derives a per-worktree database name, server port, and trunk dev-server port from the working directory — `just info` prints the resolved values. No `.env` is required for the default local setup. Copy `.env.example` to `.env` only if you need to override anything (e.g. point `DATABASE_URL` at a Neon URL).

2. Run the server locally — this serves the websocket + prebuilt `dist/` on the worktree's `$PORT`:
   ```
   just serve
   ```

3. In a separate terminal, start a dev web client (auto-recompiles the web-client crate, proxies websocket to the local server):
   ```
   just run
   ```

   Note: changes to `shared/` or `ratatui-app/` require re-running `just run` — trunk only watches `web-client/`.

4. Trunk opens the web client automatically; the URL is `http://127.0.0.1:$TRUNK_PORT/` (see `just info`). Alternatively, `http://127.0.0.1:$PORT/` (from `just serve`) also serves the prebuilt WASM client directly, but does not hot-reload on web-client changes.

> **Upgrading from a pre-worktree setup:** if your existing `.env` pins `DATABASE_URL=...:5432/hanabi`, either remove that line to pick up the per-worktree default or run `just db-reset` once to wipe the legacy `hanabi` database. The shared `hanabi-pg` container name and volume name are unchanged; only the database inside it is now per-worktree.

### Running multiple worktrees in parallel

Each git worktree gets its own server/trunk ports and its own Postgres database inside the shared `hanabi-pg` container, derived from a hash of the worktree's path. `just info` shows the resolved values; override any of them in the worktree's `.env` if you ever hit a port collision.

Optional: if you use [portless](https://github.com/vercel-labs/portless), map a stable hostname at this worktree's trunk port. Run `just info`, then add the resolved `TRUNK_PORT` to your portless config under something like `hanabi-${branch}.test`. The websocket auto-derives from `window.location.host`, so the portless → trunk → server chain works without any code changes.

## Deployment (Fly.io + Neon)

One-time setup:

1. **Provision Neon Postgres**: create a free project at [neon.tech](https://neon.tech), copy the pooled connection string.
2. **Create the Fly app**:
   ```
   flyctl launch --no-deploy
   ```
   (Say no when asked to provision Postgres — we're using Neon.)
3. **Set the DB secret**:
   ```
   flyctl secrets set DATABASE_URL='postgresql://...neon.tech/...?sslmode=require'
   ```

Deploy from a local checkout:
```
just release
```

Or trigger the `Deploy to Fly.io` GitHub Actions workflow manually (requires `FLY_API_TOKEN` repo secret).

The app serves both the websocket (`/websocket`) and the WASM bundle (everything else) from the same origin. The Fly Machine is configured to auto-stop when idle and auto-start on the next incoming connection (~1-3s cold start), so hobby-scale play costs near-zero.
