# Hanabi
[Hanabi](https://en.wikipedia.org/wiki/Hanabi_(card_game)) is a cooperative card game where players are only aware of other players cards and can only communicate with each other through a system of hints that reveal limited information. The objective is to play a series of cards in the correct sequential order in order to complete all sets, resulting in fireworks (Hanabi is a japenese word for "fireworks"). Think multiplayer solataire!

This version of Hanabi is a multiplayer web app, built in Rust using the [ratatui](https://ratatui.rs/) terminal crate and [egui] UI framework crate (https://github.com/gold-silver-copper/egui_ratatui) to render WASM for a retro-feeling Hanabi game.

## Demo

<img width="1485" height="764" alt="hanabi-screenshot" src="https://github.com/user-attachments/assets/6ac1fb4e-ebe9-4b3c-8daa-ecbe367e4288" />

**Warning**
* The current version of this app is fairly limited in terms of "lobby" capabilities. In order to start a game with other players, you must share the generated URL after "creating" a game. When the link is opened, they will join your game.
* There is minimal security or cheating protection built-in at the current moment. Players can rejoin the game and take over the spot as long as their chosen username is the same. Thus, each player should pick a unique name!

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
