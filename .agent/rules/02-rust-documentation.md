# Rust Code Documentation Standard

Every Rust source file created or edited in `crates/` or `xtask/` MUST follow these documentation conventions:

1. **Module Header (`// ===`)**:
   Start each `.rs` file with a banner explaining purpose, `## Key Concepts`, and `## Architecture Notes`.
2. **Annotated Crate Imports**:
   Document external crate imports above `use` statements explaining what the crate provides and why each imported item is needed:
   ```rust
   // `sea_orm` - Async PostgreSQL ORM.
   // - `DatabaseConnection` - Shared connection pool handle
   use sea_orm::DatabaseConnection;
   ```
3. **Section Headers**:
   Organize logical blocks with `// SECTION: SECTION NAME`.
4. **Rustdoc (`///`) & Inline Why-Comments**:
   Add `///` doc comments on all public structs, enums, traits, and functions; keep inline `//` comments focused on non-obvious invariants or edge cases.
