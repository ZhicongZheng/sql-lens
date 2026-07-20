# Design: Default Static UI Delivery

## Approach

1. Keep explicit `web.static_dir` as the authoritative override.
2. When unset, discover a built SPA under well-known relative paths if `index.html` exists:
   - `crates/sql-lens-app/web/dist`
   - `web/dist`
3. Missing discovery → API-only (no error). Explicit bad path → existing startup error.
4. Document `npm run build` and config in README/CONFIG; add `scripts/build-web.sh` and `sql-lens.example.toml`.

## Wiring

`start_runtime_from_config` already uses `HttpServerConfig::from(&config.web)`. Apply discovery after `From` when `static_dir` is `None`.
