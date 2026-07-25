## ADDED Requirements

### Requirement: Precompressed asset variants produced at build time

The production build SHALL produce brotli- and gzip-compressed variants of the WASM bundle and JavaScript shim alongside the original files in `dist/`. The compressed variants SHALL be present in the runtime image so that the server can serve them without performing compression at request time.

#### Scenario: build artifacts include compressed variants
- **WHEN** the production build (`trunk build --release` followed by the compression step) completes
- **THEN** for each `dist/*.wasm` and `dist/*.js` file, a corresponding `.br` and `.gz` file exists alongside it

#### Scenario: compressed variants ship in the runtime image
- **WHEN** the runtime container starts
- **THEN** the `.br` and `.gz` variants for the WASM bundle and JS shim are present on disk and accessible to the static-file handler

### Requirement: Content-encoding negotiation for compressed assets

When the static-file handler serves an asset that has a precompressed variant available, it SHALL respect the client's `Accept-Encoding` header. Brotli SHALL be preferred when offered; gzip SHALL be used when brotli is not offered but gzip is; the uncompressed variant SHALL be returned otherwise. The response SHALL carry the appropriate `Content-Encoding` header matching what was sent.

#### Scenario: client accepts brotli
- **WHEN** a client requests a WASM or JS asset with `Accept-Encoding: br, gzip`
- **THEN** the server responds with the brotli-compressed variant and `Content-Encoding: br`

#### Scenario: client accepts only gzip
- **WHEN** a client requests a WASM or JS asset with `Accept-Encoding: gzip`
- **THEN** the server responds with the gzip-compressed variant and `Content-Encoding: gzip`

#### Scenario: client offers no compression
- **WHEN** a client requests a WASM or JS asset with no `Accept-Encoding` header, or with `identity` only
- **THEN** the server responds with the uncompressed variant and no `Content-Encoding` header (or `Content-Encoding: identity`)

### Requirement: Immutable cache headers for content-hashed assets

The static-file handler SHALL send `Cache-Control: public, max-age=31536000, immutable` on responses for content-hashed asset paths — those whose filename contains the build-time content hash (the WASM bundle and the JavaScript shim emitted by Trunk).

#### Scenario: hashed WASM bundle
- **WHEN** a client requests the content-hashed WASM bundle (e.g., `web-client-<hash>_bg.wasm`)
- **THEN** the response includes `Cache-Control: public, max-age=31536000, immutable`

#### Scenario: hashed JS shim
- **WHEN** a client requests the content-hashed JS shim (e.g., `web-client-<hash>.js`)
- **THEN** the response includes `Cache-Control: public, max-age=31536000, immutable`

### Requirement: Revalidating cache headers for the HTML entry point

The static-file handler SHALL send a cache policy on `index.html` that requires the browser to revalidate with the server on every visit, so that newly deployed builds (with new hashed asset filenames) are picked up promptly.

#### Scenario: HTML entry point
- **WHEN** a client requests `/` or `/index.html`
- **THEN** the response includes `Cache-Control: no-cache` (or an equivalent policy that forces revalidation, such as `no-store` or `max-age=0, must-revalidate`)
