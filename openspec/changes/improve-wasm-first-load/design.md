## Context

The current web client is a Rust crate compiled to WASM via Trunk and served as static files by an Axum server (`server/src/app.rs:36-47`), behind Fly.io's HTTP edge. The server's static handler is a one-liner: `Router::new().route("/websocket", ...).fallback_service(ServeDir::new("dist"))`. There is no compression layer, no cache-control configuration, and `web-client/index.html` ships a body that contains only an invisible `<canvas>`. The release WASM bundle is ~19 MB uncompressed.

Trunk produces `dist/index.html` containing an auto-generated `<script type="module">` that does `await init(...)` and then `dispatchEvent(new CustomEvent("TrunkApplicationStarted"))`. That event is a stable hook for "the app is now running."

Pinned versions worth knowing for the design:
- `tower-http = "0.5.0"` with features `["fs"]` only (in `server/Cargo.toml`).
- `axum = "0.8"` with `["ws"]`.
- Dockerfile build stage uses `rust:1-bookworm` (Debian, has `apt-get`); the runtime stage is `debian:bookworm-slim`.

## Goals / Non-Goals

**Goals:**
- Show something the moment the HTML parses, well before the WASM is fetched/compiled.
- Cut the bytes on the wire for the WASM/JS shim by a large factor (target: 19 MB → ~5 MB or less).
- Make repeat visits truly free of asset downloads via correct cache headers.
- Keep the loader's behavioral surface (dismiss + timeout) decoupled from its visual treatment, so a fancier branded loader can be swapped in later without rewiring the dismiss logic.

**Non-Goals:**
- Real download-progress UI. Would require sidestepping Trunk's auto-generated boot script.
- Cargo release-profile size optimizations (`lto`, `panic = "abort"`, `strip`, `wasm-opt -Oz`). Defer until we measure how much compression alone bought us.
- Fly pre-warming (`min_machines_running = 1`).
- A fancy or animated visual treatment. The loader is intentionally minimal v1.
- Errors that distinguish *why* boot failed (network vs. WASM compile vs. JS exception). v1 only knows "didn't finish in time."

## Decisions

### D1. Loader lives in `web-client/index.html`; Trunk's boot is left untouched

Trunk auto-injects a `<script type="module">` whose `await init(...)` dispatches `TrunkApplicationStarted` on success. We register an event listener on `window` in a plain `<script>` block in `index.html` (which runs synchronously during parse, before the module script defers and runs). The listener hides the loader element.

A `setTimeout(..., 30_000)` started at parse time swaps the loader's inner content to an error state if the event hasn't fired. The event listener stays attached after the timeout fires, so a late `TrunkApplicationStarted` still dismisses the loader cleanly (the "late boot" scenario in the loading-ux spec).

**Why this over overriding Trunk's boot script:** Trunk's generated script handles WASM instantiation, the JS bindings, and event dispatch. Replacing it means tracking the upstream contract by hand. The event-based approach is one-way coupling — Trunk publishes the event, we subscribe. If Trunk changes its scheme later, we only need to update our listener.

**Trade-off:** We can't surface a *real* error message when `init()` rejects, because we never see the exception. We only know "didn't finish in 30 s." Acceptable for v1; the user-facing copy ("Took longer than expected — try refreshing") covers both real timeouts and silent failures.

### D2. Loader DOM is one element, identified by `id="loader"`

The loader is a single `<div id="loader">` containing a centered text label and a pure-CSS spinner (`@keyframes spin` rotating a `border-radius: 50%; border-top: 3px solid …; border: 3px solid transparent` element). No images, no SVG, no web fonts. The dismiss script targets `#loader` only; the inner markup is free to change.

This is the "leave the door open for a fancier loader" seam: replacing the contents of `#loader` (or replacing the element entirely while keeping the `id`) does not require touching the listener or timeout code.

### D3. Pre-compress at build time, serve precompressed variants

In the Dockerfile build stage, after `trunk build --release`:

```dockerfile
RUN apt-get update && apt-get install -y --no-install-recommends brotli && rm -rf /var/lib/apt/lists/*
RUN find dist -type f \( -name "*.wasm" -o -name "*.js" \) -exec brotli --best --keep {} +
RUN find dist -type f \( -name "*.wasm" -o -name "*.js" \) -exec gzip -9 --keep {} +
```

In `server/src/app.rs`:

```rust
.fallback_service(
    ServeDir::new("dist")
        .precompressed_br()
        .precompressed_gzip()
)
```

`tower-http`'s `ServeDir` consults `Accept-Encoding`, prefers brotli, falls back to gzip, then to identity. When a precompressed variant is present on disk it serves it and sets `Content-Encoding`. When absent it falls back transparently — so forgetting the compression step is a safe failure (slower, not broken).

**Why precompressed over runtime `CompressionLayer`:** brotli quality 11 on a 19 MB blob is expensive (~seconds of CPU). Doing it once at image build time on the build machine is strictly better than doing it on every cache miss on Fly's shared-cpu-1x.

**Why not just `.precompressed_br()` and skip gzip:** Browser support for brotli over HTTPS is ~98%, but `curl` defaults to no `Accept-Encoding` and some corporate proxies strip `br`. Keeping gzip as a fallback is cheap insurance — the cost is one extra `.gz` file per asset in the image.

