// =============================================================================
// CLIENT-SIDE XOR OBFUSCATION & CANVAS IMAGE COMPRESSION
// =============================================================================
// Exact TypeScript twin of `crates/shared/src/utils/obfuscation.rs` so images
// compressed & obfuscated in browser Canvas decrypt identically across clients.

export function obfuscateBytes(data: Uint8Array, xorKey: string): Uint8Array {
  const keyBytes = new TextEncoder().encode(xorKey);
  if (keyBytes.length === 0) return data.slice();

  const out = new Uint8Array(data.length);
  for (let i = 0; i < data.length; i++) {
    out[i] = data[i] ^ keyBytes[i % keyBytes.length];
  }

  for (let i = 0; i + 1 < out.length; i += 2) {
    if ((keyBytes[i % keyBytes.length] & 1) === 0) {
      const tmp = out[i];
      out[i] = out[i + 1];
      out[i + 1] = tmp;
    }
  }

  return out;
}

export function deobfuscateBytes(data: Uint8Array, xorKey: string): Uint8Array {
  const keyBytes = new TextEncoder().encode(xorKey);
  if (keyBytes.length === 0) return data.slice();

  const out = data.slice();
  for (let i = 0; i + 1 < out.length; i += 2) {
    if ((keyBytes[i % keyBytes.length] & 1) === 0) {
      const tmp = out[i];
      out[i] = out[i + 1];
      out[i + 1] = tmp;
    }
  }

  for (let i = 0; i < out.length; i++) {
    out[i] ^= keyBytes[i % keyBytes.length];
  }

  return out;
}

export async function compressImageToWebp(
  file: File,
  maxDimension = 1280,
  quality = 0.84
): Promise<{ bytes: Uint8Array; contentType: string }> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, maxDimension / Math.max(bitmap.width, bitmap.height));
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    throw new Error('Failed to initialize 2D canvas context');
  }
  ctx.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();

  const blob: Blob = await new Promise((resolve, reject) => {
    canvas.toBlob(
      (res) => (res ? resolve(res) : reject(new Error('Canvas encoding returned null'))),
      'image/webp',
      quality
    );
  });

  const buf = await blob.arrayBuffer();
  return {
    bytes: new Uint8Array(buf),
    contentType: 'image/webp'
  };
}
