default:
  @just --list

build:
  cargo build && cd web-client && trunk clean && trunk build

test: build
  cargo test

# Runs a web client with a proxy to the backend server (requires `just serve`)
run:
  cd web-client && trunk serve --open --proxy-backend=ws://127.0.0.1:8000/websocket --proxy-ws

# Runs a web client with a proxy to the production shuttle server
run-release:
  cd web-client && trunk serve --open --proxy-backend=wss://hanabi.shuttleapp.rs/websocket --proxy-ws

# Builds the WASM web client into `dist/``
build-release:
  cd web-client && trunk clean && trunk build --release

# Runs a local server
serve: build
  cargo shuttle run

# Deploys the server to the production shuttle server
release: build-release
  cargo shuttle deploy
