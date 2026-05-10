# tiny-egraph

A minimal, pedagogical e-graph implementation in Rust.

## Build

```bash
cargo build
cargo test
cargo test --features analysis
```

## Examples

```bash
cargo run --example arith
cargo run --example boolean
cargo run --example array
cargo run --example constfold --features analysis
```

## License

MIT.
