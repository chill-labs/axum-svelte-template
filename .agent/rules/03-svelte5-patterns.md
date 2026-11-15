# Svelte 5 Runes, Tailwind v4 & Frontend Patterns

1. **Svelte 5 Runes Only**:
   - Use `$state`, `$derived`, `$effect`, and `$props` across all `.svelte` and `.svelte.ts` files. Do not introduce Svelte 4 `export let` props or `$:` reactive statements.
2. **Generated API Contracts**:
   - Consume DTOs and endpoints exclusively from `$lib/api/types.generated.ts` and `$lib/api/endpoints.generated.ts`. Never hand-edit generated files—run `cargo xtask gen-ts` whenever Rust DTOs or handlers change.
3. **Bilingual Error & UI Translation**:
   - Translate every backend `ErrorCode` via `uiPrefs.err(code)` (`svelte-app/src/lib/i18n/{en,vi}.ts`). Keep `en.ts`, `vi.ts`, and `types.ts` keys 100% synchronized.
4. **Shared Styling & Static Assets**:
   - Add reusable Soft UI theme tokens, gradients, and dark-mode variables to `style/input.css` (shared with `svelte-app/src/app.css` via `vite.config.ts`).
   - Reference static assets directly at `/logo/*`, `/icons/*`, `/guides/*`, and `/js/*` (never `/pkg/*`).
5. **Protected Media & Encryption**:
   - Keep `svelte-app/src/lib/utils/obfuscation.ts` and `public/js/upload.js` byte-for-byte compatible with `crates/shared/src/utils/obfuscation.rs`.
