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
