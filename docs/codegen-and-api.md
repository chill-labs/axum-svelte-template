# API Conventions & Rust-to-TypeScript Codegen (`gen-ts`)

## 1. Standardized JSON Envelope (`crates/shared/src/domain/response.rs`)

Every `/api/v1/*` handler returns `ApiResponse<T>` paired with the matching HTTP status via `ToResponse<T>` (`to_response()` / `to_response_created()`):

```json
{
  "success": true,
  "message": "OK",
  "status": { "Success": "Ok" },
  "data": { ... },
  "pagination": { "page": 1, "page_size": 20, "total_items": 42, "total_pages": 3 }
}
```

Validation failures return `400 Bad Request` with `ErrorCode::ValidationFailed` (`"VALIDATION_FAILED"`) plus per-field codes so Svelte forms can highlight exact inputs via `uiPrefs.err(code)`.

---

## 2. Three-State `Option<Option<T>>` Partial Updates

`crates/shared/src/domain/request.rs` exports `deserialize_double_option` for PATCH endpoints (`UpdateProfileRequest`, `UpdateWorkspaceRequest`, `UpdateTodoRequest`):
- Field omitted (`{}`) -> `None` (leave column untouched)
- Explicit `null` (`{"description": null}`) -> `Some(None)` (clear column to `NULL`)
- Value (`{"description": "Updated"}`) -> `Some(Some("Updated"))`

Call `req.sanitize()` followed by `req.validate_input()?` inside every service boundary before persistence.

---

## 3. AST Codegen (`crates/api/src/bin/gen_ts.rs`)

Instead of maintaining duplicate TypeScript types manually, `gen-ts` uses `syn` to parse Rust ASTs in `crates/shared/src/domain/` and Axum route registrations in `crates/api/`, generating:
- `svelte-app/src/lib/api/types.generated.ts`: Exact TypeScript interfaces, string-union enums (`ErrorCode`, `TodoStatus`, `TodoPriority`, `UserRole`, `PermResource`, `PermAction`), and `ApiResponse<T>`.
- `svelte-app/src/lib/api/endpoints.generated.ts`: Strongly typed API client wrappers matching method, path params, query params, request body, and response type.

### Workflow
```bash
cargo xtask gen-ts   # or `cargo gen-ts` via .cargo/config.toml alias
```
Never hand-edit `*.generated.ts`. Always regenerate after modifying any Rust DTO or API route.
