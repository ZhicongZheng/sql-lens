# Design

## Embed location

`sql-lens-api` gains optional feature `embedded-ui` using `rust-embed` with folder
`../sql-lens-app/web/dist` (relative to api crate manifest). `sql-lens-app` enables
it by default for the product binary.

## Serve priority (app)

1. Configured `web.static_dir`
2. Disk discovery (`crates/sql-lens-app/web/dist`, `web/dist`)
3. Embedded assets (feature on)
4. API-only fallback 404

## Compile requirement

`web/dist/index.html` must exist when `embedded-ui` is enabled. `build.rs` fails with
a clear message, or CI/release script always runs `./scripts/build-web.sh` first.

## Release

`scripts/release-binary.sh`: build-web + cargo build -p sql-lens-app --release
→ `target/release/sql-lens`
