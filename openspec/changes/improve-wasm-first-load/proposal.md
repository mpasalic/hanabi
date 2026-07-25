## Why

On a fresh visit (and on every visit after a deploy), the web client shows a blank grey screen for several seconds while the 19 MB WASM bundle is downloaded, compiled, and egui boots. There is no visual feedback that anything is happening — the page's only body content is an invisible `<canvas>`, so users have no way to tell the app from a broken page. Compounding this, the server currently sends the WASM bundle uncompressed and without cache-control headers, so the wait is longer than it needs to be and is repeated unnecessarily on every visit.

## What Changes

- Add a static loading indicator to `web-client/index.html` that is visible the moment the HTML parses, before the WASM is fetched or compiled. The indicator is plain HTML + CSS (no images, no JS frameworks) and is dismissed when Trunk's `TrunkApplicationStarted` event fires.
- Add a client-side timeout (~30 s) that swaps the indicator to a "took longer than expected — try refreshing" error state, so a failed `init()` (network drop, WASM compile error, etc.) does not strand the user on a spinner forever.
- Pre-compress `dist/*.wasm` and `dist/*.js` with brotli (max quality) and gzip at build time, in the Dockerfile.
- Switch the server's static-file handler from `ServeDir::new("dist")` to `ServeDir::new("dist").precompressed_br().precompressed_gzip()` so the pre-compressed variants are served when the client's `Accept-Encoding` header allows it.
- Send `Cache-Control: public, max-age=31536000, immutable` for the content-hashed asset paths (`*_bg.wasm`, the hashed JS shim, font files), and `Cache-Control: no-cache` for `index.html` so new builds are picked up on the next visit.

The loader markup is intentionally structured so a fancier branded loader can be dropped in later without revisiting the dismiss/timeout plumbing. Per-byte download progress, Cargo release-profile size tweaks, and Fly pre-warming are explicitly out of scope.

## Capabilities

### New Capabilities

- `web-client-loading-ux`: What the user sees in the browser between page-load and the first egui frame. Covers the loader's visibility lifecycle (shown immediately, dismissed on `TrunkApplicationStarted`, swapped to an error state on timeout) and the structural seams that let a future visual treatment replace the simple loader without rewiring its behavior.
- `static-asset-delivery`: How the server delivers the WASM bundle, JS shim, fonts, and HTML entry point to the browser. Covers content-encoding negotiation (precompressed brotli/gzip variants), cache-control semantics for content-hashed assets vs. the HTML entry point, and the build-time step that produces the compressed variants.

### Modified Capabilities

None. There are no existing specs in `openspec/specs/`.

## Impact

**Code & build:**

- `web-client/index.html` — add loader markup, spinner CSS, dismiss + timeout script.
- `Dockerfile` — install `brotli` in the build stage; pre-compress `dist/*.wasm` and `dist/*.js` after `trunk build --release`.
- `server/src/app.rs` — chain `.precompressed_br().precompressed_gzip()` onto the existing `ServeDir`; add cache-header middleware that distinguishes hashed assets from `index.html`.
- `server/Cargo.toml` — enable the `tower-http` feature(s) required for `ServeDir` precompression and response-header setting (likely `fs` + `set-header`).

**Runtime:**

- No new server CPU cost per request — compression happens once at build time.
- Modest increase in Docker image size (the build stage gains `brotli`; the runtime stage gains `.br` / `.gz` files alongside the originals).
- No DB, wire protocol, or `shared` crate changes. The websocket handshake and message protocol are untouched.

**Backward compatibility:**

- Clients that don't send `Accept-Encoding: br` or `gzip` continue to receive the uncompressed bundle.
- No breaking changes to the user-facing URL space or the websocket protocol.
