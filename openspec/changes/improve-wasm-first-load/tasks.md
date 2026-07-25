## 1. Baseline measurement

- [ ] 1.1 Capture a "before" Chrome DevTools network trace of a cold page load against the production build (record: total transfer size of `*.wasm`, time-to-first-paint of any user-visible content, time until the canvas renders the first frame). Save numbers in the PR description for after/before comparison.
- [ ] 1.2 Confirm current `dist/` filenames are content-hashed by Trunk (sanity-check: `web-client-<hex>.js` and `web-client-<hex>_bg.wasm` exist after `just build-release`).

## 2. Loading screen in `web-client/index.html`

- [ ] 2.1 Add `<div id="loader">` markup inside `<body>` (sibling to the existing `<canvas>`), containing a centered text label ("Loading Hanabi…") and a `<div class="spinner">` element. No images, no `<img>`, no external fonts.
- [ ] 2.2 Add CSS to the existing `<style>` block: position `#loader` centered (reuse the existing `.centered` positioning pattern), define `.spinner` as a pure-CSS rotating ring (`@keyframes spin`, `border-radius: 50%`, transparent border with one colored side, `animation: spin 1s linear infinite`), and define a `#loader.hidden { display: none }` rule.
- [ ] 2.3 Add a `<script>` (non-module, in the `<head>` so it runs during initial parse) that:
  - Registers `window.addEventListener("TrunkApplicationStarted", ...)` to add the `hidden` class to `#loader`.
  - Starts a `setTimeout(..., 30_000)` that swaps `#loader`'s inner text to an error message ("Took longer than expected — try refreshing.") without removing the `#loader` element. The error state must NOT add the `hidden` class.
  - When `TrunkApplicationStarted` fires, the listener also clears the pending timeout (race: timeout already fired → listener still runs and hides loader, satisfying the "late boot" scenario).
- [ ] 2.4 Manually verify in a browser: normal load shows the loader briefly, then the canvas appears and the loader is gone. Verify with `console.log` or DevTools Elements panel that `#loader.hidden` is set after boot.
- [ ] 2.5 Manually verify the timeout error state by temporarily lowering the timeout to ~2 s (or by blocking the `.wasm` request via DevTools Network → Block request URL) and confirming the error copy appears and that a real boot afterward still dismisses the loader.

## 3. Cache headers on the Axum router

- [ ] 3.1 In `server/src/app.rs`, write a small async middleware function (`axum::middleware::from_fn`) that inspects the request URI path on response and sets `Cache-Control`:
  - path ends with `.wasm` or `.js` → `Cache-Control: public, max-age=31536000, immutable`
  - path is `/` or `/index.html` → `Cache-Control: no-cache`
  - otherwise: do not set `Cache-Control` (let `ServeDir` defaults stand)
- [ ] 3.2 Chain the middleware into `build_router` after the `fallback_service` so it runs on the static-file responses but not on `/websocket`.
- [ ] 3.3 Add a unit/integration test (extend `server/tests/e2e_websocket.rs` or a new `server/tests/static_assets.rs`) that boots the router with `MemDatabase`, requests `/index.html` and `/web-client-<some-hash>.js` (any file present in `dist/`), and asserts the expected `Cache-Control` header.

## 4. Pre-compressed asset variants via `ServeDir`

- [ ] 4.1 In `server/src/app.rs`, change `ServeDir::new("dist")` to `ServeDir::new("dist").precompressed_br().precompressed_gzip()`. Confirm the existing `tower-http` feature `fs` is sufficient (these methods are gated only on `fs`).
- [ ] 4.2 In the `Dockerfile` build stage, after `RUN cd web-client && trunk build --release`, install brotli and compress: 
  - `RUN apt-get update && apt-get install -y --no-install-recommends brotli && rm -rf /var/lib/apt/lists/*`
  - `RUN find dist -type f \( -name "*.wasm" -o -name "*.js" \) -exec brotli --best --keep {} +`
  - `RUN find dist -type f \( -name "*.wasm" -o -name "*.js" \) -exec gzip -9 --keep {} +`
- [ ] 4.3 Verify locally (`docker build -t hanabi:test . && docker run --rm -it hanabi:test ls /app/dist`) that `*.wasm.br`, `*.wasm.gz`, `*.js.br`, `*.js.gz` files exist alongside the originals in the runtime image.
- [ ] 4.4 Extend the test from 3.3 (or add a sibling test) that requests a `.wasm` URL with `Accept-Encoding: br, gzip` and asserts `Content-Encoding: br` on the response; repeat with `Accept-Encoding: gzip` only (expect `gzip`); repeat with no `Accept-Encoding` (expect no `Content-Encoding`). Note: this test requires the precompressed variants to exist in `dist/` — either generate them in the test setup or document that the test requires running `just build-release` + the compression step first.

## 5. Verification

- [ ] 5.1 Run `just build && just test` and `cargo clippy --workspace --all-targets` — no new failures or warnings introduced by this change.
- [ ] 5.2 Capture an "after" Chrome DevTools network trace from the production-mode build (run the same scenario as 1.1) — record the transfer size of `*.wasm` (target: ~5 MB or less) and the time-to-first-paint (loader should appear within the first second). Add the before/after numbers to the PR description.
- [ ] 5.3 In an incognito window, reload the page twice — confirm via the Network panel that the second reload serves the WASM/JS from disk cache (status `200 (from disk cache)` or `(memory cache)`) and that `index.html` is served with a 304 revalidation.
- [ ] 5.4 Update `CLAUDE.md`'s "Conventions" section if the cache-header middleware or compression step introduces a pattern future contributors should know about. (Skip if the change is self-evident from the code.)
