use super::{file_stream_body, qs, send_json, PublishInput, PublishResult};
use crate::db::{ClientCredentials, ConnectedAccount, TokenSet};
use crate::media::build_caption;
use crate::security::AppConfig;
use anyhow::{anyhow, bail, Result};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde_json::{json, Value};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

fn parse_tokens(v: &Value, prev_refresh: Option<&str>) -> Result<TokenSet> {
    let access_token = v
        .get("access_token")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("Google tidak mengembalikan access_token"))?
        .to_string();
    let refresh_token = v
        .get("refresh_token")
        .and_then(|x| x.as_str())
        .map(String::from)
        .or_else(|| prev_refresh.map(String::from));
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
        ("client_id", &creds.client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        (
            "scope",
            "https://www.googleapis.com/auth/youtube.upload https://www.googleapis.com/auth/youtube.readonly",
        ),
        ("access_type", "offline"),
        ("prompt", "consent"),
        ("state", state),
    ]);
    format!("{AUTH_URL}?{q}")
}

pub async fn connect(
    client: &Client,
    creds: &ClientCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<Vec<ConnectedAccount>> {
    let tok = send_json(
        "Google token",
        client.post(TOKEN_URL).form(&[
            ("code", code),
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ]),
    )
    .await?;

    let tokens = parse_tokens(&tok, None)?;
    let ch = send_json(
        "YouTube channels",
        client
            .get("https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true")
            .bearer_auth(&tokens.access_token),
    )
    .await?;

    let item = ch
        .get("items")
        .and_then(|i| i.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| anyhow!("Akun Google ini tidak memiliki channel YouTube"))?;

    let ext_id = item
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("ID channel YouTube tidak ditemukan"))?
        .to_string();
    let title = item
        .get("snippet")
        .and_then(|s| s.get("title"))
        .and_then(|t| t.as_str())
        .map(String::from);

    Ok(vec![ConnectedAccount {
        platform: "youtube".to_string(),
        external_id: ext_id,
        display_name: title,
        tokens,
    }])
}

pub async fn verify_account(client: &Client, tokens: &TokenSet) -> Result<String> {
    let ch = send_json(
        "YouTube verify",
        client
            .get("https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true")
            .bearer_auth(&tokens.access_token),
    )
    .await?;
    let item = ch
        .get("items")
        .and_then(|i| i.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| anyhow!("Channel YouTube tidak ditemukan"))?;
    Ok(item
        .get("snippet")
        .and_then(|s| s.get("title"))
        .and_then(|t| t.as_str())
        .or_else(|| item.get("id").and_then(|x| x.as_str()))
        .unwrap_or("YouTube")
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
        .ok_or_else(|| anyhow!("Refresh token YouTube tidak tersedia, hubungkan ulang akun"))?;
    let r = send_json(
        "Google refresh",
        client.post(TOKEN_URL).form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("refresh_token", rt),
            ("grant_type", "refresh_token"),
        ]),
    )
    .await?;
    parse_tokens(&r, Some(rt))
}

pub async fn publish(
    client: &Client,
    cfg: &AppConfig,
    tokens: &TokenSet,
    input: &PublishInput,
) -> Result<PublishResult> {
    let size = std::fs::metadata(&input.video_path)?.len();
    let raw_title = input
        .title
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| input.caption.as_deref().filter(|s| !s.is_empty()))
        .unwrap_or("Video");
    let title: String = raw_title.chars().take(100).collect();
    let desc: String = build_caption(input.caption.as_deref(), &input.hashtags, &["Shorts"])
        .chars()
        .take(4900)
        .collect();

    let init_res = client
        .post("https://www.googleapis.com/upload/youtube/v3/videos?uploadType=resumable&part=snippet,status")
        .bearer_auth(&tokens.access_token)
        .header("Content-Type", "application/json; charset=UTF-8")
        .header("X-Upload-Content-Length", size.to_string())
        .header("X-Upload-Content-Type", "video/mp4")
        .json(&json!({
            "snippet": { "title": title, "description": desc },
            "status": { "privacyStatus": cfg.youtube_privacy, "selfDeclaredMadeForKids": false }
        }))
        .send()
        .await?;

    if !init_res.status().is_success() {
        let code = init_res.status().as_u16();
        let body = init_res.text().await.unwrap_or_default();
        bail!(
            "YouTube init -> {code}: {}",
            body.chars().take(400).collect::<String>()
        );
    }

    let upload_url = init_res
        .headers()
        .get("location")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| anyhow!("YouTube tidak mengembalikan header Location upload"))?
        .to_string();

    let body = file_stream_body(&input.video_path, 0, size).await?;
    let done = send_json(
        "YouTube upload",
        client
            .put(&upload_url)
            .header("Content-Type", "video/mp4")
            .header("Content-Length", size.to_string())
            .body(body),
    )
    .await?;

    let vid_id = done
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("YouTube tidak mengembalikan video id"))?
        .to_string();

    Ok(PublishResult {
        url: Some(format!("https://www.youtube.com/watch?v={vid_id}")),
        platform_post_id: vid_id,
    })
}
