# just task runner configuration for shuttle-template-yew

run-web-dev:
  cd web-client && trunk serve --open --proxy-backend=ws://127.0.0.1:8000/websocket --proxy-ws

build-web-release:
  cd web-client && trunk clean && trunk build --release

run-shuttle-dev:
  cargo shuttle run

deploy-shuttle-release:
  cargo shuttle project restart && cargo shuttle deploy