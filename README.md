# tiny-egraph

A minimal, pedagogical e-graph implementation in Rust.

Illustrates the core algorithms but does not compete with production
implementations ( like [egg](https://egraphs-good.github.io/) ) on performance.

```bash
cargo build
cargo test
cargo test --features analysis
```

```bash
cargo run --example arith
cargo run --example boolean
cargo run --example array
cargo run --example constfold --features analysis
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
