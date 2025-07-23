use rsa::{
    Oaep, RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey},
    pkcs8::LineEnding,
};
use sha2::Sha256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MasterKeyCryptoError {
    #[error("Invalid private key")]
    InvalidPkcs1FormatPrivateKey(rsa::pkcs1::Error),
    #[error("Invalid public key")]
    InvalidPkcs1FormatPublicKey(rsa::pkcs1::Error),

    #[error("Failed to decrypt")]
    FailedToDecrypt(rsa::Error),
    #[error("Failed to encrypt")]
    FailedToEncrypt(rsa::Error),

    #[error("Failed to generate private key")]
    FailedToGeneratePrivateKey(rsa::Error),
    #[error("Failed to export PEM format")]
    FailedToExportPemFormat(rsa::pkcs1::Error),
}

pub type Result<T, E = MasterKeyCryptoError> = std::result::Result<T, E>;

fn new_padding() -> Oaep {
    Oaep::new::<Sha256>()
}

#[derive(Debug)]
pub struct PrivateMasterKey(RsaPrivateKey);

impl PrivateMasterKey {
    /// Decrypt data using private key
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The encrypted data to decrypt
    ///
    /// # Returns
    ///
    /// Returns the decrypted plaintext data on success
    ///
    /// # Errors
    ///
    /// * `MasterKeyCryptoError::FailedToDecrypt` - Decryption failed (possibly corrupted ciphertext or key mismatch)
    ///
    /// # Security Notes
    ///
    /// - Uses RSA-OAEP-SHA256 padding scheme
    /// - For 2048-bit RSA keys, maximum decryptable data length is approximately 190 bytes
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let padding = new_padding();
        let decrypted = self
            .0
            .decrypt(padding, ciphertext)
            .map_err(MasterKeyCryptoError::FailedToDecrypt)?;
        Ok(decrypted)
    }
}

impl std::str::FromStr for PrivateMasterKey {
    type Err = MasterKeyCryptoError;

    /// Parse private key from PKCS#1 PEM format string
    ///
    /// # Arguments
    ///
    /// * `s` - PKCS#1 PEM format private key string, must start with "-----BEGIN RSA PRIVATE KEY-----"
    ///
    /// # Returns
    ///
    /// Returns `PrivateMasterKey` instance on success
    ///
    /// # Errors
    ///
    /// * `MasterKeyCryptoError::InvalidPkcs1FormatPrivateKey` - Invalid PEM format or not a valid RSA private key
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let priv_key = RsaPrivateKey::from_pkcs1_pem(s)
            .map_err(MasterKeyCryptoError::InvalidPkcs1FormatPrivateKey)?;
        Ok(PrivateMasterKey(priv_key))
    }
}

#[derive(Debug)]
pub struct PublicMasterKey(RsaPublicKey);

impl PublicMasterKey {
    /// Encrypt data using public key
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The plaintext data to encrypt, length cannot exceed maximum supported by RSA key
    ///
    /// # Returns
    ///
    /// Returns the encrypted ciphertext data on success
    ///
    /// # Errors
    ///
    /// * `MasterKeyCryptoError::FailedToEncrypt` - Encryption failed (usually data too long or key format error)
    ///
    /// # Security Notes
    ///
    /// - Uses RSA-OAEP-SHA256 padding scheme
    /// - For 2048-bit RSA keys, maximum encryptable data length is approximately 190 bytes
    /// - Each encryption of the same data produces different ciphertext (due to random padding)
    ///
    /// # Examples
    ///
    /// ```
    /// use sealbox_server::crypto::master_key::PublicMasterKey;
    ///
    /// // Note: This example requires a valid PEM format public key string
    /// // let public_key: PublicMasterKey = pem_string.parse()?;
    /// // let plaintext = b"Hello, World!";
    /// // let ciphertext = public_key.encrypt(plaintext)?;
    /// ```
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let padding = new_padding();
        let encrypted = self
            .0
            .encrypt(&mut rand::thread_rng(), padding, plaintext)
            .map_err(MasterKeyCryptoError::FailedToEncrypt)?;
        Ok(encrypted)
    }
}

impl std::str::FromStr for PublicMasterKey {
    type Err = MasterKeyCryptoError;

