# Implement

1. sql-lens-api: feature embedded-ui, rust-embed, mime_guess, SPA static handler.
2. HttpServerConfig: `use_embedded_ui: bool`.
3. Router: dir → embedded → none.
4. sql-lens-app: default feature, set use_embedded_ui when no disk UI.
5. build.rs fail if embed enabled and index missing.
6. scripts/release-binary.sh + README/CONFIG notes.
7. Tests for embedded handler with feature; CI npm build + cargo.
