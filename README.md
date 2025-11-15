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

Additional samples live in `examples/`:

- `basic.mermaid` – simple LR flow
- `labels.mermaid` – labeled edges
- `subgraph.mermaid` – nested groups (work-in-progress)
- `complex.mermaid` – larger TD pipeline with subgraphs, labels, and decisions

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