    /// Parse public key from PKCS#1 PEM format string
    ///
    /// # Arguments
    ///
    /// * `s` - PKCS#1 PEM format public key string, must start with "-----BEGIN RSA PUBLIC KEY-----"
    ///
    /// # Returns
    ///
    /// Returns `PublicMasterKey` instance on success
    ///
    /// # Errors
    ///
    /// * `MasterKeyCryptoError::InvalidPkcs1FormatPublicKey` - Invalid PEM format or not a valid RSA public key
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pub_key = RsaPublicKey::from_pkcs1_pem(s)
            .map_err(MasterKeyCryptoError::InvalidPkcs1FormatPublicKey)?;
        Ok(PublicMasterKey(pub_key))
    }
}

/// Generate a new RSA key pair for master_key, returning (private_pem, public_pem).
///
/// **Note: This function is intended for client-side use only.** The server should
/// never generate or handle private keys as per the E2EE design. The private key
/// must remain on the client.
pub fn generate_key_pair() -> Result<(String, String), MasterKeyCryptoError> {
    // Generate 2048-bit RSA key pair
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits)
        .map_err(MasterKeyCryptoError::FailedToGeneratePrivateKey)?;
    let pub_key = RsaPublicKey::from(&priv_key);

    // Export to PEM format
    let private_pem = priv_key
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(MasterKeyCryptoError::FailedToExportPemFormat)?
        .to_string();
    let public_pem = pub_key
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(MasterKeyCryptoError::FailedToExportPemFormat)?
        .to_string();

    Ok((private_pem, public_pem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_pair() {
        let result = generate_key_pair();
        assert!(result.is_ok());

        let (private_pem, public_pem) = result.unwrap();

        // Check that we got valid PEM strings
        assert!(private_pem.starts_with("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(private_pem.ends_with("-----END RSA PRIVATE KEY-----\n"));

        assert!(public_pem.starts_with("-----BEGIN RSA PUBLIC KEY-----"));
        assert!(public_pem.ends_with("-----END RSA PUBLIC KEY-----\n"));

        // Verify we can parse them back
        let _private_key: PrivateMasterKey = private_pem.parse().expect("Should parse private key");
        let _public_key: PublicMasterKey = public_pem.parse().expect("Should parse public key");
    }

    #[test]
    fn test_private_key_from_str_valid() {
        let (private_pem, _) = generate_key_pair().expect("Should generate key pair");
        let _private_key: PrivateMasterKey = private_pem.parse().expect("Should parse private key");

        // If we got here, the parsing worked
        // Test placeholder - functionality verified by integration tests
    }

    #[test]
    fn test_private_key_from_str_invalid() {
        let invalid_pem = "invalid pem data";
        let result = invalid_pem.parse::<PrivateMasterKey>();
        assert!(result.is_err());

        match result.unwrap_err() {
            MasterKeyCryptoError::InvalidPkcs1FormatPrivateKey(_) => {} // Expected
            _ => panic!("Expected InvalidPkcs1FormatPrivateKey error"),
        }
    }

    #[test]
    fn test_public_key_from_str_valid() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let _public_key: PublicMasterKey = public_pem.parse().expect("Should parse public key");

        // If we got here, the parsing worked
        // Test placeholder - functionality verified by integration tests
    }

    #[test]
    fn test_public_key_from_str_invalid() {
        let invalid_pem = "invalid pem data";
        let result = invalid_pem.parse::<PublicMasterKey>();
        assert!(result.is_err());

        match result.unwrap_err() {
            MasterKeyCryptoError::InvalidPkcs1FormatPublicKey(_) => {} // Expected
            _ => panic!("Expected InvalidPkcs1FormatPublicKey error"),
        }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (private_pem, public_pem) = generate_key_pair().expect("Should generate key pair");
        let private_key: PrivateMasterKey = private_pem.parse().expect("Should parse private key");
        let public_key: PublicMasterKey = public_pem.parse().expect("Should parse public key");

        let plaintext = b"Hello, this is a secret message!";

        // Encrypt with public key
        let ciphertext = public_key
            .encrypt(plaintext)
            .expect("Should encrypt successfully");

        // Verify ciphertext is different from plaintext
        assert_ne!(ciphertext.as_slice(), plaintext);

        // Decrypt with private key
        let decrypted = private_key
            .decrypt(&ciphertext)
            .expect("Should decrypt successfully");

        // Verify roundtrip
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_different_results() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let public_key: PublicMasterKey = public_pem.parse().expect("Should parse public key");

        let plaintext = b"Same message";

        // Encrypt the same message twice
        let ciphertext1 = public_key
            .encrypt(plaintext)
            .expect("First encryption should succeed");
        let ciphertext2 = public_key
            .encrypt(plaintext)
            .expect("Second encryption should succeed");

        // Results should be different due to random padding
        assert_ne!(ciphertext1, ciphertext2);
    }

    #[test]
    fn test_wrong_private_key_cannot_decrypt() {
        let (_, public_pem1) = generate_key_pair().expect("Should generate first key pair");
        let (private_pem2, _) = generate_key_pair().expect("Should generate second key pair");

        let public_key1: PublicMasterKey = public_pem1.parse().expect("Should parse public key");
        let private_key2: PrivateMasterKey =
            private_pem2.parse().expect("Should parse private key");

        let plaintext = b"Secret message";

        // Encrypt with first public key
        let ciphertext = public_key1
            .encrypt(plaintext)
            .expect("Should encrypt successfully");

        // Try to decrypt with second private key (should fail)
        let result = private_key2.decrypt(&ciphertext);
        assert!(result.is_err());

        match result.unwrap_err() {
            MasterKeyCryptoError::FailedToDecrypt(_) => {} // Expected
            _ => panic!("Expected FailedToDecrypt error"),
        }
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        let (private_pem, _) = generate_key_pair().expect("Should generate key pair");
        let private_key: PrivateMasterKey = private_pem.parse().expect("Should parse private key");

        let invalid_ciphertext = vec![0u8; 32]; // Invalid ciphertext

        let result = private_key.decrypt(&invalid_ciphertext);
        assert!(result.is_err());

        match result.unwrap_err() {
            MasterKeyCryptoError::FailedToDecrypt(_) => {} // Expected
            _ => panic!("Expected FailedToDecrypt error"),
        }
    }

    #[test]
    fn test_encrypt_empty_data() {
        let (private_pem, public_pem) = generate_key_pair().expect("Should generate key pair");
        let private_key: PrivateMasterKey = private_pem.parse().expect("Should parse private key");
        let public_key: PublicMasterKey = public_pem.parse().expect("Should parse public key");

        let plaintext = b"";

        let ciphertext = public_key
            .encrypt(plaintext)
            .expect("Should encrypt empty data");
        let decrypted = private_key
            .decrypt(&ciphertext)
            .expect("Should decrypt empty data");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_max_size_data() {
        let (private_pem, public_pem) = generate_key_pair().expect("Should generate key pair");
        let private_key: PrivateMasterKey = private_pem.parse().expect("Should parse private key");
        let public_key: PublicMasterKey = public_pem.parse().expect("Should parse public key");

        // For 2048-bit RSA with OAEP-SHA256, max plaintext is around 190 bytes
        let plaintext = vec![42u8; 190];

        let ciphertext = public_key
            .encrypt(&plaintext)
            .expect("Should encrypt max size data");
        let decrypted = private_key
            .decrypt(&ciphertext)
            .expect("Should decrypt max size data");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_oversized_data_fails() {
        let (_, public_pem) = generate_key_pair().expect("Should generate key pair");
        let public_key: PublicMasterKey = public_pem.parse().expect("Should parse public key");

        // Data too large for RSA encryption (should be > 214 bytes for 2048-bit RSA with OAEP)
        let plaintext = vec![42u8; 300];

        let result = public_key.encrypt(&plaintext);
        assert!(result.is_err());

        match result.unwrap_err() {
            MasterKeyCryptoError::FailedToEncrypt(_) => {} // Expected
            _ => panic!("Expected FailedToEncrypt error"),
        }
    }

    #[test]
    fn test_generate_different_key_pairs() {
        let (private_pem1, public_pem1) =
            generate_key_pair().expect("Should generate first key pair");
        let (private_pem2, public_pem2) =
            generate_key_pair().expect("Should generate second key pair");

        // Keys should be different
        assert_ne!(private_pem1, private_pem2);
        assert_ne!(public_pem1, public_pem2);
    }
}
