use crate::error::Result;

pub(crate) mod sqlite;

#[derive(Debug)]
pub struct Secret {
    pub namespace: String,
    pub key: String,
    pub version: i32,
    pub encrypted_data: Vec<u8>,
    pub encrypted_data_key: Vec<u8>,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: Option<i64>,
    pub metadata: Option<String>,
    pub access_count: i32,
}

impl Secret {
    pub(crate) async fn create(key: &str) -> Result<Self> {
        Ok(Self {
            namespace: String::new(),
            key: key.to_string(),
            version: 1,
            encrypted_data: Vec::new(),
            encrypted_data_key: Vec::new(),
            created_at: 0,
            updated_at: 0,
            expires_at: None,
            metadata: None,
            access_count: 0,
        })
    }
}

pub(crate) trait SecretRepo: Send + Sync {
    fn get_secret(&self, key: &str) -> Option<Secret>;
    fn save_secret(&self, secret: &Secret);
    fn delete_secret(&self, key: &str);
}
