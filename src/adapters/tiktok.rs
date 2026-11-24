use super::{file_stream_body, qs, send_json, PublishInput, PublishResult};
use crate::db::{ClientCredentials, ConnectedAccount, TokenSet};
use crate::media::build_caption;
use crate::security::AppConfig;
use anyhow::{anyhow, bail, Result};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde_json::{json, Value};

const API: &str = "https://open.tiktokapis.com/v2";
const MB: u64 = 1024 * 1024;

fn parse_tokens(v: &Value) -> Result<TokenSet> {
    let access_token = v
        .get("access_token")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("TikTok token error: {}", v))?
        .to_string();
    let refresh_token = v
        .get("refresh_token")
        .and_then(|x| x.as_str())
        .map(String::from);
    let expires_at = v
        .get("expires_in")
        .and_then(|x| x.as_i64())
        .map(|sec| (Utc::now() + Duration::seconds(sec)).to_rfc3339());
    Ok(TokenSet {
        access_token,
        refresh_token,
        expires_at,
    })
}

pub fn auth_url(creds: &ClientCredentials, state: &str, redirect_uri: &str) -> String {
    let q = qs(&[
        ("client_key", &creds.client_id),
        ("scope", "user.info.basic,video.publish,video.upload"),
        ("response_type", "code"),
        ("redirect_uri", redirect_uri),
        ("state", state),
    ]);
    format!("https://www.tiktok.com/v2/auth/authorize/?{q}")
}

pub async fn connect(
    client: &Client,
    creds: &ClientCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<Vec<ConnectedAccount>> {
    let r = send_json(
        "TikTok token",
        client.post(format!("{API}/oauth/token/")).form(&[
            ("client_key", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ]),
    )
    .await?;

    let tokens = parse_tokens(&r)?;
    let open_id = r
        .get("open_id")
        .and_then(|x| x.as_str())
        .unwrap_or("tiktok_user")
        .to_string();

    let display_name = verify_account(client, &tokens).await.ok();

    Ok(vec![ConnectedAccount {
        platform: "tiktok".to_string(),
        external_id: open_id,
        display_name: Some(display_name.unwrap_or_else(|| "TikTok".to_string())),
        tokens,
    }])
}

pub async fn verify_account(client: &Client, tokens: &TokenSet) -> Result<String> {
    let u = send_json(
        "TikTok user info",
        client
            .get(format!("{API}/user/info/?fields=open_id,display_name"))
            .bearer_auth(&tokens.access_token),
    )
    .await?;
    if let Some(code) = u
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_str())
    {
        if code != "ok" {
            let msg = u
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(code);
            bail!("TikTok error: {msg}");
        }
    }
    Ok(u.get("data")
        .and_then(|d| d.get("user"))
        .and_then(|usr| usr.get("display_name").or_else(|| usr.get("open_id")))
        .and_then(|x| x.as_str())
        .unwrap_or("TikTok")
        .to_string())
}

pub async fn refresh(
    client: &Client,
    tokens: &TokenSet,
    creds: &ClientCredentials,
) -> Result<TokenSet> {
    let rt = tokens
        .refresh_token
        .as_deref()
        .ok_or_else(|| anyhow!("Refresh token TikTok tidak tersedia, hubungkan ulang akun"))?;
    let r = send_json(
        "TikTok refresh",
        client.post(format!("{API}/oauth/token/")).form(&[
            ("client_key", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", rt),
        ]),
    )
    .await?;
    parse_tokens(&r)
}

pub async fn publish(
    client: &Client,
    cfg: &AppConfig,
    tokens: &TokenSet,
    input: &PublishInput,
) -> Result<PublishResult> {
    let size = std::fs::metadata(&input.video_path)?.len();
    let chunk_size = if size <= 64 * MB { size } else { 10 * MB };
    let total_chunks = if size <= 64 * MB { 1 } else { size / chunk_size };

    let full_cap = build_caption(input.caption.as_deref(), &input.hashtags, &[]);
    let title_str: String = if !full_cap.is_empty() {
        full_cap.chars().take(2200).collect()
    } else {
        input.title.clone().unwrap_or_default()
    };

    let init = send_json(
        "TikTok publish init",
        client
            .post(format!("{API}/post/publish/video/init/"))
            .bearer_auth(&tokens.access_token)
            .json(&json!({
                "post_info": {
                    "title": title_str,
                    "privacy_level": cfg.tiktok_privacy,
                    "disable_duet": false,
                    "disable_comment": false,
                    "disable_stitch": false
                },
                "source_info": {
                    "source": "FILE_UPLOAD",
                    "video_size": size,
                    "chunk_size": chunk_size,
                    "total_chunk_count": total_chunks
                }
            })),
    )
    .await?;

    if let Some(code) = init
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_str())
    {
        if code != "ok" {
            bail!("TikTok init gagal: {}", init.get("error").unwrap_or(&Value::Null));
        }
    }

    let data = init
        .get("data")
        .ok_or_else(|| anyhow!("TikTok init tidak mengembalikan data"))?;
    let upload_url = data
        .get("upload_url")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("TikTok upload_url kosong"))?;
    let publish_id = data
        .get("publish_id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("TikTok publish_id kosong"))?
        .to_string();

    for i in 0..total_chunks {
        let start = i * chunk_size;
        let end = if i == total_chunks - 1 {
            size - 1
        } else {
            start + chunk_size - 1
        };
        let len = end - start + 1;
        let body = file_stream_body(&input.video_path, start, len).await?;
        send_json(
            "TikTok chunk upload",
            client
                .put(upload_url)
                .header("Content-Type", "video/mp4")
                .header("Content-Length", len.to_string())
                .header("Content-Range", format!("bytes {start}-{end}/{size}"))
                .body(body),
        )
        .await?;
    }

    for _ in 0..60 {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let s = send_json(
            "TikTok status",
            client
                .post(format!("{API}/post/publish/status/fetch/"))
                .bearer_auth(&tokens.access_token)
                .json(&json!({ "publish_id": publish_id })),
        )
        .await?;
        let st = s
            .get("data")
            .and_then(|d| d.get("status"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        if st == "PUBLISH_COMPLETE" {
            let post_id = s
                .get("data")
                .and_then(|d| d.get("publicaly_available_post_id"))
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .map(|v| v.to_string().trim_matches('"').to_string())
                .unwrap_or_else(|| publish_id.clone());
            return Ok(PublishResult {
                platform_post_id: post_id,
                url: None,
            });
        }
        if st == "FAILED" {
            let reason = s
                .get("data")
                .and_then(|d| d.get("fail_reason"))
                .and_then(|x| x.as_str())
                .unwrap_or("unknown");
            bail!("TikTok gagal memproses video: {reason}");
        }
    }

    Ok(PublishResult {
        platform_post_id: publish_id,
        url: None,
    })
}
