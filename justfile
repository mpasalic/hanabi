set dotenv-load := true

# --- Per-worktree defaults ---
# Each git worktree gets its own server port, trunk port, and Postgres database
# inside a shared `hanabi-pg` container — all derived from a hash of the
# working directory. Override any of these in the worktree's `.env` if you ever
# hit a collision. Run `just info` to see the resolved values.
worktree_slug := `printf %s "${PWD##*/}" | tr -c 'a-zA-Z0-9_' '_' | tr '[:upper:]' '[:lower:]'`

export PORT := env_var_or_default("PORT", `printf '%d' $((18000 + 0x$(pwd | shasum -a 256 | head -c 8) % 1000))`)
export TRUNK_PORT := env_var_or_default("TRUNK_PORT", `printf '%d' $((19000 + 0x$(pwd | shasum -a 256 | head -c 8) % 1000))`)
export WORKTREE_DB := env_var_or_default("WORKTREE_DB", "hanabi_" + worktree_slug)
export DATABASE_URL := env_var_or_default("DATABASE_URL", "postgresql://postgres:postgres@127.0.0.1:5432/" + WORKTREE_DB)

default:
  @just --list

# Show resolved worktree config (run this if you're unsure what's bound where)
info:
  @echo "Worktree:     $PWD"
  @echo "Slug:         {{worktree_slug}}"
  @echo "PORT:         $PORT       (server)"
  @echo "TRUNK_PORT:   $TRUNK_PORT (web dev server)"
  @echo "WORKTREE_DB:  $WORKTREE_DB"
  @echo "DATABASE_URL: $DATABASE_URL"

build:
  cargo build && cd web-client && trunk clean && trunk build

test: build
  cargo test

# Runs a web client with a proxy to the local backend server (requires `just serve`)
run:
  cd web-client && trunk serve --open --port $TRUNK_PORT \
    --proxy-backend=ws://127.0.0.1:$PORT/websocket --proxy-ws

# Runs a web client with a proxy to the deployed Fly.io server
run-release:
  cd web-client && trunk serve --open --port $TRUNK_PORT \
    --proxy-backend=wss://hanabi-tui.fly.dev/websocket --proxy-ws

# Builds the WASM web client into `dist/`
build-release:
  cd web-client && trunk clean && trunk build --release

# Runs the server locally on $PORT against $DATABASE_URL.
# `db-init` ensures this worktree's database exists in the shared container.
serve: build db-init
  cargo run -p server

# Deploys to Fly.io. Requires `flyctl` auth and a configured app (see README).
release: build-release
  flyctl deploy

# Tail Fly.io app logs
logs:
  flyctl logs

# Start the shared local Postgres 17 container (one container for ALL worktrees).
# Data persists in the `hanabi-pg-data` volume.
db:
  @docker start hanabi-pg 2>/dev/null || docker run -d --name hanabi-pg \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_DB=postgres \
    -p 5432:5432 \
    -v hanabi-pg-data:/var/lib/postgresql/data \
    postgres:17
  @echo "Postgres running at postgresql://postgres:postgres@127.0.0.1:5432"

# Ensure this worktree's database exists inside the shared container.
# Idempotent. Auto-runs as a dep of `serve`.
db-init: db
  @docker exec hanabi-pg sh -c "until pg_isready -U postgres >/dev/null 2>&1; do sleep 0.2; done"
  @docker exec -e PGPASSWORD=postgres hanabi-pg \
    psql -U postgres -tc "SELECT 1 FROM pg_database WHERE datname='$WORKTREE_DB'" \
    | grep -q 1 || \
    docker exec -e PGPASSWORD=postgres hanabi-pg \
      psql -U postgres -c "CREATE DATABASE \"$WORKTREE_DB\""
  @echo "Database $WORKTREE_DB ready"

# Drop this worktree's database only (container keeps running, other worktrees unaffected)
db-drop:
  @docker exec -e PGPASSWORD=postgres hanabi-pg \
    psql -U postgres -c "DROP DATABASE IF EXISTS \"$WORKTREE_DB\""

# Stop the shared Postgres container (affects ALL worktrees; data preserved in the volume)
db-stop:
  docker stop hanabi-pg

# Destroy the shared Postgres container AND wipe ALL data (affects ALL worktrees)
db-reset:
  -docker stop hanabi-pg
  -docker rm hanabi-pg
  -docker volume rm hanabi-pg-data
