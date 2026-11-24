use super::{file_stream_body, qs, send_json, PublishInput, PublishResult};
use crate::db::{ClientCredentials, ConnectedAccount, TokenSet};
use crate::media::build_caption;
use anyhow::{anyhow, bail, Result};
use reqwest::Client;

const V: &str = "v21.0";

fn graph(path: &str) -> String {
    format!("https://graph.facebook.com/{V}/{path}")
}

pub fn auth_url(creds: &ClientCredentials, state: &str, redirect_uri: &str) -> String {
    let q = qs(&[
        ("client_id", &creds.client_id),
        ("redirect_uri", redirect_uri),
        ("state", state),
        (
            "scope",
            "pages_show_list,pages_read_engagement,pages_manage_posts,publish_video,instagram_basic,instagram_content_publish,business_management",
        ),
    ]);
    format!("https://www.facebook.com/{V}/dialog/oauth?{q}")
}

pub async fn verify_app_credentials(client: &Client, creds: &ClientCredentials) -> Result<()> {
    send_json(
        "Meta app credentials",
        client.post(graph("oauth/access_token")).form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("grant_type", "client_credentials"),
        ]),
    )
    .await?;
    Ok(())
}

pub async fn connect(
    client: &Client,
    creds: &ClientCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<Vec<ConnectedAccount>> {
    let short = send_json(
        "Meta short token",
        client.post(graph("oauth/access_token")).form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("code", code),
        ]),
    )
    .await?;
    let short_tok = short
        .get("access_token")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("Meta tidak mengembalikan access_token"))?;

    let long = send_json(
        "Meta long token",
        client.post(graph("oauth/access_token")).form(&[
            ("grant_type", "fb_exchange_token"),
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("fb_exchange_token", short_tok),
        ]),
    )
    .await?;
    let long_tok = long
        .get("access_token")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("Meta gagal menukar long-lived token"))?;

    let pages = send_json(
        "Meta pages",
        client
            .get(graph(
                "me/accounts?fields=id,name,access_token,instagram_business_account{id,username}&limit=100",
            ))
            .bearer_auth(long_tok),
    )
    .await?;

    let mut out = Vec::new();
    for p in pages
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default()
    {
        let Some(page_id) = p.get("id").and_then(|x| x.as_str()) else {
            continue;
        };
        let Some(page_tok) = p.get("access_token").and_then(|x| x.as_str()) else {
            continue;
        };
        let page_name = p.get("name").and_then(|x| x.as_str()).map(String::from);
        let tokens = TokenSet {
            access_token: page_tok.to_string(),
            refresh_token: None,
            expires_at: None,
        };

        out.push(ConnectedAccount {
            platform: "facebook".to_string(),
            external_id: page_id.to_string(),
            display_name: page_name.clone(),
            tokens: tokens.clone(),
        });

        if let Some(ig) = p.get("instagram_business_account") {
            if let Some(ig_id) = ig.get("id").and_then(|x| x.as_str()) {
                let ig_name = ig
                    .get("username")
                    .and_then(|x| x.as_str())
                    .map(String::from)
                    .or(page_name);
                out.push(ConnectedAccount {
                    platform: "instagram".to_string(),
                    external_id: ig_id.to_string(),
                    display_name: ig_name,
                    tokens,
                });
            }
        }
    }

    if out.is_empty() {
        bail!("Tidak ditemukan Facebook Page pada akun ini. Buat Page terlebih dahulu.");
    }
    Ok(out)
}

pub async fn verify_account(
    client: &Client,
    tokens: &TokenSet,
    external_id: &str,
) -> Result<String> {
    let info = send_json(
        "Meta verify",
        client
            .get(graph(&format!("{external_id}?fields=id,name,username")))
            .bearer_auth(&tokens.access_token),
    )
    .await?;
    Ok(info
        .get("username")
        .or_else(|| info.get("name"))
        .or_else(|| info.get("id"))
        .and_then(|x| x.as_str())
        .unwrap_or(external_id)
        .to_string())
}

