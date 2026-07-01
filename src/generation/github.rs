use anyhow::Result;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, common::Generate}};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

pub struct GitHubClient {
    repo: String,
    #[allow(dead_code)]
    token: Option<String>,
    client: Client,
}

#[derive(Deserialize)]
struct GhTag {
    name: String,
}

#[derive(Deserialize)]
struct GhRelease {
    id: u64,
    body: Option<String>,
}

impl GitHubClient {
    pub fn new(repo: &str, token: Option<&str>) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
        if let Some(t) = token {
            headers.insert("Authorization", format!("Bearer {t}").parse().unwrap());
        }
        let client = Client::builder()
            .default_headers(headers)
            .user_agent("vessel/0.1.0")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Self { repo: repo.into(), token: token.map(Into::into), client }
    }

    pub async fn list_tags(&self) -> Result<Vec<String>> {
        let url = format!("https://api.github.com/repos/{}/tags?per_page=30", self.repo);
        let tags: Vec<GhTag> = self.client.get(&url).send().await?
            .error_for_status()?
            .json().await?;
        Ok(tags.into_iter().map(|t| t.name).collect())
    }

    pub async fn get_release_body(&self, tag: &str) -> Result<Option<String>> {
        let url = format!("https://api.github.com/repos/{}/releases/tags/{}", self.repo, tag);
        let resp = self.client.get(&url).send().await?;
        if resp.status() == 404 {
            return Ok(None);
        }
        let release: GhRelease = resp.error_for_status()?.json().await?;
        Ok(release.body)
    }

    pub async fn patch_release_body(&self, tag: &str, body: &str) -> Result<()> {
        let url = format!("https://api.github.com/repos/{}/releases/tags/{}", self.repo, tag);
        let release: GhRelease = self.client.get(&url).send().await?
            .error_for_status()?.json().await?;

        let patch_url = format!("https://api.github.com/repos/{}/releases/{}", self.repo, release.id);
        #[derive(Serialize)]
        struct Patch<'a> { body: &'a str }
        self.client.patch(&patch_url)
            .json(&Patch { body })
            .send().await?
            .error_for_status()?;
        Ok(())
    }
}

/// Derive a machine-specific 32-byte key from the home directory path.
/// This is stable per machine but not cryptographically secret — it is
/// intended for local storage only.
pub fn derive_encryption_key() -> [u8; 32] {
    let home = dirs::home_dir().unwrap_or_default();
    let seed = home.to_string_lossy();
    let mut key = [0u8; 32];
    let seed_bytes = seed.as_bytes();
    for (i, b) in seed_bytes.iter().enumerate() {
        key[i % 32] ^= b;
    }
    let salt = b"vessel-token-key-v1";
    for (i, b) in salt.iter().enumerate() {
        key[i % 32] ^= b;
    }
    key
}

/// Encrypt a token with AES-256-GCM using a random nonce.
/// Returns `(ciphertext_b64, nonce_b64)`.
pub fn encrypt_token(token: &str, key: &[u8; 32]) -> Result<(String, String)> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = aes_gcm::Nonce::generate();
    let ciphertext = cipher.encrypt(&nonce, token.as_bytes())
        .map_err(|e| anyhow::anyhow!("encrypt error: {e}"))?;
    Ok((B64.encode(&ciphertext), B64.encode(&nonce)))
}

/// Decrypt a token that was encrypted with `encrypt_token`.
pub fn decrypt_token(ciphertext_b64: &str, nonce_b64: &str, key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new(key.into());
    let ciphertext = B64.decode(ciphertext_b64)?;
    let nonce_bytes = B64.decode(nonce_b64)?;
    let nonce = aes_gcm::Nonce::try_from(nonce_bytes.as_slice())?;
    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("decrypt error: {e}"))?;
    Ok(String::from_utf8(plaintext)?)
}
