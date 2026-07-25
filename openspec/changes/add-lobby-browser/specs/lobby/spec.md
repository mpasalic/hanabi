## ADDED Requirements

### Requirement: Pre-Game Browsing State

The system SHALL support a connected client state in which the client has not yet committed to any game session. In this state the client receives lobby list updates and may, without reconnecting, transition into creating a new game, joining an existing waiting game, or returning to a previous game it was already a player of.

#### Scenario: Client enters browsing state after providing identity

- **WHEN** a client opens a connection and signals that it is browsing, providing a display name
- **THEN** the server treats the client as a browser, does not associate it with any game session, and sends the current lobby list

#### Scenario: Browser remains connected without committing to a session

- **WHEN** a client is in the browsing state and does not send any further messages
- **THEN** the client remains connected, continues to receive push updates as lobbies change, and is not counted as a player in any lobby

### Requirement: Lobby List Contents

The lobby list pushed to browsing clients SHALL describe all currently-known lobbies in a form sufficient for the client to render them and decide which are actionable. Each entry SHALL include the session id, the lobby status, and the ordered list of player display names. Waiting-lobby entries SHALL additionally identify the host (by display name) and indicate whether the host is currently connected. Lobbies whose games have ended SHALL NOT appear in the list.

#### Scenario: Waiting lobby is listed as joinable

- **WHEN** a lobby exists in the waiting state and the lobby list is sent
- **THEN** the entry includes the session id, the player display names, the host's display name, the host's current connection state, and indicates the lobby is in the waiting state

#### Scenario: In-progress lobby is listed as informational only

- **WHEN** a lobby exists in the playing state and the lobby list is sent
- **THEN** the entry includes the session id and the player display names, indicates the lobby is in the playing state, and is distinguishable by the client as non-joinable

#### Scenario: Ended lobby is excluded from the list

- **WHEN** a lobby's game has ended and the lobby list is sent
- **THEN** that lobby does not appear in the list

### Requirement: Push-Based Lobby List Updates

The server SHALL push an updated lobby list to every connected browsing client whenever a change occurs that affects what a browser would see — specifically: a new lobby is created, a player joins or leaves a waiting lobby, the host's connection state changes, a game transitions from waiting to playing, or a lobby is removed. The server SHALL NOT require clients to poll for updates.

#### Scenario: Browser sees a newly created lobby

- **WHEN** a browsing client is connected and another client creates a new game
- **THEN** the browsing client receives an updated lobby list that includes the new waiting lobby

#### Scenario: Browser sees an updated roster when a player joins

- **WHEN** a browsing client is connected and a player joins an existing waiting lobby
- **THEN** the browsing client receives an updated lobby list reflecting the new player

#### Scenario: Browser sees a lobby move from waiting to playing

- **WHEN** a browsing client is connected and the host of a waiting lobby starts the game
- **THEN** the browsing client receives an updated lobby list in which that lobby's status is playing

#### Scenario: Browser sees host connection state change

- **WHEN** a browsing client is connected and a waiting lobby's host disconnects or later reconnects
- **THEN** the browsing client receives an updated lobby list reflecting the host's new connection state

### Requirement: Creating a New Lobby

A connected client SHALL be able to create a new game session. The server SHALL generate a unique session id for the new lobby, place the requesting client into it as its first player, designate that client as the host, and return the session id to the creator so the existing share-the-link flow continues to work.

#### Scenario: Client creates a new game

- **WHEN** a client provides a display name and requests to create a game
- **THEN** the server creates a waiting lobby with a freshly generated unique session id, the requesting client is its sole player and host, and the session id is returned to the creator

### Requirement: Host Designation Is Permanent For The Lobby's Lifetime

A waiting lobby SHALL have exactly one player designated as host: the player who created the lobby. The host designation SHALL NOT transfer to any other player under any circumstances. If the host's connection is lost, the host designation remains bound to the host's player slot; the lobby is temporarily without a connected host until the host reconnects.

#### Scenario: Creator is the host

- **WHEN** a new lobby has just been created
- **THEN** the creating client is designated as host

#### Scenario: Host designation does not move when host disconnects

