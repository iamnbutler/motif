# Motif

An immediate-mode UI framework for Rust with GPU rendering.

> **Note:** This project is in early development. APIs are unstable.

## Features

- Type-safe geometry with coordinate space distinction (logical vs device pixels)
- Painter's stack model for hierarchical drawing
- Metal GPU backend (macOS)
- Debug CLI for runtime inspection
- Hot reload support via cargo-hot

## Crates

| Crate | Description |
|-------|-------------|
| `motif` | Main crate, re-exports core |
| `motif_core` | Core types, Metal renderer, text system |
| `motif_debug` | Debug server (Unix socket IPC) |
| `motif_debug_cli` | `motif-debug` CLI binary |
| `motif_test` | Test utilities |

## Quick Start

```bash
cargo run --example playground
```

### Hot Reload

```bash
cargo hot --example hot --features hot
```

Edit `crates/motif/examples/hot.rs` and save to see live changes.

### Debug CLI

```bash
cargo install --path crates/motif_debug_cli
motif-debug scene.stats
motif-debug screenshot
```

See [motif_debug_cli/README.md](crates/motif_debug_cli/README.md) for full command reference.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
