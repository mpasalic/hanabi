## 1. Wire protocol additions (`shared` crate)

- [ ] 1.1 Add `LobbySummary` struct and `LobbySummaryStatus` enum (`Waiting { host_name, host_connected }` | `Playing`) to `shared/src/client_logic.rs`
- [ ] 1.2 Add `ClientToServerMessage::EnterBrowser { player_name: String }` variant
- [ ] 1.3 Add `ClientToServerMessage::LeaveLobby` variant (no payload)
- [ ] 1.4 Add `ServerToClientMessage::LobbyList { lobbies: Vec<LobbySummary> }` variant
- [ ] 1.5 Confirm `shared/tests/client_logic_parity.rs` still passes; add coverage for new variants if the existing test doesn't already round-trip every variant

## 2. Host designation + host-only StartGame (`server` crate)

- [ ] 2.1 Add `host_name: String` field to `GameLobby` in `server/src/lobby.rs`; set it in the `CreateGame` handler from the creating player's name; never mutate elsewhere
- [ ] 2.2 In `GameLobby::broadcast_game_state` and `broadcast_player_state`, populate `OnlinePlayer.is_host` as `p.name == self.host_name` (currently always `false` at `lobby.rs:63,113,152,163`)
- [ ] 2.3 In the `StartGame` handler: resolve the host's currently-connected client_id by finding the player whose `name == host_name` and inspecting their `ConnectionState`. Reject with `LobbyError::InvalidState` if the caller's `client_id` does not match. Use distinguishable error messages for "non-host" vs. "host disconnected"
- [ ] 2.4 `server/tests/lobby.rs`: add `host_designation_visible_in_lobby_state` (creator's `is_host` is `true`; others `false`)
- [ ] 2.5 `server/tests/lobby.rs`: add `non_host_start_game_is_rejected`
- [ ] 2.6 `server/tests/lobby.rs`: add `start_game_while_host_disconnected_is_rejected` (host creates, host disconnects via `LobbyServer::disconnected`, another player attempts `StartGame`)
- [ ] 2.7 `server/tests/lobby.rs`: add `host_disconnect_does_not_transfer` (host disconnects, other players' `is_host` remain `false`; only the host slot is host)
- [ ] 2.8 `server/tests/lobby.rs`: regression test `name_match_join_reclaims_slot` (preserve existing reconnection-by-name behavior in both Waiting and Playing lobbies)

## 3. Browser state + `EnterBrowser` / `LeaveLobby` / lobby-list push (`server` crate)

- [ ] 3.1 Add `browsers: HashSet<ClientId>` to `LobbyServer` and initialize it in `LobbyServer::new`
- [ ] 3.2 Add private `LobbyServer::current_lobby_list(&self) -> Vec<LobbySummary>` that builds the summaries from `self.game_lobbies`, excluding any in `Ended` state, and inlining `host_name` + `host_connected` for `Waiting` lobbies
- [ ] 3.3 Add private `LobbyServer::broadcast_lobby_list_to_browsers(&self, client_map: ...)` that snapshots the list once and sends `ServerToClientMessage::LobbyList` to every browsing client. Pass in whatever access to `LobbyClient.sender` is needed (likely a borrow of `ServerStateSchema.client_map`, or store senders inside the browsers map — pick the simplest shape that keeps `app.rs` thin)
- [ ] 3.4 Implement the `EnterBrowser` handler: insert the caller into `browsers`, immediately reply to the caller with the current `LobbyList`. (Optionally: if caller is already in a lobby, remove them first — but `LeaveLobby` is the proper way; first cut can just trust that browsers don't double-enter)
- [ ] 3.5 Implement the `LeaveLobby` handler:
  - Resolve the caller's current lobby via `get_lobby_session_for_client`. If none, return `LobbyError::InvalidState` (or no-op silently — pick one and document it inline)
  - If the lobby is `Playing` or `Ended`: reject (out of scope per spec)
  - If caller is **not** the host: remove their `SocketPlayer` from the lobby; broadcast lobby state to remaining players; add caller to `browsers` and send them a fresh `LobbyList`; broadcast lobby list to all browsers
  - If caller **is** the host: remove the lobby entirely; for each remaining player in the destroyed lobby, add their client_id to `browsers` and send them a `LobbyList`; broadcast lobby list to all browsers (the lobby is now absent)
- [ ] 3.6 Wire the lobby-list push into every relevant state transition: end of `CreateGame`, end of `Join` (when target is `Waiting`), end of `LeaveLobby`, end of `StartGame` (Waiting → Playing), end of `disconnected` (when the disconnected client was in a waiting lobby), and end of `PlayerAction` when the game outcome lands (Playing → Ended)
- [ ] 3.7 In `LobbyServer::disconnected`: also remove the client from `browsers` if present
- [ ] 3.8 `server/tests/lobby.rs`: add `enter_browser_returns_current_lobby_list` (browser sees the existing lobbies on entry)
- [ ] 3.9 `server/tests/lobby.rs`: add `creating_a_lobby_pushes_list_to_existing_browsers` (browser A is listening; client B creates a game; A receives an updated list including the new lobby)
- [ ] 3.10 `server/tests/lobby.rs`: add `host_disconnect_flips_host_connected_in_pushed_list`
- [ ] 3.11 `server/tests/lobby.rs`: add `non_host_leave_lobby_removes_player_and_pushes_list`
- [ ] 3.12 `server/tests/lobby.rs`: add `host_leave_lobby_destroys_lobby_and_remaining_players_receive_lobby_list`
- [ ] 3.13 `server/tests/lobby.rs`: add `playing_lobby_in_list_omits_host_info` (verifies the type-level guarantee from `LobbySummaryStatus::Playing` carrying no host fields)
- [ ] 3.14 `server/tests/lobby.rs`: add `ended_lobby_excluded_from_list`
- [ ] 3.15 `server/tests/e2e_websocket.rs`: add an end-to-end round-trip — open ws, send `EnterBrowser`, receive `LobbyList`; another ws sends `CreateGame`, first ws receives an updated `LobbyList`; first ws sends `LeaveLobby` while in browser state and gets a clean rejection (or whichever behavior 3.5 settled on)

## 4. Lobby browser UI + waiting-room host gating (`ratatui-app` crate)

- [ ] 4.1 Add a new `LobbyBrowserApp` (in `ratatui-app/src/browser.rs` or extending `input_app.rs`) that holds `display_name`, `current_list: Vec<LobbySummary>`, and selection cursor state
- [ ] 4.2 Render the browser screen per the design's UI sketch: header with welcome, "Create new game" button, "Waiting for players" section with one entry per `Waiting` lobby (showing player names, host annotation, `(host disconnected)` marker when applicable), "In progress" section with `Playing` entries shown muted with no action affordance
- [ ] 4.3 Handle keyboard / click input: navigate the list, "Enter" on a waiting entry produces a `Join` intent, "C" or click on the create affordance produces a `Create` intent, "Esc" or click on a leave affordance produces a `LeaveBrowser` intent (return the user to name entry, optional v1 polish)
- [ ] 4.4 Expose an `update(lobby_list: Vec<LobbySummary>)` method so the parent (web-client / native main) can feed in pushed updates
- [ ] 4.5 In the existing waiting-room rendering in `hanabi_app.rs`, gate the "Start Game" action on `self.my_player().is_host == true`; for non-hosts replace it with a `"Waiting for {host_name} to start..."` line (find `host_name` from the players list where `is_host`)
- [ ] 4.6 Add a host-only "End Lobby" affordance in the waiting room (sends `LeaveLobby`). Confirm-before-acting is a client-side concern — include a simple inline confirmation prompt rather than a modal in v1
- [ ] 4.7 Add a non-host "Leave" affordance in the waiting room (sends `LeaveLobby`, no confirmation needed)
- [ ] 4.8 `ratatui-app/tests/render_snapshots.rs`: snapshot of the browser screen with an empty list
- [ ] 4.9 `ratatui-app/tests/render_snapshots.rs`: snapshot of the browser screen with one Waiting and one Playing lobby
- [ ] 4.10 `ratatui-app/tests/render_snapshots.rs`: snapshot of the browser screen showing a `Waiting` lobby with `host_connected: false`
- [ ] 4.11 `ratatui-app/tests/render_snapshots.rs`: snapshot of the waiting-room screen rendered as host (shows Start) vs non-host (shows "Waiting for host…")

## 5. Client wiring (`web-client` crate)

- [ ] 5.1 Add `TuiState::Browser { player_name, server_address, browser_app: LobbyBrowserApp, websocket: Option<WebSocket> }` variant
- [ ] 5.2 After `TuiState::AppInput` completes with a non-empty name AND no `session_id` in the query string, transition to `TuiState::Browser` (instead of straight to `CreatingGame`)
- [ ] 5.3 In `TuiState::Browser`, open the websocket if not already open (re-using `setup_websocket` with a new `init_message` of `ClientToServerMessage::EnterBrowser`)
- [ ] 5.4 Handle incoming `ServerToClientMessage::LobbyList` in the Browser state by feeding it to `browser_app.update(...)`
- [ ] 5.5 When the browser surfaces a `Create` intent, send `CreateGame` and on `CreatedGame` reply perform the existing URL-redirect (so the share-the-link flow is unchanged); when it surfaces a `Join` intent, send `Join { player_name, session_id }` and transition to `HanabiApp` (keeping the websocket open across the transition)
- [ ] 5.6 When already in `TuiState::HanabiApp` waiting-room and the user triggers `LeaveLobby`, send the message; on receiving `LobbyList` from the server, transition back to `TuiState::Browser` (treat any `LobbyList` arriving while in HanabiApp as "lobby ended, you're now browsing" per the design's signal-reuse decision)
- [ ] 5.7 Preserve the existing direct-to-`HanabiApp` flow when a `session_id` is provided in the URL query (link-share path stays identical)
- [ ] 5.8 Manual smoke test (the only thing snapshot tests don't catch): open two browser tabs, name-entry in both, verify tab A sees tab B's created lobby appear without refresh; B joins; both reach the waiting room; only B's tab shows the Start button; B starts; both reach the game; tab C entering as browser sees the playing lobby in the "In progress" section

## 6. Cross-cutting cleanup

- [ ] 6.1 `cargo test --workspace` is green
- [ ] 6.2 `cargo clippy --workspace --all-targets -- -D warnings` is clean (or warnings explicitly justified in PR)
- [ ] 6.3 Update any documentation in `CLAUDE.md` / repo `README.md` if it claims "share the link is the only way to join" or similar (quick grep — likely nothing to change)
