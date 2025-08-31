# Hanabi
[Hanabi](https://en.wikipedia.org/wiki/Hanabi_(card_game)) is a cooperative card game where players are only aware of other players cards and can only communicate with each other through a system of hints that reveal limited information. The objective is to play a series of cards in the correct sequential order in order to complete all sets, resulting in fireworks (Hanabi is a japenese word for "fireworks"). Think multiplayer solataire!

This version of Hanabi is a multiplayer web app, built in Rust using the [ratatui](https://ratatui.rs/) and [egui](https://github.com/gold-silver-copper/egui_ratatui) for a terminal-like experience in the browser, connected to a axum websocket server hosted on [shuttle](shuttle.rs).

## How to play

Web Client Demo: https://mpasalic.github.io/hanabi
* **Note:** Lobby capabilities are very basic. To start a game with friends, [open Hanabi](https://mpasalic.github.io/hanabi) and press [Enter] to create a game. This will redirect to a new session. The invite others, share the URL in your browser. Once everyone has joined, press [S] to start the game. Enjoy!

<img width="1485" height="764" alt="hanabi-screenshot" src="https://github.com/user-attachments/assets/6ac1fb4e-ebe9-4b3c-8daa-ecbe367e4288" />

**Imporant notes**
* There is no authentication or any security that prevents impersonation. If anyone gets disconnected, simply open the session URL again and choose the same name (it is persisted to make it easier). This also means everyone needs a unique name, or else the game will consider them the same person!

# Development
## Workspaces

- `web-client/` this is the EGui wrapper client to serve a app through the web using WebAssembly
  - `ratatui-app/` is a lib package that implements the actual Ratatui UI
- `shuttle-server/` this the server that runs the lobby + game engine. It uses a framework created by shuttle.rs to easily allow deployments (which have a free tier!)
- `shared/` this is where all the shared models + API live

## Dependencies

- `cargo install just`
- `cargo install trunk`
- `cargo install shuttle`
- Install "Docker Helper"
  - 
- probably way more, don't remember anymore. Please add more if you notice them.

## Local enviornment

1. Run the server locally
   `just run-shuttle-dev`

2. Start a development web client
   `just run-web-dev`

- Note: This will auto-recompile on changes within web-client/ but unfortauntely it does not do so for the dependency workspaces. You'll need to restart later

3. Load the web client
   `http://127.0.0.1:8080/`

Note: Running the shuttle server will also serve the web client through `http://127.0.0.1:8000`, but this doesn't auto-compile when changes are detected, and you must manually run `just build-web-release`

## Deployment

`just deploy-shuttle-release`

This will deploy the local repo to https://hanabi.shuttle.rs
