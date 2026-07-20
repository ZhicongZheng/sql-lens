# Implement: Default Static UI Delivery

1. Add `discover_default_static_dir()` in app runtime; use in `start_runtime_from_config`.
2. Test: temp dist with index.html is auto-discovered; explicit invalid path still fails.
3. Update README quickstart + CONFIG `static_dir` docs.
4. Add `scripts/build-web.sh` and `sql-lens.example.toml`.
5. Validate fmt/tests/clippy for app (+ docs only files).
