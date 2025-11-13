# Mermaid ASCII (Rust Port)

Rust rewrite of [`mermaid-ascii`](https://github.com/AlexanderGrooff/mermaid-ascii), a CLI that renders Mermaid diagrams into ASCII/Unicode art directly in your terminal.

## Status

- ✅ CLI scaffolding (`cargo run`) with argument parsing/logging
- ✅ Mermaid parser (graph structure, styles, subgraphs)
- 🚧 Graph layout and drawing code currently in progress

## Getting Started

```bash
cargo run -- --file examples/basic.mermaid
```

Flags mirror the original Go tool:

- `-f, --file` (use `-` or omit for stdin)
- `-v, --verbose`
- `-a, --ascii`
- `-c, --coords`
- `-x, --paddingX <int>`
- `-y, --paddingY <int>`
- `-p, --borderPadding <int>`

## Development

```
cargo fmt
cargo clippy
cargo test
```

## License

MIT (same as upstream). See `LICENSE` when added.
