# just task runner configuration for shuttle-template-yew

run-dev:
  cd web-client && trunk serve --open

build-release:
  cd web-client && trunk clean && trunk build --release

shuttle-run:
  cargo shuttle run

shuttle-deploy:
  cargo shuttle project restart && cargo shuttle deploy