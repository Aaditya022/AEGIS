use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("signature verification failed")]
    SignatureVerification,
    #[error("invalid key format: {0}")]
    InvalidKeyFormat(String),
}

pub type Result<T> = std::result::Result<T, CryptoError>;

pub fn hash_bytes(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

pub fn hash_str(data: &str) -> String {
    hex::encode(hash_bytes(data.as_bytes()))
}

pub fn sign(key: &SigningKey, data: &[u8]) -> Signature {
    key.sign(data)
}

pub fn verify(key: &VerifyingKey, data: &[u8], signature: &Signature) -> Result<()> {
    key.verify(data, signature)
        .map_err(|_| CryptoError::SignatureVerification)
}

pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

pub fn hash_chain(prev_hash: &[u8], event_data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash);
    hasher.update(event_data);
    hasher.finalize().to_vec()
}
