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
    /// 生成一个新的32字节随机数据密钥，用于AES-256-GCM加密
    /// 
    /// # Returns
    /// 
    /// 返回一个包含随机生成的32字节密钥的 `DataKey` 实例
    /// 
    /// # Examples
    /// 
    /// ```
    /// use sealbox_server::crypto::data_key::DataKey;
    /// 
    /// let data_key = DataKey::new();
    /// assert_eq!(data_key.as_bytes().len(), 32);
    /// ```
    pub fn new() -> DataKey {
        let mut rng = rand::thread_rng();
        let mut data_key = vec![0u8; 32];
        rng.fill(&mut data_key[..]);
        DataKey(data_key)
    }

    /// 从提供的字节数组创建数据密钥
    /// 
    /// # Arguments
    /// 
    /// * `bytes` - 必须为32字节长度的密钥数据
    /// 
    /// # Returns
    /// 
    /// 成功时返回 `Ok(DataKey)`，如果字节长度不是32则返回 `Err(DataKeyCryptoError::InvalidKeyLength)`
    /// 
    /// # Errors
    /// 
    /// * `DataKeyCryptoError::InvalidKeyLength` - 当输入字节长度不是32时
    /// 
    /// # Examples
    /// 
    /// ```
    /// use sealbox_server::crypto::data_key::DataKey;
    /// 
    /// let key_bytes = vec![0u8; 32];
    /// let data_key = DataKey::from_bytes(&key_bytes).unwrap();
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<DataKey> {
        if bytes.len() != 32 {
            return Err(DataKeyCryptoError::InvalidKeyLength(bytes.len()));
        }
        Ok(DataKey(bytes.to_vec()))
    }

    /// 返回数据密钥的字节表示
    /// 
    /// # Returns
    /// 
    /// 返回包含32字节密钥数据的字节切片引用
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// 创建用于AES-256-GCM加密/解密的密码器实例
    /// 
    /// # Returns
    /// 
    /// 返回配置好的 `Aes256Gcm` 密码器实例，可用于后续的加密解密操作
    pub fn cipher(&self) -> Aes256Gcm {
        let data_key = self.0.clone();
        let key = Key::<Aes256Gcm>::from_slice(&data_key);
        let cipher = Aes256Gcm::new(key);
        cipher
    }

    /// 使用AES-256-GCM算法加密数据
    /// 
    /// # Arguments
    /// 
    /// * `data` - 要加密的明文数据
    /// 
    /// # Returns
    /// 
    /// 成功时返回加密后的数据，格式为 [nonce(12字节) | ciphertext]
    /// 
    /// # Errors
    /// 
    /// * `DataKeyCryptoError::FailedToEncrypt` - 加密操作失败时
    /// 
    /// # Security Notes
    /// 
    /// - 每次加密都使用随机生成的nonce，确保相同数据加密后的密文不同
    /// - 输出格式包含12字节nonce + 密文 + 16字节认证标签
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

    /// 使用AES-256-GCM算法解密数据
    /// 
    /// # Arguments
    /// 
    /// * `data` - 要解密的密文数据，必须是由本类 `encrypt` 方法产生的格式
    /// 
    /// # Returns
    /// 
    /// 成功时返回解密后的明文数据
    /// 
    /// # Errors
    /// 
    /// * `DataKeyCryptoError::FailedToDecrypt` - 解密失败（密文损坏、认证失败或格式错误）
    /// 
    /// # Security Notes
    /// 
    /// - 会验证数据完整性和认证标签
    /// - 输入数据必须包含有效的nonce和认证标签
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
