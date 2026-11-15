# Strict Prohibition on Rust Formatters

- **NEVER** execute `cargo fmt`, `rustfmt`, or `cargo xtask fmt` (even with `--check` or scoped to a single file).
- Maintain clean, consistent manual formatting matching surrounding code when editing `.rs` files.
- Verify Rust code quality strictly via `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace --lib`.
