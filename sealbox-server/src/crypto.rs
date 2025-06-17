use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use rand::Rng;
use rsa::{
    Oaep, RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey},
};
use sha2::Sha256;

use crate::error::Result;

/// Generate a new RSA key pair for master_key, returning (private_pem, public_pem)
pub fn generate_master_key_pair() -> Result<(String, String)> {
    // Generate 2048-bit RSA key pair
    let mut rng = rand::thread_rng(); // rand@0.8
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pub_key = RsaPublicKey::from(&priv_key);

    // Export to PEM format
    let private_pem = priv_key.to_pkcs1_pem(Default::default())?.to_string();
    let public_pem = pub_key.to_pkcs1_pem(Default::default())?.to_string();

    Ok((private_pem, public_pem))
}

/// Generate a new data key, returning a vector of bytes
pub fn generate_data_key() -> Result<Vec<u8>> {
    let mut rng = rand::thread_rng();
    let mut data_key = vec![0u8; 32];
    rng.fill(&mut data_key[..]);
    Ok(data_key)
}

/// Decrypts the encrypted data key with private key (PEM).
pub fn decrypt_data_key(encrypted_data_key: &[u8], private_key_pem: &str) -> Result<Vec<u8>> {
    let priv_key = rsa::RsaPrivateKey::from_pkcs1_pem(private_key_pem)?;
    let padding = Oaep::new::<Sha256>();
    let decrypted = priv_key.decrypt(padding, encrypted_data_key)?;
    Ok(decrypted)
}

/// Encrypts the data key with public key (PEM).
pub fn encrypt_data_key(data_key: &[u8], public_key_pem: &str) -> Result<Vec<u8>> {
    let pub_key = RsaPublicKey::from_pkcs1_pem(public_key_pem)?;
    let padding = Oaep::new::<Sha256>();
    let encrypted = pub_key.encrypt(&mut rand::thread_rng(), padding, data_key)?;
    Ok(encrypted)
}

/// Encrypts the data with AES-256-GCM using the provided data key.
pub fn encrypt_data(data: &[u8], data_key: &[u8]) -> Result<Vec<u8>> {
    // Convert data_key to Key<Aes256Gcm>
    let key = Key::<Aes256Gcm>::from_slice(data_key);
    let cipher = Aes256Gcm::new(key);

    // Generate a random nonce (12 bytes)
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // Encrypt the data
    let ciphertext = cipher.encrypt(&nonce, data)?;

    // Output: [nonce | ciphertext]
    let mut result = nonce.to_vec();
    result.extend(ciphertext);
    Ok(result)
}

/// Decrypts the data with AES-256-GCM using the provided data key.
pub fn decrypt_data(encrypted_data: &[u8], data_key: &[u8]) -> Result<Vec<u8>> {
    // Convert data_key to Key<Aes256Gcm>
    let key = Key::<Aes256Gcm>::from_slice(data_key);
    let cipher = Aes256Gcm::new(key);

    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let plaintext = cipher.decrypt(nonce, ciphertext)?;
    Ok(plaintext)
}
