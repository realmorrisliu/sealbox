use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Read, Write},
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use reqwest::Client;
use sealbox_server::{
    crypto::{
        data_key::DataKey,
        master_key::{PrivateMasterKey, PublicMasterKey},
    },
    repo::SecretInfo,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{config::Config, output::OutputManager};

use super::secret_commands::{fetch_decrypted_secret, save_secret_value};

const ENVELOPE_VERSION: u32 = 1;
const ARCHIVE_FORMAT_VERSION: u32 = 1;
const ARCHIVE_TYPE: &str = "sealbox.encrypted-tar";
const ARCHIVE_CIPHER: &str = "AES-256-GCM";
const KEY_CIPHER: &str = "RSA-OAEP-SHA256";
const MANIFEST_PATH: &str = "manifest.json";
const SECRETS_PATH: &str = "secrets.json";

#[derive(Debug, Deserialize)]
struct ListSecretsResponse {
    secrets: Vec<SecretInfo>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct EncryptedExportEnvelopeV1 {
    envelope_version: u32,
    archive_type: String,
    archive_cipher: String,
    key_cipher: String,
    encrypted_data_key_b64: String,
    encrypted_archive_b64: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ExportManifestV1 {
    format_version: u32,
    application: String,
    exported_at: i64,
    secret_count: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ExportSecretRecordV1 {
    key: String,
    value: String,
    version: i32,
    expires_at: Option<i64>,
    metadata: Option<String>,
}

#[derive(Debug)]
struct ExportArchiveV1 {
    manifest: ExportManifestV1,
    records: Vec<ExportSecretRecordV1>,
}

pub async fn export_secrets(
    config: &Config,
    output: &OutputManager,
    file_path: String,
    keys_pattern: Option<String>,
    format: String,
) -> Result<()> {
    validate_archive_format(&format)?;
    config
        .validate()
        .context("Configuration validation failed")?;

    output.print_info("Fetching secret list...");
    let mut secret_infos = fetch_secret_infos(config).await?;

    if let Some(pattern) = keys_pattern {
        secret_infos.retain(|secret| secret.key.contains(&pattern));
    }

    if secret_infos.is_empty() {
        anyhow::bail!("No secrets matched export criteria");
    }

    output.print_info("Decrypting secrets locally for encrypted archive export...");
    let mut records = Vec::with_capacity(secret_infos.len());
    for secret in secret_infos {
        let decrypted = fetch_decrypted_secret(config, &secret.key, Some(secret.version))
            .await
            .with_context(|| {
                format!(
                    "Failed to decrypt '{}' version {} for export",
                    secret.key, secret.version
                )
            })?;
        records.push(ExportSecretRecordV1 {
            key: decrypted.key,
            value: decrypted.value,
            version: decrypted.version,
            expires_at: decrypted.expires_at,
            metadata: decrypted.metadata,
        });
    }

    let manifest = ExportManifestV1 {
        format_version: ARCHIVE_FORMAT_VERSION,
        application: "sealbox-cli".to_string(),
        exported_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        secret_count: records.len(),
    };
    let tar_bytes = build_archive_tar(&manifest, &records)?;
    let envelope = encrypt_archive(config, &tar_bytes)?;

    write_private_file(&file_path, &serde_json::to_vec_pretty(&envelope)?)?;

    output.print_success(&format!(
        "Exported {} secrets to encrypted archive: {}",
        records.len(),
        file_path
    ));
    output.print_value(&json!({
        "file": file_path,
        "archive_type": ARCHIVE_TYPE,
        "envelope_version": ENVELOPE_VERSION,
        "format_version": ARCHIVE_FORMAT_VERSION,
        "secret_count": records.len()
    }))?;

    Ok(())
}

pub async fn import_secrets(
    config: &Config,
    output: &OutputManager,
    file_path: String,
    format: String,
) -> Result<()> {
    validate_archive_format(&format)?;
    config
        .validate()
        .context("Configuration validation failed")?;

    output.print_info(&format!(
        "Reading and decrypting encrypted archive: {file_path}"
    ));
    let envelope_bytes =
        fs::read(&file_path).with_context(|| format!("Failed to read archive: {file_path}"))?;
    let envelope: EncryptedExportEnvelopeV1 =
        serde_json::from_slice(&envelope_bytes).context("Failed to parse export envelope")?;
    let tar_bytes = decrypt_archive(config, &envelope)?;
    let archive = read_archive_tar(&tar_bytes)?;
    let records = migrate_archive(archive)?;

    output.print_info(&format!(
        "Importing {} secrets from archive format version {}...",
        records.len(),
        ARCHIVE_FORMAT_VERSION
    ));

    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut imported_count = 0usize;
    let mut skipped_count = 0usize;

    for record in records {
        let ttl = match record.expires_at {
            Some(expires_at) if expires_at <= now => {
                skipped_count += 1;
                output.print_warning(&format!(
                    "Skipping expired secret '{}' version {}",
                    record.key, record.version
                ));
                continue;
            }
            Some(expires_at) => Some(expires_at - now),
            None => None,
        };

        save_secret_value(
            config,
            output,
            record.key.clone(),
            record.value,
            ttl,
            record.metadata,
        )
        .await
        .with_context(|| format!("Failed to import secret '{}'", record.key))?;
        imported_count += 1;
    }

    output.print_success(&format!(
        "Import completed! Imported: {imported_count}, skipped expired: {skipped_count}"
    ));
    output.print_value(&json!({
        "file": file_path,
        "imported": imported_count,
        "skipped_expired": skipped_count,
        "format_version": ARCHIVE_FORMAT_VERSION
    }))?;

    Ok(())
}

async fn fetch_secret_infos(config: &Config) -> Result<Vec<SecretInfo>> {
    let client = Client::new();
    let response = client
        .get(config.api_url("secrets"))
        .bearer_auth(&config.server.token)
        .send()
        .await
        .context("Failed to request server")?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to get error information".to_string());
        anyhow::bail!(
            "Server returned error while listing secrets (status code: {}):\n{}",
            status,
            error_body
        );
    }

    let result: ListSecretsResponse = response
        .json()
        .await
        .context("Failed to parse secret list response")?;
    Ok(result.secrets)
}

fn validate_archive_format(format: &str) -> Result<()> {
    match format {
        "encrypted-tar" | "sealbox-v1" => Ok(()),
        _ => anyhow::bail!(
            "Unsupported archive format: {}. Supported formats: encrypted-tar, sealbox-v1",
            format
        ),
    }
}

fn encrypt_archive(config: &Config, tar_bytes: &[u8]) -> Result<EncryptedExportEnvelopeV1> {
    let public_key_pem = fs::read_to_string(&config.keys.public_key_path).with_context(|| {
        format!(
            "Failed to read public key file: {}",
            config.keys.public_key_path.display()
        )
    })?;
    let public_key =
        PublicMasterKey::from_str(&public_key_pem).context("Failed to parse public key")?;

    let data_key = DataKey::new();
    let encrypted_archive = data_key
        .encrypt(tar_bytes)
        .context("Failed to encrypt archive")?;
    let encrypted_data_key = public_key
        .encrypt(data_key.as_bytes())
        .context("Failed to encrypt archive data key")?;

    Ok(EncryptedExportEnvelopeV1 {
        envelope_version: ENVELOPE_VERSION,
        archive_type: ARCHIVE_TYPE.to_string(),
        archive_cipher: ARCHIVE_CIPHER.to_string(),
        key_cipher: KEY_CIPHER.to_string(),
        encrypted_data_key_b64: BASE64.encode(encrypted_data_key),
        encrypted_archive_b64: BASE64.encode(encrypted_archive),
    })
}

fn decrypt_archive(config: &Config, envelope: &EncryptedExportEnvelopeV1) -> Result<Vec<u8>> {
    validate_envelope(envelope)?;

    let private_key_pem = fs::read_to_string(&config.keys.private_key_path).with_context(|| {
        format!(
            "Failed to read private key file: {}",
            config.keys.private_key_path.display()
        )
    })?;
    let private_key =
        PrivateMasterKey::from_str(&private_key_pem).context("Failed to parse private key")?;

    let encrypted_data_key = BASE64
        .decode(&envelope.encrypted_data_key_b64)
        .context("Invalid archive data key encoding")?;
    let encrypted_archive = BASE64
        .decode(&envelope.encrypted_archive_b64)
        .context("Invalid encrypted archive encoding")?;

    let data_key_bytes = private_key
        .decrypt(&encrypted_data_key)
        .context("Failed to decrypt archive data key")?;
    let data_key =
        DataKey::from_bytes(&data_key_bytes).context("Invalid archive data key length")?;

    data_key
        .decrypt(&encrypted_archive)
        .context("Failed to decrypt archive")
}

fn validate_envelope(envelope: &EncryptedExportEnvelopeV1) -> Result<()> {
    if envelope.envelope_version != ENVELOPE_VERSION {
        anyhow::bail!(
            "Unsupported export envelope version: {}",
            envelope.envelope_version
        );
    }
    if envelope.archive_type != ARCHIVE_TYPE {
        anyhow::bail!("Unsupported archive type: {}", envelope.archive_type);
    }
    if envelope.archive_cipher != ARCHIVE_CIPHER {
        anyhow::bail!("Unsupported archive cipher: {}", envelope.archive_cipher);
    }
    if envelope.key_cipher != KEY_CIPHER {
        anyhow::bail!("Unsupported archive key cipher: {}", envelope.key_cipher);
    }
    Ok(())
}

fn build_archive_tar(
    manifest: &ExportManifestV1,
    records: &[ExportSecretRecordV1],
) -> Result<Vec<u8>> {
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        append_json_file(&mut builder, MANIFEST_PATH, manifest)?;
        append_json_file(&mut builder, SECRETS_PATH, records)?;
        builder.finish().context("Failed to finish tar archive")?;
    }
    Ok(tar_bytes)
}

fn append_json_file<T: Serialize + ?Sized>(
    builder: &mut tar::Builder<&mut Vec<u8>>,
    path: &str,
    value: &T,
) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_cksum();
    builder
        .append_data(&mut header, path, Cursor::new(bytes))
        .with_context(|| format!("Failed to append {path} to tar archive"))?;
    Ok(())
}

fn read_archive_tar(tar_bytes: &[u8]) -> Result<ExportArchiveV1> {
    let mut archive = tar::Archive::new(Cursor::new(tar_bytes));
    let mut manifest_bytes = None;
    let mut secrets_bytes = None;

    for entry in archive.entries().context("Failed to read tar archive")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let path = entry
            .path()
            .context("Failed to read tar entry path")?
            .to_string_lossy()
            .into_owned();

        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .with_context(|| format!("Failed to read tar entry {path}"))?;

        match path.as_str() {
            MANIFEST_PATH => {
                if manifest_bytes.replace(bytes).is_some() {
                    anyhow::bail!("Encrypted archive contains duplicate {MANIFEST_PATH}");
                }
            }
            SECRETS_PATH => {
                if secrets_bytes.replace(bytes).is_some() {
                    anyhow::bail!("Encrypted archive contains duplicate {SECRETS_PATH}");
                }
            }
            _ => anyhow::bail!("Unexpected file in encrypted archive: {path}"),
        }
    }

    parse_archive_files(
        &manifest_bytes.context("Encrypted archive is missing manifest.json")?,
        &secrets_bytes.context("Encrypted archive is missing secrets.json")?,
    )
}

