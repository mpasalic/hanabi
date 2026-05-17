default:
  @just --list

build:
  cargo build && cd web-client && trunk clean && trunk build

test: build
  cargo test

# Runs a web client with a proxy to the local backend server (requires `just serve`)
run:
  cd web-client && trunk serve --open --proxy-backend=ws://127.0.0.1:8080/websocket --proxy-ws

# Runs a web client with a proxy to the deployed Fly.io server
run-release:
  cd web-client && trunk serve --open --proxy-backend=wss://hanabi-tui.fly.dev/websocket --proxy-ws

# Builds the WASM web client into `dist/`
build-release:
  cd web-client && trunk clean && trunk build --release

# Runs the server locally. Needs DATABASE_URL in env or .env at repo root.
# Serves the prebuilt `dist/` at / and websocket at /websocket on :8080.
serve: build
  cargo run -p server

# Deploys to Fly.io. Requires `flyctl` auth and a configured app (see README).
release: build-release
  flyctl deploy

# Tail Fly.io app logs
logs:
  flyctl logs
