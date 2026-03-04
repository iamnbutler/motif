# Contributing to Motif

Thanks for your interest in contributing! Motif is an early-stage immediate-mode UI framework in Rust, and all contributions — code, docs, bug reports, or ideas — are welcome.

## Prerequisites

| Tool | Notes |
|------|-------|
| **Rust** (stable) | Install via [rustup](https://rustup.rs/) |
| **macOS** | Required for the Metal GPU backend (clippy, tests, and examples run on macOS only) |
| **cargo-hot** (optional) | Hot-reload support — `cargo install cargo-hot` |

> **Note:** `cargo fmt` and `cargo doc` work on Linux/Windows. Clippy, tests, and GPU examples require macOS.

## Repository Layout

```
crates/
  motif/           # Public API crate — re-exports motif_core
  motif_core/      # Core types, Metal renderer, text system, element traits
  motif_debug/     # Debug server (Unix socket IPC) and snapshot utilities
  motif_debug_cli/ # `motif-debug` binary — runtime inspection CLI
  motif_test/      # Integration test harness (TestHarness, TestRenderContext)
scripts/
  release.sh       # Publish helper (used by maintainers)
docs/
  benches.jsonl    # Benchmark history (auto-updated by CI)
```

## Building

```bash
# Check that the workspace parses (works on any platform)
cargo metadata --no-deps

# Full build (macOS required)
cargo build --workspace

# Run an example
cargo run --example playground

# Hot-reload example
cargo hot --example hot --features hot
```

## Testing

Motif uses a TDD workflow. New features should have tests written first.

```bash
# Run all tests (macOS required for integration tests)
cargo test --workspace

# Run only unit tests that don't need Metal
cargo test -p motif_core

# Run a specific test
cargo test -p motif_test harness_hit_test_single
```

**What to test and what not to:**

- **Do test**: business logic, hit-testing, scene building, element layout, protocol serialization
- **Do not test**: things the compiler can already validate (type constructors, field types, etc.)

The `motif_test` crate provides `TestHarness` and `TestRenderContext` for writing integration tests without a real window.

## Code Style

```bash
cargo fmt --all          # Format (required before submitting)
cargo clippy --workspace -- -D warnings   # Lint (macOS)
```

Follow the existing naming conventions and module structure. Keep changes focused — one concern per PR.

## Using the Debug CLI

While developing, `motif-debug` lets you inspect a running app without stopping it:

```bash
cargo install --path crates/motif_debug_cli

motif-debug scene.stats      # quad/text counts, viewport
motif-debug scene.quads      # list all quads
motif-debug screenshot       # capture current frame
motif-debug                  # REPL mode
```

## Using Spool

The project tracks tasks using `spool`, a lightweight task-tracking tool. The `TODO.md` at the repo root is auto-generated from spool. You can view the current backlog there before picking up work.

## Submitting a Pull Request

1. Fork the repo and create a branch: `git checkout -b feat/my-feature`
2. Write tests first, make them fail, then implement the change
3. Run `cargo fmt --all` and fix any `cargo clippy` warnings
4. Keep the PR small and focused
5. Describe *why* the change is needed, not just what it does

For larger changes, open an issue first to discuss the approach.

## Reporting Bugs

Open an issue with:
- What you expected to happen
- What actually happened
- A minimal reproduction (ideally a `motif_test`-style test case)

## License

Contributions are licensed under the same dual MIT / Apache-2.0 license as the rest of the project.
