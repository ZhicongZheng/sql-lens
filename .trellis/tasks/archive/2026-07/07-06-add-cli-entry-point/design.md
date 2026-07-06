# CLI Entry Point Design

## Objective

Turn `sql-lens-app` into the first usable binary entry point.

The binary should parse CLI arguments, load and validate config, and return clear process exit codes. It should not start SQL Lens services yet.

## Crate Boundary

Modify:

- `crates/sql-lens-app/Cargo.toml`
- `crates/sql-lens-app/src/main.rs`

Expected lockfile update:

- `Cargo.lock`

Likely test file:

- `crates/sql-lens-app/tests/cli.rs`

Task metadata lives under:

- `.trellis/tasks/07-06-add-cli-entry-point/`

## Dependencies

Add to `sql-lens-app`:

```toml
clap = { version = "4", features = ["derive"] }
sql-lens-config = { path = "../sql-lens-config" }
```

No async runtime or test helper crates should be added in this task.

## CLI Contract

Use clap derive:

```rust
#[derive(clap::Parser)]
#[command(name = "sql-lens", version, about = "...", long_about = None)]
struct Cli {
    #[arg(long, value_name = "FILE", default_value = "sql-lens.toml")]
    config: PathBuf,
}
```

Behavior:

- `sql-lens --version` is handled by clap and exits successfully.
- `sql-lens --config <FILE>` loads that file through `SqlLensConfig::from_path`.
- `sql-lens` without `--config` attempts to load `sql-lens.toml`.
- Loaded config is validated with `SqlLensConfig::validate`.
- Load or validation failure is printed to stderr and exits non-zero.
- Successful load and validation exits zero.

## Error Contract

Use a small app-level error wrapper:

```rust
enum AppError {
    ConfigLoad(ConfigLoadError),
    ConfigValidation(ConfigValidationError),
}
```

`main` should convert it to stderr output and `ExitCode::FAILURE`.

Do not unwrap config load or validation results in main.

## Test Strategy

Use integration tests with standard library only:

- `std::process::Command`.
- `env!("CARGO_BIN_EXE_sql-lens")` for the compiled test binary.
- `std::env::temp_dir` plus a unique directory for config files.

Test cases:

- `--version` exits zero and includes the package version.
- `--config valid.toml` exits zero.
- `--config missing.toml` exits non-zero and stderr includes a read error.
- `--config invalid.toml` exits non-zero and stderr includes validation details.

## Compatibility

- Existing `sql-lens-config` API remains unchanged.
- `sql-lens-app` remains synchronous.
- Future tasks can add logging initialization, service startup, and subcommands around this CLI skeleton.

## Risks

- Defaulting to `sql-lens.toml` means running `sql-lens` without a config file will fail until config generation exists. This matches `CONFIG.md` and keeps startup behavior explicit.
- Integration tests that use the compiled binary should avoid current working directory assumptions by passing explicit temp config paths.

## Rollback

Rollback by removing:

- `clap` and `sql-lens-config` dependencies from `sql-lens-app`.
- CLI implementation in `main.rs`.
- CLI integration tests.
- Lockfile changes introduced only by clap.
