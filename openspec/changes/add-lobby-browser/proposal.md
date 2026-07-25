## Why

Today, the only way to join a Hanabi game is for the host to share the session URL out-of-band — there is no way to discover that someone is already waiting for players. This makes the most common scenario (showing the project to a group at a hangout, where several people want to join a game I just created) needlessly awkward. A simple, push-driven lobby browser would make joining a game as easy as picking it from a list, while keeping the existing share-the-link flow intact for cases where it works fine.

## What Changes

- Add a pre-game **browsing** state where a client is connected to the server but has not yet committed to a session. Today the websocket's first message must be `CreateGame`, `Join`, or `Spectate`; this change makes "I'm here, what's available?" a real state.
- Add a server-pushed **lobby list** that browsing clients receive on entry and on every change (lobby created, player joined/left, game started, lobby ended). No polling.
- Add a new **lobby browser screen** in the client between name entry and joining/creating a game. Lists waiting games (joinable) and in-progress games (informational only — not joinable, not spectatable from this screen).
- Introduce **host designation**: the player who creates a game is the host. The currently-unused `OnlinePlayer.is_host` field gets populated. The server enforces host-only `StartGame`. The host designation is permanent for the lobby's lifetime — it never transfers. If the host disconnects, the lobby is temporarily without a connected host (no one can start) until the host reconnects; if the host explicitly leaves, the lobby is removed entirely and remaining players bounce back to the browser.
- Add an explicit **leave-lobby** action so a player can return to the browser screen without dropping their websocket connection. Available to non-host players (returns them to browsing); for the host it removes the lobby (see above).
- Preserve current **join-by-name** semantics: a join whose display name matches an existing player in the target lobby reassigns that player's slot to the joining client (this is how reconnection works today and continues to be how it works). The server does not attempt to distinguish accidental name collisions from intentional reconnections — clients are expected to choose distinct names within a lobby.
- Preserve the existing flows: creating a game still redirects to the session-URL page (so the link remains shareable); join-by-URL still works; spectate-by-URL still works.
- **BREAKING (wire protocol)**: `ClientToServerMessage` and `ServerToClientMessage` gain new variants. `CreateGame` may grow optional fields. Any already-connected client built against the old protocol breaks — acceptable since there is no client versioning today and no production user base beyond the maintainer.

Explicit non-goals for this change:
- No spectating from the lobby browser (the existing share-the-URL spectate path is unchanged; revisit only after the "spectator sees everyone's cards" footgun is addressed)
- No auth, no join codes, no matchmaking, no game-visibility toggle
- No mid-game joining (Hanabi's roster is locked when the game starts)
- No persistence of the lobby list across server restarts (in-memory is fine; Fly auto-stop wipes the list, which matches reality of who's actually connected)

## Capabilities

### New Capabilities

- `lobby`: All pre-game multiplayer coordination. Covers the browsing state and lobby list, joining/creating/leaving a waiting lobby, host designation and transfer, and host-only game start. The in-game protocol (`PlayerAction`, snapshots, etc.) remains outside this capability.

### Modified Capabilities

None. `openspec/specs/` is empty today; existing pre-game protocol behavior (`CreateGame`, `Join`, `StartGame`) is being absorbed into the new `lobby` capability rather than modifying a prior spec.

## Impact

**Wire protocol** (`shared/src/client_logic.rs`)
- New `ClientToServerMessage` variants: `EnterBrowser`, `LeaveLobby` (exact names TBD in design)
- New `ServerToClientMessage` variant for pushing the lobby list
- New `LobbySummary` type for list entries
- `OnlinePlayer.is_host` starts being populated meaningfully (today always `false`)

**Server** (`server/src/lobby.rs`)
- `LobbyServer` tracks "browsing" clients separately from per-lobby clients, so it can push list updates to them
- `GameLobby` gains a host concept — host = the player who created the lobby; designation is permanent and never reassigned
- `StartGame` handler enforces host identity AND requires the host's connection to currently be connected (rejects start while host is disconnected)
- Lobby list push fires on every state change that affects what a browser would see (create / join / leave / start / host connection-state change)
- Explicit `LeaveLobby` for the host removes the lobby entirely; for non-hosts it removes only their player slot (current "name match = reconnection" semantics on `Join` are preserved unchanged)

**Client UI** (`ratatui-app/`, `web-client/`)
- New lobby browser screen in `ratatui-app` (sits between current name-entry and the in-game UI)
- New `TuiState` variant in `web-client` for the browser
- Websocket lifecycle changes: socket opens earlier (right after name entry, in browser mode) rather than at session commit
- Waiting-room UI conditionally shows "Start Game" only to the host; non-hosts see "Waiting for {host} to start..."

**No DB schema changes.** Lobbies are in-memory; the `game_config` / `player` / `game_log` tables are only touched when a game actually starts (unchanged from today).

**Tests touched**: `server/tests/lobby.rs` (new flows, host enforcement, duplicate-name), `server/tests/e2e_websocket.rs` (new message round-trips), `ratatui-app/tests/` snapshots (new browser screen + host-vs-non-host waiting-room states). No new test infrastructure required.
