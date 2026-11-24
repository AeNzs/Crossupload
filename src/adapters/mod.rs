pub mod meta;
pub mod tiktok;
pub mod youtube;

use crate::db::{ClientCredentials, ConnectedAccount, TokenSet};
use crate::security::AppConfig;
use anyhow::{anyhow, bail, Result};
use reqwest::{Body, Client, RequestBuilder};
use serde_json::Value;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

pub struct PublishInput {
    pub video_path: PathBuf,
    pub title: Option<String>,
    pub caption: Option<String>,
    pub hashtags: Vec<String>,
}

pub struct PublishResult {
    pub platform_post_id: String,
    pub url: Option<String>,
}

pub fn build_http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(1800))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(Into::into)
}

pub fn qs(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

pub async fn send_json(label: &str, rb: RequestBuilder) -> Result<Value> {
    let res = rb
        .send()
        .await
        .map_err(|e| anyhow!("{label}: gagal menghubungi server ({e})"))?;
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        let preview: String = text.chars().take(500).collect();
        bail!("{label} -> {}: {preview}", status.as_u16());
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

pub async fn file_stream_body(path: &Path, start: u64, len: u64) -> Result<Body> {
    let mut f = File::open(path).await?;
    if start > 0 {
        f.seek(SeekFrom::Start(start)).await?;
    }
    let slice = f.take(len);
    Ok(Body::wrap_stream(ReaderStream::new(slice)))
}

pub fn get_auth_url(
    provider: &str,
    creds: &ClientCredentials,
    state: &str,
    redirect_uri: &str,
) -> Result<String> {
    match provider {
        "youtube" => Ok(youtube::auth_url(creds, state, redirect_uri)),
        "tiktok" => Ok(tiktok::auth_url(creds, state, redirect_uri)),
        "meta" => Ok(meta::auth_url(creds, state, redirect_uri)),
        _ => bail!("Provider {provider} tidak dikenal"),
    }
}

pub async fn connect_provider(
    client: &Client,
    provider: &str,
    creds: &ClientCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<Vec<ConnectedAccount>> {
    match provider {
        "youtube" => youtube::connect(client, creds, code, redirect_uri).await,
        "tiktok" => tiktok::connect(client, creds, code, redirect_uri).await,
        "meta" => meta::connect(client, creds, code, redirect_uri).await,
        _ => bail!("Provider {provider} tidak dikenal"),
    }
}

pub async fn verify_account(
    client: &Client,
    provider: &str,
    tokens: &TokenSet,
    external_id: &str,
) -> Result<String> {
    match provider {
        "youtube" => youtube::verify_account(client, tokens).await,
        "tiktok" => tiktok::verify_account(client, tokens).await,
        "meta" => meta::verify_account(client, tokens, external_id).await,
        _ => bail!("Provider {provider} tidak dikenal"),
    }
}

pub fn supports_refresh(platform: &str) -> bool {
    matches!(platform, "youtube" | "tiktok")
}

pub async fn refresh_tokens(
    client: &Client,
    platform: &str,
    tokens: &TokenSet,
    creds: &ClientCredentials,
) -> Result<TokenSet> {
    match platform {
        "youtube" => youtube::refresh(client, tokens, creds).await,
        "tiktok" => tiktok::refresh(client, tokens, creds).await,
        _ => Ok(tokens.clone()),
    }
}

pub async fn publish(
    client: &Client,
    cfg: &AppConfig,
    platform: &str,
    tokens: &TokenSet,
    external_id: &str,
    input: &PublishInput,
) -> Result<PublishResult> {
    match platform {
        "youtube" => youtube::publish(client, cfg, tokens, input).await,
        "tiktok" => tiktok::publish(client, cfg, tokens, input).await,
        "facebook" => meta::publish_facebook(client, tokens, external_id, input).await,
        "instagram" => meta::publish_instagram(client, tokens, external_id, input).await,
        _ => bail!("Platform {platform} tidak dikenal"),
    }
}
