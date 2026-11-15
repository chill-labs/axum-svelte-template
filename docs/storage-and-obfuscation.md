# Optional Object Storage (Cloudflare R2) & XOR Media Obfuscation

## 1. Zero-Config Fallback When R2 Is Unset

Cloudflare R2 is **completely optional**. When `APP__R2__ACCOUNT_ID`, `APP__R2__ACCESS_KEY_ID`, `APP__R2__SECRET_ACCESS_KEY`, or `APP__R2__BUCKET_NAME` are empty (the `.env.example` default):
- `Settings::active_r2()` returns `None`.
- `StorageService::from_settings(&settings)` (`crates/services/src/storage.rs`) returns `None` without network calls or startup errors.
- Upload presign/confirm endpoints return `503 Service Unavailable` with `ErrorCode::StorageNotConfigured` (`"STORAGE_NOT_CONFIGURED"`), allowing the rest of the application (Auth, Workspaces, Todos, SSE, Admin) to run normally.

---

## 2. Reversible Per-File XOR Obfuscation

To prevent direct bucket URLs from rendering raw images without authenticated metadata access, every uploaded file is obfuscated client-side using a server-generated 64-char hex key (`shared::utils::obfuscation::generate_key()`):

1. **Client Compression (`public/js/upload.js` / `svelte-app/src/lib/utils/obfuscation.ts`)**:
   - Decodes image via `createImageBitmap()`, resizes to fit `1200px` / `1280px`, encodes to WebP (`<= 5 MiB`, `MAX_IMAGE_UPLOAD_BYTES`).
2. **Presign (`POST /api/v1/media/presign`)**:
   - Server validates `UploadPurpose` (`user_avatar`, `workspace_logo`, `todo_attachment`), size (`<= 5 MiB`), and MIME type, then returns `{ upload_url, storage_key, xor_key, expires_in_secs }`.
3. **Obfuscate & Direct PUT**:
   - Browser applies repeating-key XOR with UTF-8 bytes of `xor_key`, followed by adjacent-pair swap `(i, i + 1)` whenever `(keyBytes[i % keyBytes.length] & 1) === 0`.
   - Uploads bytes directly to `upload_url` via `PUT` (zero Rust server RAM buffering).
4. **Confirm (`POST /api/v1/media/confirm`)**:
   - Server persists metadata in `media_files` (`database/schema/05_media_files.sql`).
5. **Deobfuscate & Cache (`ProtectedImage.svelte` / `public/js/upload.js`)**:
   - Fetches obfuscated object, swaps adjacent pairs back, XORs with `xor_key`, caches in memory + Cache API (`protected-images-v1`), and renders a `blob:...` URL.

Both `crates/shared/src/utils/obfuscation.rs` and the JS/TS helpers implement the exact same byte algorithm and are unit-tested for odd/even byte lengths.
