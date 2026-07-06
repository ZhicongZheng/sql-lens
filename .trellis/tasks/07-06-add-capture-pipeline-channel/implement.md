# Add capture pipeline channel implementation plan

## Steps

1. Add `crates/sql-lens-capture` with Cargo metadata inherited from the workspace.
2. Add `sql-lens-capture` to the root workspace members.
3. Implement bounded channel config, publisher, receiver, publish outcomes, publish errors, and stats.
4. Add test helpers to construct representative `SqlEvent` values.
5. Add unit tests for enqueue/receive, drop-newest, reject-new, and closed receiver.
6. Update backend directory/spec docs so future agents know the new crate boundary.
7. Run validation:
   - `cargo fmt --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

## Constraints

- Keep capture crate protocol-neutral.
- Do not parse SQL or packets.
- Do not call storage/API/WebSocket/plugin code.
- Do not block publish on receiver progress.
- Do not add `uuid`, `time`, `chrono`, `thiserror`, `anyhow`, or `tokio-util`.

## Acceptance mapping

- Workspace crate: root `Cargo.toml` member and `crates/sql-lens-capture`.
- Capacity configurable: `CapturePipelineConfig::new(NonZeroUsize, CaptureOverloadPolicy)`.
- Explicit overload policy: `CaptureOverloadPolicy`.
- Dropped counter: `CapturePipelineStats.dropped_events`.
- Non-blocking publish: `CaptureEventPublisher::publish` uses `try_send`.
