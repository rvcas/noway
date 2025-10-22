# noway - Wayback Machine Downloader

## Commands

- **Build**: `cargo build --release` - creates static binary at `target/release/noway`
- **Check**: `cargo check` - fast type checking
- **Lint**: `cargo clippy` - run clippy linter
- **Run**: `cargo run --release -- <url>` or `./run <url>`

## Architecture

Single-binary Rust CLI tool that downloads archived pages from the Wayback Machine. Core flow:
1. Query Wayback CDX API for all archived URLs matching input
2. Spawn parallel tokio tasks (5 concurrent by default) with semaphore for rate limiting
3. Download HTML content to output directory (random name or specified with `-o`)

## Code Style

- **Error handling**: Use `miette` with `.into_diagnostic()` for std/external errors, `.context()` for user-facing messages
- **Async**: Tokio runtime, reqwest with `rustls-tls` (no OpenSSL dependencies for static builds)
- **Concurrency**: Semaphore pattern for rate limiting, not raw task spawning
- **Dependencies**: Keep minimal - tokio, clap, reqwest, miette, serde_json, names, url, urlencoding
- **Simplicity**: Prefer clarity over DRY, professional but pragmatic
- **CLI**: Use clap derive macros, sensible defaults, short/long flags
