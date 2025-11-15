# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AdGuardian-Term is a terminal-based, real-time traffic monitoring and statistics dashboard for AdGuard Home instances. It's written in Rust and displays DNS query logs, blocking statistics, filter lists, and domain analytics in a TUI (Terminal User Interface).

## Development Commands

### Building and Running
- `cargo run` - Build and run the application in development mode
- `cargo build` - Compile the source and dependencies
- `cargo build --release` - Build optimized release binary (output: `./target/release/adguardian`)
- `make` - Full workflow: clean, build, test, doc, and run

### Testing and Quality Assurance
- `cargo test` - Run all unit tests and rustdoc tests
- `cargo check` - Check for compilation errors without building
- `cargo clippy` - Run linter for common mistakes and improvements
- `cargo fmt -- --check` - Check code formatting
- `cargo bench` - Execute benchmark tests
- `cargo doc --no-deps --all-features` - Generate documentation from rustdoc

### Makefile Shortcuts
The Makefile provides convenient shortcuts for common tasks:
- `make run` - Build and run
- `make build` - Compile
- `make test` - Run tests
- `make check` - Check compilation
- `make fmt` - Format check
- `make clippy` - Lint
- `make doc` - Generate docs
- `make clean` - Remove build artifacts
- `make bench` - Run benchmarks

## Architecture

### High-Level Structure

The application follows a **concurrent producer-consumer pattern** with separate data fetching and UI rendering tasks:

1. **Main Entry (`main.rs`)**: Initializes the Tokio runtime, validates configuration via `welcome.rs`, and spawns the async run loop
2. **Data Fetcher (loop in `main.rs`)**: Periodically fetches data from AdGuard Home API endpoints and sends updates through channels
3. **UI Renderer (`ui.rs`)**: Receives data via channels and redraws the terminal interface using the `ratatui` library

### Module Organization

```
src/
├── main.rs           # Entry point, runtime setup, data fetching loop
├── welcome.rs        # Startup banner and config validation (prompts for missing env vars)
├── ui.rs             # Main UI rendering loop, layout management, event handling
├── fetch/            # AdGuard Home API client modules
│   ├── mod.rs
│   ├── fetch_query_log.rs    # DNS query log endpoint
│   ├── fetch_stats.rs        # Statistics endpoint (top domains, clients, counts)
│   ├── fetch_status.rs       # Server status endpoint (version, running state)
│   └── fetch_filters.rs      # Filter list endpoint (enabled blocklists)
└── widgets/          # TUI component rendering
    ├── mod.rs
    ├── chart.rs      # Historical query count line chart
    ├── gauge.rs      # Block percentage gauge
    ├── table.rs      # Query log table
    ├── list.rs       # Generic list widget (top domains/clients)
    ├── status.rs     # Status info paragraph
    └── filters.rs    # Filter list display
```

### Data Flow

1. **Initialization**: `welcome.rs` validates that `ADGUARD_IP`, `ADGUARD_PORT`, `ADGUARD_USERNAME`, `ADGUARD_PASSWORD` are set (prompts if missing)
2. **Channel Setup**: Three `tokio::sync::mpsc` channels are created for query logs, stats, and status data
3. **Concurrent Tasks**:
   - UI task (`draw_ui`) runs in a separate tokio task, receiving updates and rendering
   - Fetcher loop runs at intervals (default 2 seconds, configurable via `ADGUARD_UPDATE_INTERVAL`)
4. **Graceful Shutdown**: User presses 'q', 'Q', or Ctrl+C → UI task notifies shutdown signal → fetcher loop breaks → both tasks clean up

### Key Design Patterns

- **Async/Await**: Uses Tokio for async HTTP requests and concurrent task execution
- **Channel-based Communication**: Decouples data fetching from UI rendering
- **TUI with Ratatui**: Terminal UI library (fork of `tui-rs`) for drawing widgets
- **Error Handling**: Uses `anyhow` crate for idiomatic error propagation

## Configuration

The application requires AdGuard Home credentials, provided via environment variables or CLI flags:

### Required Configuration
- `ADGUARD_IP` / `--adguard-ip` - AdGuard Home instance IP address
- `ADGUARD_PORT` / `--adguard-port` - AdGuard Home port
- `ADGUARD_USERNAME` / `--adguard-username` - Username
- `ADGUARD_PASSWORD` / `--adguard-password` - Password

### Optional Configuration
- `ADGUARD_PROTOCOL` - Protocol to use (default: `http`)
- `ADGUARD_UPDATE_INTERVAL` - UI refresh rate in seconds (default: `2`)

## Dependencies

Key dependencies (see `Cargo.toml` for full list):
- **reqwest** (0.11) - HTTP client with blocking, JSON, and rustls-tls features
- **tokio** (1.x) - Async runtime with full features
- **ratatui** (0.20.1) - Terminal UI library (as `tui` package alias)
- **crossterm** (0.22.0) - Terminal manipulation for keyboard/mouse events
- **serde** + **serde_json** (1.0) - JSON deserialization
- **anyhow** (1.0) - Error handling
- **chrono** (0.4) - Date/time parsing
- **base64** (0.13) - Auth header encoding

## Testing Notes

- Unit tests are embedded in source files using `#[cfg(test)]` modules
- Run `cargo test --all-features` to ensure all feature flags are tested
- `cargo clippy -- -D warnings` treats warnings as errors (used in CI)

## Building Release Binaries

The project uses GitHub Actions to build multi-platform binaries:
- Linux: `adguardian-linux`
- macOS: `adguardian-macos`
- Windows: `adguardian-windows.exe`

For local release builds: `cargo build --release`, binary will be at `./target/release/adguardian`

## Additional Notes

- The crate is published to crates.io as `adguardian` (version in `Cargo.toml`)
- Docker images are available at `lissy93/adguardian` (DockerHub) and `ghcr.io/lissy93/adguardian` (GHCR)
- Documentation is auto-generated and published to GitHub Pages at https://lissy93.github.io/AdGuardian-Term/adguardian
- UI layout adapts to terminal size: bottom panel (filters, top domains, clients) only renders if height > 42 lines