pub async fn publish_facebook(
    client: &Client,
    tokens: &TokenSet,
    external_id: &str,
    input: &PublishInput,
) -> Result<PublishResult> {
    let size = std::fs::metadata(&input.video_path)?.len();
    let start = send_json(
        "Facebook Reels start",
        client
            .post(graph(&format!("{external_id}/video_reels")))
            .form(&[
                ("upload_phase", "start"),
                ("access_token", tokens.access_token.as_str()),
            ]),
    )
    .await?;

    let upload_url = start
        .get("upload_url")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("Facebook tidak mengembalikan upload_url"))?;
    let video_id = start
        .get("video_id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("Facebook tidak mengembalikan video_id"))?
        .to_string();

    let body = file_stream_body(&input.video_path, 0, size).await?;
    send_json(
        "Facebook Reels upload",
        client
            .post(upload_url)
            .header("Authorization", format!("OAuth {}", tokens.access_token))
            .header("offset", "0")
            .header("file_size", size.to_string())
            .body(body),
    )
    .await?;

    let desc: String = build_caption(input.caption.as_deref(), &input.hashtags, &[])
        .chars()
        .take(2000)
        .collect();

    let mut form_fields = vec![
        ("upload_phase", "finish".to_string()),
        ("video_id", video_id.clone()),
        ("video_state", "PUBLISHED".to_string()),
        ("description", desc),
        ("access_token", tokens.access_token.clone()),
    ];
    if let Some(ref t) = input.title {
        if !t.trim().is_empty() {
            form_fields.push(("title", t.clone()));
        }
    }

    send_json(
        "Facebook Reels finish",
        client
            .post(graph(&format!("{external_id}/video_reels")))
            .form(&form_fields),
    )
    .await?;

    Ok(PublishResult {
        url: Some(format!("https://www.facebook.com/reel/{video_id}")),
        platform_post_id: video_id,
    })
}

pub async fn publish_instagram(
    client: &Client,
    tokens: &TokenSet,
    external_id: &str,
    input: &PublishInput,
) -> Result<PublishResult> {
    let size = std::fs::metadata(&input.video_path)?.len();
    let caption: String = build_caption(input.caption.as_deref(), &input.hashtags, &[])
        .chars()
        .take(2200)
        .collect();

    let container = send_json(
        "Instagram container",
        client.post(graph(&format!("{external_id}/media"))).form(&[
            ("media_type", "REELS"),
            ("upload_type", "resumable"),
            ("caption", caption.as_str()),
            ("access_token", tokens.access_token.as_str()),
        ]),
    )
    .await?;

    let container_id = container
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("Instagram tidak mengembalikan container id"))?
        .to_string();

    let body = file_stream_body(&input.video_path, 0, size).await?;
    send_json(
        "Instagram resumable upload",
        client
            .post(format!(
                "https://rupload.facebook.com/ig-api-upload/{V}/{container_id}"
            ))
            .header("Authorization", format!("OAuth {}", tokens.access_token))
            .header("offset", "0")
            .header("file_size", size.to_string())
            .body(body),
    )
    .await?;

    let mut ready = false;
    for _ in 0..60 {
        let s = send_json(
            "Instagram container status",
            client
                .get(graph(&format!("{container_id}?fields=status_code,status")))
                .bearer_auth(&tokens.access_token),
        )
        .await?;
        let code = s.get("status_code").and_then(|x| x.as_str()).unwrap_or("");
        if code == "FINISHED" {
            ready = true;
            break;
        }
        if code == "ERROR" || code == "EXPIRED" {
            let detail = s.get("status").and_then(|x| x.as_str()).unwrap_or(code);
            bail!("Instagram gagal memproses video: {detail}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    if !ready {
        bail!("Instagram timeout menunggu pemrosesan video");
    }

    let pub_res = send_json(
        "Instagram publish",
        client
            .post(graph(&format!("{external_id}/media_publish")))
            .form(&[
                ("creation_id", container_id.as_str()),
                ("access_token", tokens.access_token.as_str()),
            ]),
    )
    .await?;

    let media_id = pub_res
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or(&container_id)
        .to_string();

    let permalink = send_json(
        "Instagram permalink",
        client
            .get(graph(&format!("{media_id}?fields=permalink")))
            .bearer_auth(&tokens.access_token),
    )
    .await
    .ok()
    .and_then(|v| v.get("permalink").and_then(|p| p.as_str()).map(String::from));

    Ok(PublishResult {
        platform_post_id: media_id,
        url: permalink,
    })
}
