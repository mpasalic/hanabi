# HanabiApp

TODO

- [ ] Lobby UI
- [ ] Fix reconnection (too aggressive)
- [ ] Improve Create/Join UI
- [ ] Game log scrolling

### Test run #1

- [ ] BLOCKER: WE NEED CONFIRM ACTION UI
- [ ] BLOCKER: Mouse coordinate glitchyness on Davids
- [ ] David: Game log needs more color and stuff
- [ ] David: Player/card indexes would be nice
- [ ] Back button doesn't work?
- [ ] David: Private notes
- [ ] Scrollbar doesn't work
- [ ] Jeffs client disconnected
- [x] End game state broken
- [x] IT FUCKING WORKS

## Workspaces

- `web-client/` this is the EGui wrapper client to serve a app through the web using WebAssembly
  - `ratatui-app/` is a lib package that implements the actual Ratatui UI
- `shuttle-server/` this the server that runs the lobby + game engine. It uses a framework created by shuttle.rs to easily allow deployments (which have a free tier!)
- `shared/` this is where all the shared models + API live

## Dependencies

- `cargo install just`
- probably way more, don't remember anymore. Please add more if you notice them.

## Local enviornment

1. Run the server locally
   `just run-shuttle-dev`

2. Start a development web client
   `just run-web-dev`

- Note: This will auto-recompile on changes within web-client/ but unfortauntely it does not do so for the dependency workspaces. You'll need to restart later
- TODO Ask Simon how to fix this

3. Load the web client
   `http://127.0.0.1:8080/`

Note: Running the shuttle server will also serve the web client through `http://127.0.0.1:8000`, but this doesn't auto-compile when changes are detected, and you must manually run `just build-web-release`

## Deployment

`just deploy-shuttle-release`

This will deploy the local repo to https://hanabi.shuttle.rs
