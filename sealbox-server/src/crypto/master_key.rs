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

pub struct PrivateMasterKey(RsaPrivateKey);

impl PrivateMasterKey {
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let padding = new_padding();
        let decrypted = self
            .0
            .decrypt(padding, ciphertext)
            .map_err(|err| MasterKeyCryptoError::FailedToDecrypt(err))?;
        Ok(decrypted)
    }
}

impl std::str::FromStr for PrivateMasterKey {
    type Err = MasterKeyCryptoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let priv_key = RsaPrivateKey::from_pkcs1_pem(s)
            .map_err(|err| MasterKeyCryptoError::InvalidPkcs1FormatPrivateKey(err))?;
        Ok(PrivateMasterKey(priv_key))
    }
}

pub struct PublicMasterKey(RsaPublicKey);

impl PublicMasterKey {
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let padding = new_padding();
        let encrypted = self
            .0
            .encrypt(&mut rand::thread_rng(), padding, plaintext)
            .map_err(|err| MasterKeyCryptoError::FailedToEncrypt(err))?;
        Ok(encrypted)
    }
}

impl std::str::FromStr for PublicMasterKey {
    type Err = MasterKeyCryptoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let pub_key = RsaPublicKey::from_pkcs1_pem(s)
            .map_err(|err| MasterKeyCryptoError::InvalidPkcs1FormatPublicKey(err))?;
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
        .map_err(|err| MasterKeyCryptoError::FailedToGeneratePrivateKey(err))?;
    let pub_key = RsaPublicKey::from(&priv_key);

    // Export to PEM format
    let private_pem = priv_key
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(|err| MasterKeyCryptoError::FailedToExportPemFormat(err))?
        .to_string();
    let public_pem = pub_key
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(|err| MasterKeyCryptoError::FailedToExportPemFormat(err))?
        .to_string();

    Ok((private_pem, public_pem))
}