fn parse_archive_files(manifest_bytes: &[u8], secrets_bytes: &[u8]) -> Result<ExportArchiveV1> {
    let manifest_value: Value =
        serde_json::from_slice(manifest_bytes).context("Failed to parse archive manifest")?;
    let format_version = manifest_value
        .get("format_version")
        .and_then(Value::as_u64)
        .context("Archive manifest is missing format_version")?;

    match format_version {
        1 => {
            let manifest: ExportManifestV1 = serde_json::from_value(manifest_value)
                .context("Failed to parse v1 archive manifest")?;
            let records: Vec<ExportSecretRecordV1> =
                serde_json::from_slice(secrets_bytes).context("Failed to parse v1 secrets")?;
            if manifest.secret_count != records.len() {
                anyhow::bail!(
                    "Archive manifest secret_count {} does not match secrets.json count {}",
                    manifest.secret_count,
                    records.len()
                );
            }
            Ok(ExportArchiveV1 { manifest, records })
        }
        other => anyhow::bail!("Unsupported export archive format version: {other}"),
    }
}

fn migrate_archive(archive: ExportArchiveV1) -> Result<Vec<ExportSecretRecordV1>> {
    match archive.manifest.format_version {
        1 => Ok(archive.records),
        other => anyhow::bail!("Unsupported export archive format version: {other}"),
    }
}