- **WHEN** the host of a waiting lobby disconnects and at least one other player remains
- **THEN** no other player is promoted to host and the lobby remains in the waiting state with the original player still designated as the host

#### Scenario: Host designation is visible in lobby state

- **WHEN** any client receives the state of a waiting lobby
- **THEN** exactly one player in the player list is marked as the host

### Requirement: Host-Only Game Start

Only the player currently designated as host of a waiting lobby SHALL be able to start the game, and only while that host's connection is in a connected state. The server SHALL reject start requests from non-host players and SHALL reject start requests issued while the host is disconnected. Rejected start requests SHALL NOT change the lobby state.

#### Scenario: Connected host starts the game

- **WHEN** the host of a waiting lobby is connected, requests to start the game, and the lobby satisfies the conditions for starting
- **THEN** the lobby transitions to the playing state and the game begins for all players in the lobby

#### Scenario: Non-host attempts to start the game

- **WHEN** a player in a waiting lobby who is not the host requests to start the game
- **THEN** the server rejects the request with an error and the lobby remains in the waiting state

#### Scenario: Start is impossible while host is disconnected

- **WHEN** the host of a waiting lobby is disconnected
- **THEN** no client is able to start the game, and any start request issued in this state is rejected

### Requirement: Join Resolves By Display Name

When a client requests to join a lobby by providing a session id and a display name, the server SHALL first attempt to match the display name against the lobby's existing players. If a matching player exists, the join is treated as a reconnection: the matched player's connection slot is reassigned to the requesting client, regardless of the lobby's current status. If no matching player exists, the join is accepted as a new player only if the lobby is in the waiting state; otherwise the join is rejected. The server SHALL NOT attempt to distinguish accidental name collisions from intentional reconnections; clients are expected to choose distinct display names within a lobby.

#### Scenario: Reconnection by name match in a waiting lobby

- **WHEN** a client requests to join a waiting lobby with a display name that matches an existing player in that lobby
- **THEN** the matched player slot's connection is reassigned to the requesting client and the updated lobby state is broadcast to all clients in the lobby

#### Scenario: Reconnection by name match in a playing lobby

- **WHEN** a client requests to join a playing lobby with a display name that matches an existing player in that lobby
- **THEN** the matched player slot's connection is reassigned to the requesting client and the requesting client receives the current game state

#### Scenario: New player joins a waiting lobby

- **WHEN** a client requests to join a waiting lobby with a display name that does not match any existing player in that lobby
- **THEN** the client is added to the lobby's player list as a new non-host player and the updated lobby state is broadcast to all clients in the lobby

#### Scenario: New player rejected from a playing or ended lobby

- **WHEN** a client requests to join a playing or ended lobby with a display name that does not match any existing player in that lobby
- **THEN** the server rejects the request with an error and the lobby is not modified

### Requirement: Non-Host Leaves A Waiting Lobby

A non-host client in a waiting lobby SHALL be able to leave the lobby and return to the browsing state without closing its connection. Leaving removes the client's player slot from the lobby (in contrast to disconnecting, which preserves the slot for reconnection by name).

#### Scenario: Non-host player leaves a waiting lobby

- **WHEN** a non-host client in a waiting lobby signals that it is leaving
- **THEN** the client's player slot is removed from that lobby, the remaining clients in the lobby receive updated lobby state, and the leaving client transitions into the browsing state with the current lobby list

### Requirement: Host Leaves A Waiting Lobby

If the host of a waiting lobby explicitly leaves the lobby (as opposed to disconnecting), the lobby SHALL be removed entirely. All remaining players in the lobby SHALL be returned to the browsing state. This prevents a permanently unstartable orphan lobby, since the host designation does not transfer.

#### Scenario: Host leaves with other players present

- **WHEN** the host of a waiting lobby signals that it is leaving and other players are present
- **THEN** the lobby is removed, all remaining players in the lobby are transitioned into the browsing state with the current lobby list, and all browsing clients receive an updated lobby list with that lobby absent

#### Scenario: Host leaves with no other players present

- **WHEN** the host of a waiting lobby signals that it is leaving and no other players are present
- **THEN** the lobby is removed and all browsing clients receive an updated lobby list with that lobby absent
