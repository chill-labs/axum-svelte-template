// =============================================================================
// PER-FILE XOR OBFUSCATION UTILITIES
// =============================================================================
//
// Applies lightweight reversible XOR + adjacent byte-swap obfuscation so images
// stored in Cloudflare R2 cannot be opened as plain JPEGs/WebPs if bucket URLs
// are scraped directly without authenticated metadata keys.

// `rand` - Cryptographic RNG for per-upload obfuscation key generation.
use rand::RngCore;

/// Generates a 32-byte random hex string (`64` hex chars) as an image XOR key.
pub fn generate_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Applies XOR with repeating key bytes followed by adjacent-pair swapping on even rounds.
pub fn obfuscate(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return data.to_vec();
    }

    let mut out: Vec<u8> = data
        .iter()
        .enumerate()
        .map(|(idx, &b)| b ^ key[idx % key.len()])
        .collect();

    let len = out.len();
    for i in (0..len.saturating_sub(1)).step_by(2) {
        if key[i % key.len()] & 1 == 0 {
            out.swap(i, i + 1);
        }
    }

    out
}

/// Reverses `obfuscate` exactly (swap-back in reverse step order, then XOR).
pub fn deobfuscate(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return data.to_vec();
    }

    let mut out = data.to_vec();
    let len = out.len();
    for i in (0..len.saturating_sub(1)).step_by(2) {
        if key[i % key.len()] & 1 == 0 {
            out.swap(i, i + 1);
        }
    }

    for (idx, b) in out.iter_mut().enumerate() {
        *b ^= key[idx % key.len()];
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_obfuscation_preserves_bytes() {
        let original = b"WEBP-BINARY-PAYLOAD-FOR-PROTECTED-IMAGE-CHECK-12345".to_vec();
        let key = generate_key();
        assert_eq!(key.len(), 64);

        let encrypted = obfuscate(&original, key.as_bytes());
        assert_ne!(encrypted, original);

        let decrypted = deobfuscate(&encrypted, key.as_bytes());
        assert_eq!(decrypted, original);
    }

    #[test]
    fn roundtrip_odd_length_payload() {
        let original = vec![10, 20, 30, 40, 50];
        let key = b"sample-key-01";
        let enc = obfuscate(&original, key);
        assert_eq!(deobfuscate(&enc, key), original);
    }
}