fn write_private_file(path: &str, bytes: &[u8]) -> Result<()> {
    let path_ref = Path::new(path);
    if let Some(parent) = path_ref.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create export archive directory: {}",
                parent.display()
            )
        })?;
    }

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(path_ref)
        .with_context(|| format!("Failed to create encrypted archive: {path}"))?;
    file.write_all(bytes)
        .with_context(|| format!("Failed to write encrypted archive: {path}"))?;
    file.sync_all()
        .with_context(|| format!("Failed to sync encrypted archive: {path}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sealbox_server::crypto::master_key::generate_key_pair;
    use tempfile::TempDir;

    fn test_config_with_keys() -> (Config, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let public_key_path = temp_dir.path().join("public.pem");
        let private_key_path = temp_dir.path().join("private.pem");
        let (private_pem, public_pem) = generate_key_pair().unwrap();
        fs::write(&public_key_path, public_pem).unwrap();
        fs::write(&private_key_path, private_pem).unwrap();

        let mut config = Config::default();
        config.server.token = "test-token".to_string();
        config.keys.public_key_path = public_key_path;
        config.keys.private_key_path = private_key_path;
        (config, temp_dir)
    }

    fn test_records() -> Vec<ExportSecretRecordV1> {
        vec![ExportSecretRecordV1 {
            key: "db/password".to_string(),
            value: "secret-value".to_string(),
            version: 3,
            expires_at: Some(4_102_444_800),
            metadata: Some(r#"{"type":"credential","username":"app"}"#.to_string()),
        }]
    }

    #[test]
    fn test_archive_encrypt_decrypt_roundtrip() {
        let (config, _temp_dir) = test_config_with_keys();
        let records = test_records();
        let manifest = ExportManifestV1 {
            format_version: ARCHIVE_FORMAT_VERSION,
            application: "sealbox-cli".to_string(),
            exported_at: 1_700_000_000,
            secret_count: records.len(),
        };

        let tar_bytes = build_archive_tar(&manifest, &records).unwrap();
        let envelope = encrypt_archive(&config, &tar_bytes).unwrap();
        let decrypted_tar = decrypt_archive(&config, &envelope).unwrap();
        let archive = read_archive_tar(&decrypted_tar).unwrap();

        assert_eq!(archive.manifest, manifest);
        assert_eq!(archive.records, records);
    }

    #[test]
    fn test_validate_archive_format_rejects_plaintext_formats() {
        let result = validate_archive_format("json");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported archive format")
        );
    }

    #[test]
    fn test_parse_archive_rejects_unsupported_version() {
        let manifest = json!({
            "format_version": 999,
            "application": "sealbox-cli",
            "exported_at": 1,
            "secret_count": 0
        });
        let records: Vec<ExportSecretRecordV1> = Vec::new();

        let result = parse_archive_files(
            &serde_json::to_vec(&manifest).unwrap(),
            &serde_json::to_vec(&records).unwrap(),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported export archive format version")
        );
    }

    #[test]
    fn test_validate_envelope_rejects_unsupported_version() {
        let envelope = EncryptedExportEnvelopeV1 {
            envelope_version: 999,
            archive_type: ARCHIVE_TYPE.to_string(),
            archive_cipher: ARCHIVE_CIPHER.to_string(),
            key_cipher: KEY_CIPHER.to_string(),
            encrypted_data_key_b64: String::new(),
            encrypted_archive_b64: String::new(),
        };

        let result = validate_envelope(&envelope);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported export envelope version")
        );
    }

    #[test]
    fn test_parse_archive_validates_secret_count() {
        let manifest = ExportManifestV1 {
            format_version: ARCHIVE_FORMAT_VERSION,
            application: "sealbox-cli".to_string(),
            exported_at: 1,
            secret_count: 2,
        };
        let records = test_records();

        let result = parse_archive_files(
            &serde_json::to_vec(&manifest).unwrap(),
            &serde_json::to_vec(&records).unwrap(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("secret_count"));
    }
}
