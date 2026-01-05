# Contributing to Lotus

Thanks for considering contributions! Even though Lotus began as a research prototype, the goal is a production-grade codec. To keep changes smooth:

1. Fork and open a pull request describing the motivation.
2. Add tests for new behaviors and run `cargo fmt`, `cargo clippy`, and `cargo test`.
3. For performance work, attach Criterion output or `scripts/reproduce_paper.sh` results.
4. Keep the public API surface minimal and avoid adding `unsafe` blocks.
5. Use `LotusError` for recoverable errors instead of panicking.

For discussions about new variants or real-world usage, please open a GitHub Discussion so we can track design notes.
