# Testing

This document covers every testing layer in the repo, when to use each, and the
exact commands/files involved. It's written for engineers and agents who are
about to make a change and need to know **where the test should go**.

If you only read one section: jump to [Decision tree](#decision-tree).

## TL;DR

| Layer | Purpose | Where | Speed |
|---|---|---|---|
| **Unit (game logic)** | Pure rule correctness, single action | `shared/src/logic.rs` `#[test]` | <1ms |
| **Property (invariants)** | "X is never true after any move sequence" | `shared/tests/proptest_invariants.rs` | ~100ms |
| **Client-server parity** | Optimistic client mutation matches server | `shared/tests/client_logic_parity.rs` | <1ms |
| **Component snapshot** | One UI piece renders correctly | `ratatui-app/tests/component_snapshots.rs` | <10ms |
| **Full-screen snapshot** | Whole `HanabiApp` for a given state | `ratatui-app/tests/render_snapshots.rs` | <10ms |
| **In-memory server** | `LobbyServer` message handling, no DB | `server/tests/lobby.rs` | <50ms |
| **Postgres hydration** | Save → restart → replay from DB | `server/tests/hydration_pg.rs` (gated) | ~100ms |
| **End-to-end websocket** | Full Axum + JSON wire + 2 clients | `server/tests/e2e_websocket.rs` | ~200ms |

Run everything: `just test` (or `cargo test --workspace`).

Run a single layer: `cargo test -p <crate> --test <test_file>`.

## Decision tree

**What did you change?**

- **Game rules** (`shared/src/logic.rs`, `shared/src/model.rs` — `play()`, effects,
  `run_effects`, outcome logic)
  → Add a `#[test]` in `shared/src/logic.rs::tests` for the specific case
  → If it's a new *invariant* ("this can never happen"), also add it to
    `shared/tests/proptest_invariants.rs`

- **Client-side optimistic UI update** (`shared/src/client_logic.rs`,
  particularly `GameStateSnapshot::apply_local_mutation`)
  → Add a case in `shared/tests/client_logic_parity.rs` that cross-checks
    against the authoritative engine for the same action

- **A UI component** in `ratatui-app/src/components.rs` (a card, a slot, a
  player panel, the board, the log…)
  → Add a `#[test]` in `ratatui-app/tests/component_snapshots.rs` that renders
    just that component

- **The overall screen layout / state-to-UI dispatch** in
  `ratatui-app/src/hanabi_app.rs` (`ui()`, `game_ui()`, `lobby_ui()`,
  `connecting_ui()`)
  → Add a full-screen test in `ratatui-app/tests/render_snapshots.rs`

- **Server message handling** (`server/src/lobby.rs`, `message_received`,
  `disconnected`, `broadcast_game_state`)
  → Add a test in `server/tests/lobby.rs` using `MemDatabase`. Drive
    `message_received` directly, assert on what each client receives via
    its mpsc receiver.

- **Persistence / hydration / migrations** (`server/src/model.rs`,
  `server/migrations/*.sql`, hydrate replay loop in `server/src/lobby.rs`)
  → Add a test in `server/tests/hydration_pg.rs`. Requires
    `TEST_DATABASE_URL`; locally that's `just db-init && export
    TEST_DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:5432/hanabi_hanabi`.

- **The wire protocol** (`ClientToServerMessage` / `ServerToClientMessage`
  enums in `shared/src/client_logic.rs`, JSON encoding, websocket handshake)
  → Add a scenario to `server/tests/e2e_websocket.rs` that drives real
    `tokio-tungstenite` clients.

- **The Axum router / websocket upgrade / connection lifecycle**
  (`server/src/app.rs`)
  → Same: extend `server/tests/e2e_websocket.rs`.

- **`web-client` / WASM shell / egui adapter** (`web-client/`)
  → Currently **no automated coverage**. Verify by hand: `just serve` in one
    terminal, `just run` in another, open the browser.

**When in doubt, prefer the cheapest layer that can actually catch the
regression you care about.** Unit tests don't catch wire-protocol drift;
end-to-end tests are slow and noisy for a one-line logic bug.

## Layer reference

### 1. Unit tests on game logic (`shared/src/logic.rs`)

The original test layer — 26 `#[test]` cases inside `logic.rs::tests`. Each
constructs a `GameState` directly with hand-picked cards and asserts on the
output of `play()` or `run_effects()`.

**Add to this layer when:** you're changing how a single `PlayerAction` is
validated or what effects it produces.

**Run:** `cargo test -p shared logic::tests`

**Pattern:**
```rust
#[test]
fn test_my_new_effect() {
    let mut state = /* construct GameState */;
    let result = state.play(PlayerAction::PlayCard(SlotIndex(0)));
    assert_eq!(result, Ok(vec![/* expected GameEffect sequence */]));
}
```

### 2. Property tests on invariants (`shared/tests/proptest_invariants.rs`)

Generates random `PlayerAction` sequences (`proptest`) and asserts game-state
invariants after each step. Invalid actions are silently skipped — the engine
returns `Err` and the proptest moves on. Currently checks:

- `remaining_bomb_count <= num_fuses`
- `played_cards.len() <= 25`
- Per suit, played faces form a strict `1..=N` progression
- `Win` outcome ⇒ 25 played cards

**Add to this layer when:** you want to assert "this can never happen" across
*any* reachable state, not just a hand-picked one. Particularly valuable when
you're tightening a rule (e.g. "after this fix, the score is never 26").

**Run:** `cargo test -p shared --test proptest_invariants`

**Pattern:** add another `prop_assert!` inside the existing test. To add a new
property, copy the `proptest! { #[test] fn ... }` block.

**Caveat:** if you add an invariant that *currently* fails, proptest will
shrink to a minimal failing case and report it — useful for finding bugs, but
fix the bug or weaken the invariant before merging.

### 3. Client-server parity (`shared/tests/client_logic_parity.rs`)

Cross-checks `GameStateSnapshot::apply_local_mutation` (the client's
optimistic preview) against the authoritative `shared::logic` engine. Today
only `PlayerAction::MoveSlot` has a local mutation; if you add another, write
the parity test the same day or you will see UI flicker in production.

**Add to this layer when:** you teach the client to optimistically preview a
new `PlayerAction` variant before the server confirms.

**Run:** `cargo test -p shared --test client_logic_parity`

**Pattern:**
```rust
let optimistic = /* apply_local_mutation on the snapshot */;
let authoritative = /* GameLog::log + into_client_game_state */;
assert_eq!(optimistic, authoritative);
```

### 4. Component snapshots (`ratatui-app/tests/component_snapshots.rs`)

Renders a single `Node<'static>` returned by any public function in
`ratatui-app/src/components.rs` and snapshots the resulting text. Smallest
possible UI test — one card is 5×3.

**Add to this layer when:** you change how a specific component is laid out
or styled (e.g. card colors, hint glyphs, slot spacing).

**Run:** `cargo test -p ratatui-app --test component_snapshots`

**Pattern:**
```rust
let node = card_node(&CardProps { /* ... */ });
insta::assert_snapshot!(render_node(node, 5, 3));
```

The `render_node(node, w, h)` helper at the top of the file does the
`compute_layout` + `render_ref` dance.

**Components currently testable in isolation** (all return `Node<'static>`):
`card_node`, `slot_node`, `player_node`, `played_cards_tree`,
`discarded_cards_tree`, `board_stats_node_tree`, `draw_pile`,
`board_node_tree`, `game_log_tree`, `card_pile`.

### 5. Full-screen snapshots (`ratatui-app/tests/render_snapshots.rs`)

Renders the entire `HanabiApp` for a given `HanabiClient` state against a
`TestBackend`. No egui, no WASM. Three snapshots today: connecting screen,
lobby with 2 players, mid-game.

**Add to this layer when:** you change how multiple components compose, the
top-level layout, or how state variants (`Connecting` / `Lobby` / `Playing` /
`Spectating` / `Ended`) dispatch to UIs.

**Run:** `cargo test -p ratatui-app --test render_snapshots`

**Pattern:**
```rust
let state = HanabiClient::Loaded(HanabiGame::Playing { /* ... */ });
insta::assert_snapshot!(render(state, 156, 38));
```

Use builders from `shared::test_data` to construct the state — see
[`GameStateSnapshotBuilder`](#test-fixtures-shared-fixtures-and-builders).

### 6. In-memory server (`server/tests/lobby.rs`)

Constructs a `LobbyServer` with `MemDatabase` (no Docker, no Postgres) and
drives `message_received` directly. Each "client" is just an mpsc receiver
wrapped in `LobbyClient`.

**Add to this layer when:** you change how `LobbyServer` reacts to a
`ClientToServerMessage` — broadcasting the right `ServerToClientMessage`,
updating the in-memory lobby state, handling reconnects, etc.

**Run:** `cargo test -p server --test lobby`

**Pattern:**
```rust
let db: Arc<dyn Database> = Arc::new(MemDatabase::new());
let mut server = LobbyServer::new(db);
let mut alice = fake_client(1);

server
    .message_received(&alice.handle, ClientToServerMessage::CreateGame {
        player_name: "alice".into(),
    })
    .await?;

let msg = alice.rx.recv().await.unwrap();
assert!(matches!(msg, ServerToClientMessage::CreatedGame { .. }));
```

The `fake_client(id) -> FakeClient { handle, rx }` helper at the top of the
file does the mpsc wiring.

### 7. Postgres hydration (`server/tests/hydration_pg.rs`)

The only test that hits real SQL. Seeds the DB via `PgDatabase`, drops the
in-memory `LobbyServer`, creates a fresh one, and has a saved player rejoin —
which forces `hydrate()` to replay the action log.

**Add to this layer when:** you change `model.rs` SQL, the schema in
`migrations/*.sql`, the hydration replay loop in `LobbyServer::hydrate`, or
the JSON encoding of `PlayerAction`.

**Run locally:**
```sh
just db-init  # ensures the shared hanabi-pg container + worktree DB
TEST_DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:5432/hanabi_hanabi \
  cargo test -p server --test hydration_pg
```

Without `TEST_DATABASE_URL`, the test silently no-ops (prints `skipping…`).
CI sets it via a Postgres service container.

**Pattern:** populate the DB through the `Database` trait, then create a new
`LobbyServer` against the same pool and assert on what a rejoining client
sees.

### 8. End-to-end websocket (`server/tests/e2e_websocket.rs`)

Spawns the actual Axum router via `build_router()` on `127.0.0.1:0`, connects
real `tokio-tungstenite` clients, and sends JSON over the wire. Uses
`MemDatabase` so it stays hermetic.

**Add to this layer when:** you change the wire protocol, the websocket
upgrade flow, the connection lifecycle, or anything in `server/src/app.rs`.

**Run:** `cargo test -p server --test e2e_websocket`

**Pattern:** the `spawn_server() -> (port, JoinHandle)`, `connect(port)`,
`send_msg`, and `drain_until_game_state` helpers at the top of the file do
the plumbing. New scenarios are usually a sequence of send/receive pairs.

This is the slowest layer — prefer in-memory `lobby.rs` tests when the
websocket-specific bits aren't the thing you're testing.

## Test fixtures: `shared::test_data`

All test fixtures and the fluent builder live in `shared/src/test_data.rs`,
gated behind the `test-helpers` Cargo feature. Any crate that wants them
declares:

```toml
[dev-dependencies]
shared = { path = "../shared", features = ["test-helpers"] }
```

It's already declared for `ratatui-app` and `server`.

Available helpers:

- `generate_minimal_test_game_state()` → an empty-but-valid 2-player
  `GameStateSnapshot`
- `generate_example_panic_case_1()` → a rich mid-game `HanabiGame::Playing`
  captured from a real session, useful for snapshotting non-trivial UI
- `playing_game(session_id, snapshot) -> HanabiGame` → wraps a snapshot in
  throwaway lobby metadata
- `GameStateSnapshotBuilder` → fluent overrides for one-off scenarios:
  ```rust
  let snap = GameStateSnapshotBuilder::new()
      .with_played(CardSuit::Red, CardFace::One)
      .with_bomb_count(2)
      .build();
  ```

**Adding new fixtures:** put them here, not in the test file that consumes
them. Other tests will want them later.

## Common agent workflows

### "I changed the UI. Tests fail with a giant text diff. Now what?"

The snapshot is in `tests/snapshots/`. The diff is showing you what changed.

- **If the change is intentional:** `cargo insta accept` (or `cargo insta
  review` for an interactive walk-through). Commit the updated `.snap`.
- **If the change is unintentional:** fix the underlying code, re-run.

`cargo insta` is in `Cargo.lock`; install once with `cargo install
cargo-insta` if you don't have it.

You can also blanket-accept on first run by setting `INSTA_UPDATE=always
cargo test ...` — this is how to bootstrap a brand new snapshot file.

### "Tests pass locally but break on CI"

Likely paths:

1. **`hydration_pg` is skipping locally** because you don't have
   `TEST_DATABASE_URL` set — CI does. Run it locally with the env var to
   reproduce.
2. **clippy / fmt** — CI runs them as advisory (currently `continue-on-error`
   in `.github/workflows/integration.yml`). They won't block, but pay
   attention to the output.
3. **A new dev-dep needs the `test-helpers` feature** — if a new crate's
   tests use `shared::test_data`, declare `shared = { path = "../shared",
   features = ["test-helpers"] }` under `[dev-dependencies]`.

### "I'm adding a new `PlayerAction` variant"

Touch these in order:

1. `shared/src/model.rs` — define the variant
2. `shared/src/logic.rs` — handle it in `play()`, add a `#[test]`
3. `shared/tests/proptest_invariants.rs` — extend `arb_action()` to generate
   the new variant
4. **If it has an optimistic client preview:** `shared/src/client_logic.rs`
   `apply_local_mutation` + a parity test in
   `shared/tests/client_logic_parity.rs`
5. `ratatui-app/src/hanabi_app.rs` — handle it in `handle_action` /
   `process_app_action`
6. Snapshot test exposure as needed in `render_snapshots.rs`

### "I'm adding a new `ClientToServerMessage` or `ServerToClientMessage`"

This is a wire-protocol change — there's no versioning, so connected clients
will break across the change.

1. `shared/src/client_logic.rs` — add the variant
2. `server/src/lobby.rs` — handle (for C2S) or emit (for S2C) it in
   `LobbyServer`
3. `server/tests/lobby.rs` — in-memory test for the handler logic
4. `server/tests/e2e_websocket.rs` — at least one scenario that exercises the
   JSON encoding on the real wire

### "I just want the fastest possible feedback loop"

```sh
cargo test -p shared       # ~1s, covers ~75% of test count
```

Or scope to one file:

```sh
cargo test -p shared --test client_logic_parity
```

Avoid `cargo test --workspace` during inner-loop iteration — it rebuilds the
WASM bits and the server binary; not what you want.

### "Tests I'm running take Postgres setup that I don't have"

```sh
just db          # starts the shared hanabi-pg container
just db-init     # creates this worktree's database in it
```

The `just db-init` step is idempotent. After that, set
`TEST_DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:5432/$WORKTREE_DB`
(or `hanabi_hanabi` for the default worktree).

## What's *not* covered

- **`web-client/`** has no automated tests. Egui/eframe coupling makes
  headless testing high effort for low value right now. Verify changes
  manually in a browser.
- **The Fly.io deployment** isn't tested in CI; `deploy-fly.yml` is manual
  (`workflow_dispatch`).
- **Performance** — there's no benchmark suite. If something feels slow, use
  `cargo bench` ad-hoc or profile with a tool of your choice.
