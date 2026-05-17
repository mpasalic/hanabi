# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS build
WORKDIR /app

# wasm target + trunk (pinned prebuilt binary for fast, deterministic builds)
RUN rustup target add wasm32-unknown-unknown
ARG TRUNK_VERSION=v0.21.7
RUN curl -sSfL "https://github.com/trunk-rs/trunk/releases/download/${TRUNK_VERSION}/trunk-x86_64-unknown-linux-gnu.tar.gz" \
    | tar -xz -C /usr/local/bin

COPY . .

# Build web client (wasm) then the server binary
RUN cd web-client && trunk build --release
RUN cargo build --release -p server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/target/release/server /app/server
COPY --from=build /app/dist /app/dist

ENV PORT=8080
EXPOSE 8080
CMD ["/app/server"]
