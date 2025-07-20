use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use rand::Rng;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataKeyCryptoError {
    #[error("Invalid key length, expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),
    #[error("Failed to decrypt")]
    FailedToDecrypt(aes_gcm::Error),
    #[error("Failed to encrypt")]
    FailedToEncrypt(aes_gcm::Error),
}

pub type Result<T, E = DataKeyCryptoError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct DataKey(Vec<u8>);
impl DataKey {
    pub fn new() -> DataKey {
        let mut rng = rand::thread_rng();
        let mut data_key = vec![0u8; 32];
        rng.fill(&mut data_key[..]);
        DataKey(data_key)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<DataKey> {
        if bytes.len() != 32 {
            return Err(DataKeyCryptoError::InvalidKeyLength(bytes.len()));
        }
        Ok(DataKey(bytes.to_vec()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn cipher(&self) -> Aes256Gcm {
        let data_key = self.0.clone();
        let key = Key::<Aes256Gcm>::from_slice(&data_key);
        let cipher = Aes256Gcm::new(key);
        cipher
    }

    /// Encrypts the data with AES-256-GCM using the provided data key.
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Generate a random nonce (12 bytes)
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the data
        let ciphertext = self
            .cipher()
            .encrypt(&nonce, data)
            .map_err(|err| DataKeyCryptoError::FailedToEncrypt(err))?;

        // Output: [nonce | ciphertext]
        let mut result = nonce.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    /// Decrypts the data with AES-256-GCM using the provided data key.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = self
            .cipher()
            .decrypt(&nonce, ciphertext)
            .map_err(|err| DataKeyCryptoError::FailedToDecrypt(err))?;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_key_new() {
        let key = DataKey::new();
        assert_eq!(key.as_bytes().len(), 32);
    }

    #[test]
    fn test_data_key_from_bytes_valid() {
        let bytes = vec![0u8; 32];
        let key = DataKey::from_bytes(&bytes).expect("Should create DataKey from valid bytes");
        assert_eq!(key.as_bytes(), &bytes);
    }

    #[test]
    fn test_data_key_from_bytes_invalid_length() {
        let bytes = vec![0u8; 16]; // Wrong length
        let result = DataKey::from_bytes(&bytes);
        assert!(result.is_err());
        match result.unwrap_err() {
            DataKeyCryptoError::InvalidKeyLength(len) => assert_eq!(len, 16),
            _ => panic!("Expected InvalidKeyLength error"),
        }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = DataKey::new();
        let plaintext = b"Hello, World! This is a secret message.";
        
        // Encrypt
        let ciphertext = key.encrypt(plaintext).expect("Encryption should succeed");
        
        // Verify ciphertext is different from plaintext
        assert_ne!(ciphertext.as_slice(), plaintext);
        
        // Verify nonce is included (first 12 bytes)
        assert!(ciphertext.len() > 12);
        
        // Decrypt
        let decrypted = key.decrypt(&ciphertext).expect("Decryption should succeed");
        
        // Verify roundtrip
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_different_nonces() {
        let key = DataKey::new();
        let plaintext = b"Same message";
        
        let ciphertext1 = key.encrypt(plaintext).expect("First encryption should succeed");
        let ciphertext2 = key.encrypt(plaintext).expect("Second encryption should succeed");
        
        // Should have different nonces, so ciphertexts should be different
        assert_ne!(ciphertext1, ciphertext2);
        
        // But both should decrypt to the same plaintext
        let decrypted1 = key.decrypt(&ciphertext1).expect("First decryption should succeed");
        let decrypted2 = key.decrypt(&ciphertext2).expect("Second decryption should succeed");
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }

    #[test]
    fn test_different_keys_cannot_decrypt() {
        let key1 = DataKey::new();
        let key2 = DataKey::new();
        let plaintext = b"Secret message";
        
        let ciphertext = key1.encrypt(plaintext).expect("Encryption should succeed");
        
        // Different key should not be able to decrypt
        let result = key2.decrypt(&ciphertext);
        assert!(result.is_err());
        match result.unwrap_err() {
            DataKeyCryptoError::FailedToDecrypt(_) => {}, // Expected
            _ => panic!("Expected FailedToDecrypt error"),
        }
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let key = DataKey::new();
        let invalid_data = vec![0u8; 20]; // Too short, invalid format
        
        let result = key.decrypt(&invalid_data);
        assert!(result.is_err());
        match result.unwrap_err() {
            DataKeyCryptoError::FailedToDecrypt(_) => {}, // Expected
            _ => panic!("Expected FailedToDecrypt error"),
        }
    }

    #[test]
    fn test_empty_plaintext() {
        let key = DataKey::new();
        let plaintext = b"";
        
        let ciphertext = key.encrypt(plaintext).expect("Should encrypt empty data");
        let decrypted = key.decrypt(&ciphertext).expect("Should decrypt empty data");
        
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_plaintext() {
        let key = DataKey::new();
        let plaintext = vec![42u8; 1024 * 10]; // 10KB of data
        
        let ciphertext = key.encrypt(&plaintext).expect("Should encrypt large data");
        let decrypted = key.decrypt(&ciphertext).expect("Should decrypt large data");
        
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_key_bytes_are_random() {
        let key1 = DataKey::new();
        let key2 = DataKey::new();
        
        // Keys should be different (extremely unlikely to be the same)
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }
}
