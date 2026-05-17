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

# Start a local Postgres 17 in Docker. Data persists in the `hanabi-pg-data` volume.
# Use this DATABASE_URL: postgresql://postgres:postgres@127.0.0.1:5432/hanabi
db:
  @docker start hanabi-pg 2>/dev/null || docker run -d --name hanabi-pg \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_DB=hanabi \
    -p 5432:5432 \
    -v hanabi-pg-data:/var/lib/postgresql/data \
    postgres:17
  @echo "Postgres running at postgresql://postgres:postgres@127.0.0.1:5432/hanabi"

# Stop the local Postgres container (data preserved in the volume)
db-stop:
  docker stop hanabi-pg

# Destroy the local Postgres container AND wipe its data
db-reset:
  -docker stop hanabi-pg
  -docker rm hanabi-pg
  -docker volume rm hanabi-pg-data
