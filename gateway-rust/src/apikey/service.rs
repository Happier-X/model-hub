//! Key 生成、哈希与脱敏。

use rand::RngCore;
use sha2::{Digest, Sha256};

pub const API_KEY_PREFIX: &str = "sk-modelhub-";

/// 生成 `sk-modelhub-` + 高熵随机 hex。
pub fn generate_raw_key() -> String {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{API_KEY_PREFIX}{}", hex::encode(bytes))
}

/// SHA-256(raw) hex；存储与校验均用此值，不存明文。
pub fn hash_key(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    hex::encode(digest)
}

/// 脱敏展示：保留前缀与末 4 位。
pub fn mask_key(raw: &str) -> String {
    if raw.len() <= API_KEY_PREFIX.len() + 4 {
        return format!("{API_KEY_PREFIX}****");
    }
    let suffix = &raw[raw.len() - 4..];
    format!("{API_KEY_PREFIX}****{suffix}")
}

/// 恒定时间比较哈希（避免早期返回泄漏）。
pub fn hashes_equal(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_has_prefix_and_entropy() {
        let key = generate_raw_key();
        assert!(key.starts_with(API_KEY_PREFIX));
        assert!(key.len() > API_KEY_PREFIX.len() + 16);
        let other = generate_raw_key();
        assert_ne!(key, other);
    }

    #[test]
    fn hash_is_stable_and_not_raw() {
        let raw = "sk-modelhub-deadbeefcafebabe";
        let h1 = hash_key(raw);
        let h2 = hash_key(raw);
        assert_eq!(h1, h2);
        assert_ne!(h1, raw);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn mask_hides_middle() {
        let raw = "sk-modelhub-0123456789abcdef";
        let masked = mask_key(raw);
        assert!(masked.starts_with(API_KEY_PREFIX));
        assert!(masked.contains("****"));
        assert!(masked.ends_with("cdef"));
        assert!(!masked.contains("0123456789ab"));
    }

    #[test]
    fn store_never_keeps_plaintext_via_hash_only() {
        let raw = generate_raw_key();
        let stored = hash_key(&raw);
        assert!(!stored.contains("sk-modelhub-"));
        assert_ne!(stored, raw);
    }
}