### D4. Cache headers via a single middleware on the fallback service

Trunk emits filenames like `web-client-<hex>_bg.wasm` and `web-client-<hex>.js`. Fonts under `dist/assets/` are not hashed.

We add a small `axum::middleware::from_fn` after `ServeDir` that inspects the response's URI path:
- path ends in `.wasm` or `.js` → set `Cache-Control: public, max-age=31536000, immutable`
- path is `/` or `/index.html` → set `Cache-Control: no-cache`
- everything else → no override (let ServeDir defaults stand)

This is a few lines, lives entirely in `server/src/app.rs`, and avoids splitting the fallback into multiple routes.

**Why one middleware over per-route `SetResponseHeaderLayer`:** `ServeDir` as `fallback_service` handles all paths, including dynamic ones we can't enumerate. Splitting `/` from `/web-client-*.wasm` into separate routes would require globbing or a regex route matcher. A `from_fn` middleware is simpler.

**Why match on extension rather than the hex hash pattern:** in this codebase, the only `.wasm` and `.js` files in `dist/` are the hashed ones Trunk emits. Matching `.wasm`/`.js` is equivalent in practice and is one less regex to maintain.

**Required tower-http feature change:** `from_fn` is from `axum`, not `tower-http`, so no new tower-http features are required. The `fs` feature is sufficient for `ServeDir` plus its precompression methods. If we later want `SetResponseHeaderLayer` instead, that needs the `set-header` feature — flag as an open question if measurement shows the middleware is too coarse.

### D5. Dockerfile and `dist/` layout

The build stage already does `cd web-client && trunk build --release`, which writes to `../dist` (via `Trunk.toml`'s `dist = "../dist"`). The compression step runs from `/app` and finds `.wasm` / `.js` under `dist/`. The runtime stage's existing `COPY --from=build /app/dist /app/dist` then carries the precompressed variants along automatically — no second `COPY` needed.

## Risks / Trade-offs

- **[Risk] Stale `dist/` checked in to the repo with old hashes** → Mitigation: `dist/` is a build output, generated by Trunk in the Docker build. Tracked-in copies are for the existing simple-static deployment; the change touches only `web-client/index.html` (source) and the Dockerfile (which regenerates `dist/`).
- **[Risk] `setTimeout` fires before WASM finishes on a very slow connection** → Mitigation: 30 s is generous for a 19 MB → ~5 MB blob on Slow 3G (~500 Kbps ≈ 80 s — so this could still misfire). The error copy explicitly says "try refreshing" rather than "an error occurred," which is honest about the ambiguity. Open question: do we want to tune the timeout up to 60–90 s, or trust 30 s? Listed below.
- **[Risk] Brotli compression at Docker build adds ~5–10 s of build time** → Mitigation: acceptable; this is build-time cost only, paid by maintainers not users.
- **[Risk] `no-cache` on `index.html` means every visit is a 304 (revalidation) round-trip** → Mitigation: that's the point. The HTML is tiny (~3 KB); the cost of one revalidation per session is dwarfed by avoiding a stale WASM hash.
- **[Risk] Browser caches the HTML across deploys despite `no-cache`** → Mitigation: `no-cache` requires revalidation; the server's `ETag` (set by `ServeDir`) will differ across builds, so the browser fetches the new HTML, which references the new hashed asset. If a CDN ever sits in front, we'd need to verify it honors `no-cache` end-to-end — but currently there's no CDN.
- **[Risk] A subresource (font) lacks cache headers and is re-fetched every visit** → Mitigation: out of scope for v1. The fonts are small relative to the WASM and not on the critical path; the default `ServeDir` behavior is acceptable.

## Migration Plan

This is a single PR, single deploy. No data migration, no protocol change, no feature flag.

**Deploy:**
1. Land the change on `main`; Fly auto-builds the new Docker image and rolls.
2. First visit after deploy: browsers fetch the new `index.html` (was: `no-cache`-revalidated; now: revalidated and content differs because of new hashed asset names from the rebuilt WASM). They re-download the new hashed `.wasm.br` / `.js.br`. From then on, those URLs return `Cache-Control: immutable` and won't be re-fetched until a future deploy changes the hash.

**Rollback:** revert the commit and redeploy. Browsers that cached the *new* `.wasm.br` will continue to use it (it's still valid for whatever hash it was named for); they revalidate `index.html` on the next visit and pick up the old hashed-asset references.

## Open Questions

1. **Timeout duration.** 30 s is the proposed default. On Slow 3G a 5 MB brotli'd bundle takes ~80 s. Do we set the timeout to 60 s or 90 s to avoid false positives, or stay at 30 s and accept that very slow connections will see the error message but the loader will still eventually be dismissed when boot completes? **Recommendation: 30 s.** The error copy is friendly and the late-boot scenario is handled.
2. **Should font files (`.ttf` under `dist/assets/`) get a long cache header?** They're not hashed, so far-future caching would risk staleness across font updates. Punting to a future change.
3. **Should we also pre-compress the HTML and font files?** HTML is ~3 KB; not worth a compression round. Fonts are already compressed (TTF is binary; brotli might shave a little). Punting unless measurement says otherwise.
4. **Replace Trunk's boot script later for real error reporting?** Out of scope for this change but a natural follow-up if WASM init failures become a debugging pain.
