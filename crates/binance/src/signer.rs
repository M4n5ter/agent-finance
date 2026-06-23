use anyhow::{Context, Result};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

pub trait Signer: Send + Sync {
    fn sign(&self, payload: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct HmacSha256Signer {
    secret: String,
}

impl HmacSha256Signer {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }
}

impl Signer for HmacSha256Signer {
    fn sign(&self, payload: &str) -> Result<String> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes())
            .context("failed to create Binance HMAC signer")?;
        mac.update(payload.as_bytes());
        Ok(hex::encode(mac.finalize().into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_known_hmac_sha256_payload() {
        let signer = HmacSha256Signer::new("key");
        let signature = signer
            .sign("The quick brown fox jumps over the lazy dog")
            .unwrap();

        assert_eq!(
            signature,
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }
}
