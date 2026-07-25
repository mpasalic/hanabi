## Context

Today the client is committed to a session from the very first websocket message: `CreateGame`, `Join`, or `Spectate`. The lobby has no "browsing" state, no list view, and no host concept (`OnlinePlayer.is_host` exists in the wire format but is hardcoded to `false` everywhere it's populated — `server/src/lobby.rs:63,113,152,163`).

The current state worth knowing:

- `server/src/app.rs` already separates websocket plumbing from lobby logic. `ServerStateSchema` owns a `client_map: HashMap<ClientId, LobbyClient>` (every connected socket) and a single `LobbyServer`. The websocket handler routes inbound messages to `lobby_server.message_received(&client, msg)` and calls `lobby_server.disconnected(client_id)` on socket close.
- `server/src/lobby.rs` owns `LobbyServer { game_lobbies: HashMap<SessionId, GameLobby> }`. Lobbies are in-memory; persistence (`Database`) is only written on `StartGame` (`db.create_game(...)` in the Start handler) and `PlayerAction`. There is **no** persistence for waiting lobbies — restart wipes them, which the proposal accepts.
- Disconnect handling already preserves player slots (`disconnected()` flips `connection` to `Disconnected` but does not remove the player). Reconnection via name match in `Join` already exists for any lobby state and is how players get their seats back today — the spec preserves this verbatim.
- Test infrastructure is in great shape: `server/tests/lobby.rs` drives `LobbyServer::message_received` directly via mpsc channels with `MemDatabase` (no websocket, no Postgres, no Docker). `server/tests/e2e_websocket.rs` covers the full pipe. `ratatui-app/tests/{render,component}_snapshots.rs` covers UI. New flows in this change should land tests in all three.

Constraints already accepted in the proposal:

- Wire protocol additions are **BREAKING** — no version negotiation today, no production user base beyond the maintainer; ok.
- Hostless waiting lobbies are a real state (host disconnected → no one can start → players wait). No host transfer.
- In-memory lobby list. No restart cleanup, no orphan-lobby reaping beyond what already happens.

## Goals / Non-Goals

**Goals**

- Connected-but-unattached "browsing" client state with server-pushed lobby list updates (no polling).
- Host designation that is permanent for the lobby's lifetime, populated meaningfully into the existing `OnlinePlayer.is_host` field, enforced on `StartGame`.
- `LeaveLobby` message so players can return to the browser without dropping their websocket.
- Lobby browser UI in `ratatui-app` (works in both `web-client` and the native CLI client).
- Tests at all three layers (in-process lobby, e2e websocket, UI snapshots).
- Implementation should land as a thin, reviewable set of stacked changes.

**Non-Goals**

- No auth, no name-uniqueness enforcement, no impersonation prevention (per user direction: trust users).
- No spectator entry from the browser. Existing spectate-by-URL path is untouched.
- No mid-game join / no rejoining-as-new-player to a `Playing` lobby (already rejected today; preserved).
- No restart-survivable lobby list. Lobbies remain in-process; restart empties the list.
- No auto-cleanup of stale empty/orphaned lobbies in this change (current behavior already leaves all-disconnected lobbies around; not making it worse, not fixing it here).
- No version negotiation in the wire protocol.

## Decisions

### Browsing clients live as a `HashSet<ClientId>` on `LobbyServer`

Add `browsers: HashSet<ClientId>` (or equivalently, `HashMap<ClientId, BrowserClient>` keyed by client_id) to `LobbyServer`. The websocket handler doesn't need to change — `LobbyServer::message_received` already gets `&LobbyClient` (which includes the mpsc sender), so on `EnterBrowser` the server just stashes the id, and on lobby-list push it looks up each browsing client in `ServerStateSchema.client_map` via the same path used for in-lobby clients.

The browser set has to live somewhere; alternatives considered:

- **Per-client status enum (`ClientStatus::Browsing | InLobby(SessionId)`) on `LobbyServer`**: rejected — duplicates information already encoded by the presence of a client_id in some lobby's player or spectator list.
- **A separate top-level field on `ServerStateSchema`**: rejected — the lobby browser is conceptually part of lobby concerns; keeping it inside `LobbyServer` keeps `app.rs` thin.

On socket disconnect, `LobbyServer::disconnected(client_id)` already runs; it just needs one extra line to `browsers.remove(&client_id)`.

### Host is identified by the player's display name, stored on `GameLobby`

Add `host_name: String` to `GameLobby`. Set it once when the lobby is created (from the creator's name). Never mutate it.

- Looking up "is this the host's currently-connected client?" becomes: `lobby.players.iter().find(|p| p.name == lobby.host_name).and_then(|p| connected_client_id(p)) == Some(caller_client_id)`.
- Naturally survives reconnection by name match (the host's slot gets a new connection; their `name` is unchanged, so the host check still resolves to them).
- Robust against future player-ordering changes (there's already a `// TODO need to change this once player order is randomized` in lobby.rs:564 — using index 0 would couple us to a thing that's about to change).

Alternatives considered:

- **Host = `players[0]` by convention**: rejected — fragile against the planned player shuffle; also implicit (host status is derived state rather than declared).
- **`host_client_id: ClientId`**: rejected — `ClientId` changes on reconnection (each socket gets a fresh id from `ServerStateSchema.clients_count`), so it would invalidate on every reconnect.

### Host-only `StartGame` checks identity AND connection state in one rule

The handler resolves "is the caller the currently-connected host" — if no, reject with `LobbyError::InvalidState`. This single rule covers:

- Non-host trying to start → caller's client_id doesn't match the host's connected client_id → reject.
- Host is disconnected → host's slot has `ConnectionState::Disconnected` → the host's connected client_id resolves to `None` → caller's id won't equal `Some(...)` → reject.

Error messages can be distinct (`"Only the host can start the game"` vs. `"Host is not connected"`) so the client can surface a useful toast, but the gate is one expression.

### Wire protocol additions (in `shared/src/client_logic.rs`)

New `ClientToServerMessage` variants:

```rust
EnterBrowser { player_name: String },
LeaveLobby,
```

New `ServerToClientMessage` variant:

```rust
LobbyList { lobbies: Vec<LobbySummary> },
```

New shared types:

```rust
pub struct LobbySummary {
    pub session_id: String,
    pub players: Vec<String>,        // display names, in lobby order
    pub status: LobbySummaryStatus,
}

pub enum LobbySummaryStatus {
    Waiting { host_name: String, host_connected: bool },
    Playing,                          // host info intentionally not exposed for Playing
}
```

`OnlinePlayer.is_host` continues to exist as-is; the server starts populating it correctly (true only for the player whose `name == lobby.host_name`).

Decisions in this shape:

- **`status` enum carries `host_name` / `host_connected` only inside `Waiting`**: Spec says playing-lobby entries are informational only. Putting host data inside the `Waiting` variant makes "no host info for playing lobbies" a type-level guarantee rather than a convention.
- **No `EnterBrowser` ack**: server sends `LobbyList` immediately after handling `EnterBrowser`; the client receives that as confirmation. One message instead of two.
- **`LeaveLobby` has no payload**: server resolves the caller's lobby via `get_lobby_session_for_client`, which already exists.
- **No `RefreshLobbyList`**: pushes are the only delivery mechanism, period. If we ever need a poll fallback we can add it later.

### When the lobby list is pushed

`LobbyServer` gains a private `broadcast_lobby_list_to_browsers()` that snapshots the current lobby set into `Vec<LobbySummary>` and sends `ServerToClientMessage::LobbyList { ... }` to every browser. It's called from:

| Event | Why |
|---|---|
| `CreateGame` succeeds | New waiting lobby appears |
| `Join` succeeds in a `Waiting` lobby | Roster + `host_connected` may change (host reconnecting) |
| `Join` succeeds in a `Playing` lobby | `host_connected` is N/A but listing is unchanged, so this push is technically unnecessary; skip |
| `LeaveLobby` (non-host) | Roster changes |
| `LeaveLobby` (host) | Lobby disappears entirely |
| `StartGame` transitions Waiting → Playing | Status flips |
| `disconnected(client_id)` removes a connection from any waiting lobby | `host_connected` may flip; also any non-host disconnect changes nothing structurally but is cheap to push |
| Game transitions to `Ended` (in `PlayerAction` when outcome lands) | Removes from list |

The "broadcast on every relevant event" approach is the simplest correct thing. At current scale (single-digit concurrent users) the fan-out is trivial; if we ever care, we can debounce or diff inside `broadcast_lobby_list_to_browsers` later.

### `LeaveLobby` semantics

The handler resolves the caller's lobby (existing `get_lobby_session_for_client`), then:

- If lobby is in `Playing` or `Ended` state: reject. Leaving a game in progress is out of scope (per spec "Non-Host Leaves A Waiting Lobby" — waiting only).
- If the caller is **not** the host: remove their `SocketPlayer` from the player list, broadcast updated lobby state to remaining players, broadcast updated lobby list to browsers, add the leaver to `browsers` and send them the current `LobbyList`.
- If the caller **is** the host: remove the lobby entirely, broadcast a `LobbyList` push to all browsers (the lobby is gone), broadcast some signal to the remaining-player clients that the lobby ended (see next decision), and add the leaver to `browsers` and send them the current `LobbyList`.

### "Lobby was destroyed by host" signal to remaining players

When the host explicitly leaves a waiting lobby with other players present, those players need to know "this is over, you're back at the browser." Two viable approaches:

- **(a)** Server pushes a fresh `LobbyList` to those clients and adds them to `browsers` automatically. Client treats receiving `LobbyList` while in waiting-room state as "the lobby is gone, switch to browser view."
- **(b)** New explicit `ServerToClientMessage::LobbyClosed` variant.

Going with **(a)** for now: receiving `LobbyList` is already the implicit "you are now browsing" signal (the only context that ever sees `LobbyList` is a browser). When the server destroys a host-left lobby, it migrates the remaining clients into `browsers` and pushes them the list. The waiting-room UI sees `LobbyList` arrive, transitions to browser view.

This is one fewer message variant, and centralizes the "you are now in browser state" rule on the client (whenever you get a LobbyList, you're a browser).

### Client state machine (`web-client/src/main.rs::TuiState`)

Today:

```
AppInput  →  CreatingGame  →  HanabiApp (with redirect URL)
          ↘                 ↗
            HanabiApp (joined via session_id from URL)
```

New:

```
                       ┌────────────────────────────────────┐
                       │                                    │
AppInput  →  Browser  ─┼─►  CreatingGame  →  HanabiApp      │
                       │                                    │
                       └─►  HanabiApp (Join from list)     │
                                                            │
HanabiApp (joined via URL)  ←──────────────────────────────┘
                       (URL bypass keeps current direct-to-HanabiApp flow)
```

- `TuiState::Browser { player_name, server_address, current_list }` — connected, receiving `LobbyList` pushes, presents the browser UI.
- When the user clicks "Create" the websocket stays open, the client sends `CreateGame`, on `CreatedGame` reply it does the existing URL-redirect dance. (Could also avoid the redirect — but preserving the redirect keeps the "share the link" flow exactly as-is.)
- When the user clicks "Join" on a list entry the client sends `Join { player_name, session_id }`, transitions to `HanabiApp`.
- When arriving via `?session_id=...` in the URL, skip browser entirely (current direct-to-HanabiApp behavior preserved).
- In `HanabiApp` (waiting room), the existing "Start Game" UI affordance is shown only when the local player's `OnlinePlayer.is_host` is true.

Browser is the **only** state that opens a websocket purely to listen (no session). All other states are already tied to a session.

### Browser UI sketch (in `ratatui-app`)

A new module `ratatui-app/src/browser.rs` (or extending `input_app.rs`) renders:

```
┌─ Lobby Browser ─ welcome, alice ─────────────────┐
│                                                  │
│   [+ Create new game]                            │
│                                                  │
│   Waiting for players                            │
│   ──────────────────────────────────────────     │
│   ▸ alice (host)             1/5    [Join]       │
│   ▸ bob, charlie             2/5    [Join]       │
│   ▸ dan, eve (host disc)     2/5    [Join]       │
│                                                  │
│   In progress                                    │
│   ──────────────────────────────────────────     │
│   · fred, grace, henry       (playing)           │
└──────────────────────────────────────────────────┘
```

`(host disc)` annotation appears when `host_connected == false`. Playing lobbies render in a muted style and have no action affordance.

Snapshot tests cover: empty list, list with only waiting, list with mix of waiting+playing, hostless waiting lobby, and the waiting-room screen rendered as host vs. non-host.

### Migration / deployment

No DB schema changes. Wire protocol is breaking; deploy server and client together. Anyone with a stale tab gets a JSON deserialization error on the server (already logged via `tracing::warn!` in `client_msg`) and the message is dropped — annoying but not damaging; a refresh fixes it. Acceptable per the proposal's "no production user base" framing.

### Test plan

- `server/tests/lobby.rs` — add cases:
  - `enter_browser_returns_current_lobby_list`
  - `creating_a_lobby_pushes_list_to_existing_browsers`
  - `host_designation_visible_in_lobby_state` (verifies `OnlinePlayer.is_host` is true for creator)
  - `non_host_start_game_is_rejected`
  - `start_game_while_host_disconnected_is_rejected`
  - `host_disconnect_does_not_transfer`
  - `non_host_leave_lobby_removes_player`
  - `host_leave_lobby_destroys_lobby_and_browsers_clients`
  - `name_match_join_reclaims_slot` (regression — preserve existing reconnection)
- `server/tests/e2e_websocket.rs` — add at least one round-trip of `EnterBrowser` → `LobbyList` + `LeaveLobby` → `LobbyList` to lock in the wire format.
- `ratatui-app/tests/render_snapshots.rs` — snapshots listed in the UI section above.
- `shared/tests/client_logic_parity.rs` — already covers serialization parity; new variants are picked up automatically.

### Implementation order (suggested stacked PRs)

This is a recommendation, not a spec — but landing in this order keeps each step independently mergeable and testable:

1. **shared**: wire format additions only (new `ClientToServerMessage` / `ServerToClientMessage` variants, `LobbySummary`). Compiles, no behavior change. Serialization parity test forces the rest to match.
2. **server**: host designation + host-only `StartGame` (no browser yet). `OnlinePlayer.is_host` starts being meaningful; `StartGame` returns an error to non-hosts. Tests as listed.
3. **server**: browser state + `EnterBrowser` / `LeaveLobby` / `LobbyList` push triggers.
4. **client (ratatui-app)**: browser screen module + waiting-room conditional Start button.
5. **client (web-client)**: new `TuiState::Browser` variant; route post-name-entry through it when no `session_id` query param; preserve URL-bypass to `HanabiApp`.

## Risks / Trade-offs

- **Wire format break with no version handshake** → existing tabs will silently fail after deploy. Mitigation: nothing — refresh-after-deploy is fine for this user base. If we want better someday: add a `version` field to the first client message and reject mismatches with a typed error the client can render.
- **Stale / hostless lobbies linger forever** → an all-disconnected waiting lobby (or a host-disconnected lobby that nobody comes back to) stays in `game_lobbies` until process restart, and clutters the browser list. Mitigation: out of scope here. Cheap follow-up later is a per-lobby `last_activity: Instant` and a sweep on `EnterBrowser` (no background timers needed) to drop lobbies idle past N minutes.
- **Host-leave destroys the lobby with no undo** → an accidental click bounces all players to the browser. Mitigation: client-side confirmation modal on host's leave action; label it "End Lobby" (not "Leave"). Server contract is correct regardless.
- **Fan-out cost of unconditional lobby-list pushes** → at scale (100s of browsers, 10s of changes/sec) we'd push 1000s of msgs/sec. Mitigation: not a concern at single-digit user scale; if needed, debounce inside `broadcast_lobby_list_to_browsers` or diff against the last sent payload before sending.
- **Trust-users model for name collisions** → two players honestly picking the same name can stomp each other's slots silently. Spec is explicit about this. Mitigation: none; UI could show a warning when joining a lobby that already contains a player with your chosen name (a polite "are you reconnecting or a different person?") — opt-in follow-up, not required here.
- **Host coupling to `host_name` string** → if we ever support renaming a player mid-lobby, the host pointer would dangle. We don't support rename today; flag it in code comments next to `host_name`.

## Open Questions

- **Should host-leave actually destroy the lobby, or should it just leave the lobby hostless and stuck?** Currently spec'd as destroy. Alternative (leave-it-hostless) is simpler server-side but leaves the orphan permanently un-startable, which seems worse. Pushable.
- **Should the lobby list show *playing* lobbies at all in v1?** Spec says yes (informational). They're not actionable — could be argued they add noise. Keeping them for now to match the proposal's "give users a sense of activity" intent; trivial to hide later.
- **Should we add a "lobby created at" timestamp to `LobbySummary` for sort order / age display?** Could help the browser ("alice's game, 12s old"). Out of scope per the minimal spec; easy to add later if the UI wants it.
