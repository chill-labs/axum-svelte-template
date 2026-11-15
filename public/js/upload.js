// =============================================================================
// CLIENT-SIDE CANVAS COMPRESSION & XOR MEDIA OBFUSCATION
// =============================================================================
//
// Browser helper providing:
// 1. Canvas WebP resizing/compression (`<= 5 MiB`, matching `MAX_IMAGE_UPLOAD_BYTES`)
// 2. Reversible repeating-key XOR + even-parity adjacent byte-swap obfuscation
//    byte-for-byte compatible with `crates/shared/src/utils/obfuscation.rs`
// 3. Direct R2 presigned PUT upload + metadata confirmation helpers
// 4. In-memory & Cache API decrypted Blob URL loading for `ProtectedImage.svelte`

(function () {
  "use strict";

  const MAX_IMAGE_UPLOAD_BYTES = 5 * 1024 * 1024;
  const DEFAULT_MAX_DIMENSION = 1200;
  const DEFAULT_QUALITY = 0.82;
  const DECRYPTED_CACHE_NAME = "protected-images-v1";

  const memoryBlobCache = new Map();
  const inflightDownloads = new Map();
  const textEncoder = new TextEncoder();

  /**
   * Applies repeating-key XOR followed by adjacent-pair swapping on even parity
   * key bytes. Exact JS mirror of `shared::utils::obfuscation::obfuscate`.
   *
   * @param {Uint8Array} plain
   * @param {string} key
   * @returns {Uint8Array}
   */
  function obfuscateBytes(plain, key) {
    if (!key) return plain.slice();
    const keyBytes = textEncoder.encode(key);
    const keyLen = keyBytes.length;
    if (keyLen === 0) return plain.slice();

    const out = new Uint8Array(plain.length);
    for (let i = 0; i < plain.length; i += 1) {
      out[i] = plain[i] ^ keyBytes[i % keyLen];
    }

    for (let i = 0; i + 1 < out.length; i += 2) {
      if ((keyBytes[i % keyLen] & 1) === 0) {
        const tmp = out[i];
        out[i] = out[i + 1];
        out[i + 1] = tmp;
      }
    }

    return out;
  }

  /**
   * Reverses `obfuscateBytes` (even-parity adjacent swap back, then XOR).
   * Exact JS mirror of `shared::utils::obfuscation::deobfuscate`.
   *
   * @param {Uint8Array} obfuscated
   * @param {string} key
   * @returns {Uint8Array}
   */
  function deobfuscateBytes(obfuscated, key) {
    if (!key) return obfuscated.slice();
    const keyBytes = textEncoder.encode(key);
    const keyLen = keyBytes.length;
    if (keyLen === 0) return obfuscated.slice();

    const out = obfuscated.slice();
    for (let i = 0; i + 1 < out.length; i += 2) {
      if ((keyBytes[i % keyLen] & 1) === 0) {
        const tmp = out[i];
        out[i] = out[i + 1];
        out[i + 1] = tmp;
      }
    }

    for (let i = 0; i < out.length; i += 1) {
      out[i] ^= keyBytes[i % keyLen];
    }

    return out;
  }

  /**
   * Compresses an image `File` on an offscreen `<canvas>` to WebP (with JPEG
   * fallback) and validates the 5 MiB server limit.
   *
   * @param {File} file
   * @param {{ maxDimension?: number, quality?: number, contentType?: string }} [options]
   * @returns {Promise<{ bytes: Uint8Array, contentType: string, width: number, height: number }>}
   */
  async function compressImage(file, options) {
    if (!file || !file.type || !file.type.startsWith("image/")) {
      throw new Error("UNSUPPORTED_MEDIA_TYPE");
    }

    const opts = options || {};
    const maxDimension = opts.maxDimension || DEFAULT_MAX_DIMENSION;
    const quality = typeof opts.quality === "number" ? opts.quality : DEFAULT_QUALITY;
    const targetType = opts.contentType || "image/webp";

    const bitmap = await createImageBitmap(file);
    try {
      let width = bitmap.width;
      let height = bitmap.height;

      if (width > maxDimension || height > maxDimension) {
        const scale = Math.min(maxDimension / width, maxDimension / height);
        width = Math.max(1, Math.round(width * scale));
        height = Math.max(1, Math.round(height * scale));
      }

      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        throw new Error("CANVAS_UNAVAILABLE");
      }
      ctx.drawImage(bitmap, 0, 0, width, height);

      let blob = await new Promise((resolve, reject) => {
        canvas.toBlob(
          (b) => (b ? resolve(b) : reject(new Error("IMAGE_ENCODE_FAILED"))),
          targetType,
          quality
        );
      });

      // Fallback to JPEG if browser falls back to PNG when WebP encode isn't supported.
      let actualType = blob.type || targetType;
      if (targetType === "image/webp" && actualType === "image/png") {
        const jpegBlob = await new Promise((resolve) => {
          canvas.toBlob((b) => resolve(b), "image/jpeg", quality);
        });
        if (jpegBlob && jpegBlob.size < blob.size) {
          blob = jpegBlob;
          actualType = "image/jpeg";
        }
      }

      if (blob.size > MAX_IMAGE_UPLOAD_BYTES) {
        throw new Error("FILE_TOO_LARGE");
      }

      const buffer = await blob.arrayBuffer();
      return {
        bytes: new Uint8Array(buffer),
        contentType: actualType,
        width,
        height,
      };
    } finally {
      bitmap.close();
    }
  }

  /**
   * Executes the full presign -> compress -> XOR obfuscate -> R2 PUT -> confirm
   * upload lifecycle against `/api/v1/media/{presign,confirm}`.
   *
   * @param {{
   *   file: File,
   *   purpose: "user_avatar" | "workspace_logo" | "todo_attachment",
   *   entityId: number,
   *   fieldName?: string,
   *   presignEndpoint?: string,
   *   confirmEndpoint?: string
   * }} params
   */
  async function uploadProtectedImage(params) {
    const {
      file,
      purpose,
      entityId,
      fieldName = "image",
      presignEndpoint = "/api/v1/media/presign",
      confirmEndpoint = "/api/v1/media/confirm",
    } = params;

    const compressed = await compressImage(file);
    const presignRes = await fetch(presignEndpoint, {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        purpose,
        entity_id: entityId,
        content_type: compressed.contentType,
        file_size: compressed.bytes.byteLength,
      }),
    });

    const presignJson = await presignRes.json();
    if (!presignRes.ok || !presignJson.success || !presignJson.data) {
      throw new Error(presignJson.message || "STORAGE_NOT_CONFIGURED");
    }

    const { upload_url, storage_key, xor_key } = presignJson.data;
    const encryptedBytes = obfuscateBytes(compressed.bytes, xor_key);

    const putRes = await fetch(upload_url, {
      method: "PUT",
      body: encryptedBytes,
      headers: { "Content-Type": "application/octet-stream" },
    });
    if (!putRes.ok) {
      throw new Error(`R2_PUT_FAILED_${putRes.status}`);
    }

    const confirmRes = await fetch(confirmEndpoint, {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        purpose,
        entity_id: entityId,
        field_name: fieldName,
        storage_key,
        xor_key,
        content_type: compressed.contentType,
        file_size: encryptedBytes.byteLength,
      }),
    });

    const confirmJson = await confirmRes.json();
    if (!confirmRes.ok || !confirmJson.success || !confirmJson.data) {
      throw new Error(confirmJson.message || "CONFIRM_UPLOAD_FAILED");
    }

    return confirmJson.data;
  }

  /**
   * Downloads an XOR-obfuscated image URL, reverses obfuscation, caches the
   * plaintext image in memory + Cache API, and returns a `blob:...` URL.
   *
   * @param {string} src
   * @param {string | null | undefined} xorKey
   * @param {string} [mimeType]
   * @returns {Promise<string>}
   */
  async function loadProtectedImageUrl(src, xorKey, mimeType = "image/webp") {
    if (!src) return "";
    if (!xorKey) return src;

    const cacheKey = `${src}|${xorKey}`;
    const memHit = memoryBlobCache.get(cacheKey);
    if (memHit) return memHit;

    const existing = inflightDownloads.get(cacheKey);
    if (existing) return existing;

    const task = (async () => {
      try {
        const cacheReqUrl = `${src}${src.includes("?") ? "&" : "?"}__k=${encodeURIComponent(xorKey.slice(0, 8))}`;

        if (typeof caches !== "undefined") {
          try {
            const cache = await caches.open(DECRYPTED_CACHE_NAME);
            const match = await cache.match(cacheReqUrl);
            if (match) {
              const blob = await match.blob();
              const url = URL.createObjectURL(blob);
              memoryBlobCache.set(cacheKey, url);
              return url;
            }
          } catch (_) {}
        }

        const res = await fetch(src);
        if (!res.ok) {
          throw new Error(`ProtectedImage HTTP ${res.status}`);
        }

        const encrypted = new Uint8Array(await res.arrayBuffer());
        const plain = deobfuscateBytes(encrypted, xorKey);
        const blob = new Blob([plain], { type: mimeType });

        if (typeof caches !== "undefined") {
          try {
            const cache = await caches.open(DECRYPTED_CACHE_NAME);
            await cache.put(
              cacheReqUrl,
              new Response(blob, {
                headers: {
                  "Content-Type": mimeType,
                  "Cache-Control": "public, max-age=31536000, immutable",
                },
              })
            );
          } catch (_) {}
        }

        const url = URL.createObjectURL(blob);
        memoryBlobCache.set(cacheKey, url);
        return url;
      } finally {
        inflightDownloads.delete(cacheKey);
      }
    })();

    inflightDownloads.set(cacheKey, task);
    return task;
  }

  window.obfuscateBytes = obfuscateBytes;
  window.deobfuscateBytes = deobfuscateBytes;
  window.compressImage = async function (file, contentType) {
    const result = await compressImage(file, { contentType });
    return result.bytes.buffer;
  };
  window.appMediaUpload = {
    compressImage,
    obfuscateBytes,
    deobfuscateBytes,
    uploadProtectedImage,
    loadProtectedImageUrl,
    MAX_IMAGE_UPLOAD_BYTES,
  };
})();
