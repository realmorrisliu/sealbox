use std::fs;

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

pub struct CryptoService {
    private_key: Option<RsaPrivateKey>,
    public_key: Option<RsaPublicKey>,
}

impl CryptoService {
    pub fn new() -> Self {
        Self {
            private_key: None,
            public_key: None,
        }
    }

    pub fn load_private_key(&mut self, private_key_path: &str) -> Result<()> {
        let private_key_pem = fs::read_to_string(private_key_path)
            .with_context(|| format!("Failed to read private key file: {private_key_path}"))?;

        let private_key = RsaPrivateKey::from_pkcs1_pem(&private_key_pem)
            .with_context(|| format!("Invalid private key format: {private_key_path}"))?;

        self.private_key = Some(private_key);
        Ok(())
    }

    pub fn load_public_key(&mut self, public_key_path: &str) -> Result<()> {
        let public_key_pem = fs::read_to_string(public_key_path)
            .with_context(|| format!("Failed to read public key file: {public_key_path}"))?;

        let public_key = RsaPublicKey::from_pkcs1_pem(&public_key_pem)
            .with_context(|| format!("Invalid public key format: {public_key_path}"))?;

        self.public_key = Some(public_key);
        Ok(())
    }

    pub fn decrypt_secret(
        &self,
        encrypted_secret: &str,
        encrypted_data_key: &str,
    ) -> Result<String> {
        let private_key = self
            .private_key
            .as_ref()
            .context("Private key not loaded, cannot decrypt")?;

        // Decode base64 encoded data key
        let encrypted_key_bytes = BASE64
            .decode(encrypted_data_key)
            .context("Failed to decode data key from base64")?;

        // Decrypt data key using RSA private key
        let data_key = private_key
            .decrypt(Pkcs1v15Encrypt, &encrypted_key_bytes)
            .context("Failed to decrypt data key with RSA")?;

        // Decode base64 encoded encrypted secret
        let encrypted_bytes = BASE64
            .decode(encrypted_secret)
            .context("Failed to decode secret from base64")?;

        if encrypted_bytes.len() < 12 {
            anyhow::bail!("Invalid encrypted data format: insufficient length");
        }

        // Separate nonce and ciphertext
        let (nonce, ciphertext) = encrypted_bytes.split_at(12);

        // Decrypt using AES-GCM
        let cipher =
            Aes256Gcm::new_from_slice(&data_key).context("Failed to create AES-GCM cipher")?;

        let plaintext = cipher
            .decrypt(nonce.into(), ciphertext)
            .map_err(|e| anyhow::anyhow!("AES-GCM decryption failed: {:?}", e))?;

        String::from_utf8(plaintext).context("Decrypted data is not valid UTF-8")
    }

    pub fn encrypt_secret(&self, plaintext: &str) -> Result<(String, String)> {
        let public_key = self
            .public_key
            .as_ref()
            .context("Public key not loaded, cannot encrypt")?;

        // Generate random data key
        let data_key: [u8; 32] = rand::random();

        // Encrypt secret using AES-GCM
        let cipher =
            Aes256Gcm::new_from_slice(&data_key).context("Failed to create AES-GCM cipher")?;

        let nonce: [u8; 12] = rand::random();
        let ciphertext = cipher
            .encrypt(nonce.as_ref().into(), plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("AES-GCM encryption failed: {:?}", e))?;

        // Combine nonce and ciphertext
        let mut encrypted_data = Vec::with_capacity(12 + ciphertext.len());
        encrypted_data.extend_from_slice(&nonce);
        encrypted_data.extend_from_slice(&ciphertext);

        // Encrypt data key using RSA public key
        let encrypted_key = public_key
            .encrypt(&mut rand::thread_rng(), Pkcs1v15Encrypt, &data_key)
            .context("Failed to encrypt data key with RSA")?;

        // Base64 encode
        let encrypted_secret_b64 = BASE64.encode(&encrypted_data);
        let encrypted_key_b64 = BASE64.encode(&encrypted_key);

        Ok((encrypted_secret_b64, encrypted_key_b64))
    }

    pub fn validate_key_pair(&self) -> Result<()> {
        let private_key = self
            .private_key
            .as_ref()
            .context("Private key not loaded")?;
        let public_key = self.public_key.as_ref().context("Public key not loaded")?;

        // Test encryption and decryption
        let test_message = "test-message";
        let encrypted = public_key
            .encrypt(
                &mut rand::thread_rng(),
                Pkcs1v15Encrypt,
                test_message.as_bytes(),
            )
            .context("Key pair validation: encryption test failed")?;

        let decrypted = private_key
            .decrypt(Pkcs1v15Encrypt, &encrypted)
            .context("Key pair validation: decryption test failed")?;

        let decrypted_str = String::from_utf8(decrypted)
            .context("Key pair validation: decrypted result is not valid string")?;

        if decrypted_str != test_message {
            anyhow::bail!("Key pair validation failed: encryption/decryption mismatch");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_keys() -> Result<(PathBuf, PathBuf, TempDir)> {
        let temp_dir = TempDir::new()?;
        let (private_pem, public_pem) = sealbox_server::crypto::master_key::generate_key_pair()?;

        let private_key_path = temp_dir.path().join("private.pem");
        let public_key_path = temp_dir.path().join("public.pem");

        fs::write(&private_key_path, private_pem)?;
        fs::write(&public_key_path, public_pem)?;

        Ok((private_key_path, public_key_path, temp_dir))
    }

    #[test]
    fn test_load_keys() -> Result<()> {
        let (private_path, public_path, _temp_dir) = create_test_keys()?;

        let mut crypto = CryptoService::new();
        crypto.load_private_key(private_path.to_str().unwrap())?;
        crypto.load_public_key(public_path.to_str().unwrap())?;

        assert!(crypto.private_key.is_some());
        assert!(crypto.public_key.is_some());

        Ok(())
    }

    #[test]
    fn test_validate_key_pair() -> Result<()> {
        let (private_path, public_path, _temp_dir) = create_test_keys()?;

        let mut crypto = CryptoService::new();
        crypto.load_private_key(private_path.to_str().unwrap())?;
        crypto.load_public_key(public_path.to_str().unwrap())?;

        crypto.validate_key_pair()?;

        Ok(())
    }

    #[test]
    fn test_encrypt_decrypt_secret() -> Result<()> {
        let (private_path, public_path, _temp_dir) = create_test_keys()?;

        let mut crypto = CryptoService::new();
        crypto.load_private_key(private_path.to_str().unwrap())?;
        crypto.load_public_key(public_path.to_str().unwrap())?;

        let original_secret = "This is a test password: password123!@#";
        let (encrypted_secret, encrypted_key) = crypto.encrypt_secret(original_secret)?;
        let decrypted_secret = crypto.decrypt_secret(&encrypted_secret, &encrypted_key)?;

        assert_eq!(original_secret, decrypted_secret);

        Ok(())
    }
}
