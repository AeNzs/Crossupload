use super::{file_stream_body, qs, send_json, PostMetrics, PublishInput, PublishResult};
use crate::db::{ClientCredentials, ConnectedAccount, TokenSet};
use crate::media::build_caption;
use anyhow::{anyhow, bail, Result};
use reqwest::Client;
use std::collections::HashMap;

const V: &str = "v21.0";

fn graph(path: &str) -> String {
    format!("https://graph.facebook.com/{V}/{path}")
}

pub fn auth_url(creds: &ClientCredentials, state: &str, redirect_uri: &str) -> String {
    let q = qs(&[
        ("client_id", &creds.client_id),
        ("redirect_uri", redirect_uri),
        ("state", state),
        ("auth_type", "rerequest"),
        (
            "scope",
            "pages_show_list,pages_read_engagement,pages_manage_posts,instagram_basic,instagram_content_publish,business_management",
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
                "me/accounts?fields=id,name,access_token,instagram_business_account{id,username},connected_instagram_account{id,username}&limit=100",
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

        let mut ig_obj = p
            .get("instagram_business_account")
            .or_else(|| p.get("connected_instagram_account"))
            .cloned();

        // Fallback query langsung ke node Page memakai Page Access Token.
        if ig_obj.is_none() {
            if let Ok(page_detail) = send_json(
                "Meta page IG lookup",
                client
                    .get(graph(&format!(
                        "{page_id}?fields=instagram_business_account{{id,username}},connected_instagram_account{{id,username}}"
                    )))
                    .bearer_auth(page_tok),
            )
            .await
            {
                ig_obj = page_detail
                    .get("instagram_business_account")
                    .or_else(|| page_detail.get("connected_instagram_account"))
                    .cloned();
            }
        }

        if let Some(ig) = ig_obj {
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

    let video_state = if matches!(input.visibility.as_deref(), Some(v) if v.eq_ignore_ascii_case("DRAFT")) {
        "DRAFT"
    } else {
        "PUBLISHED"
    };

    let mut form_fields = vec![
        ("upload_phase", "finish".to_string()),
        ("video_id", video_id.clone()),
        ("video_state", video_state.to_string()),
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

    if let Some(ref thumb_path) = input.custom_thumb_path {
        if let Ok(bytes) = tokio::fs::read(thumb_path).await {
            let boundary = format!("----CrossUploadBoundary{}", uuid::Uuid::new_v4().simple());
            let mut mp = Vec::with_capacity(bytes.len() + 512);
            mp.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"is_preferred\"\r\n\r\ntrue\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"access_token\"\r\n\r\n{}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"source\"; filename=\"thumb.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n",
                    tokens.access_token
                )
                .as_bytes(),
            );
            mp.extend_from_slice(&bytes);
            mp.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            let thumb_res = send_json(
                "Facebook thumbnail",
                client
                    .post(graph(&format!("{video_id}/thumbnails")))
                    .header(
                        "Content-Type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(mp),
            )
            .await;
            if let Err(e) = thumb_res {
                eprintln!("[facebook] reel {video_id} tersimpan, tetapi custom thumbnail tidak diterapkan: {e}");
            }
        }
    }

    Ok(PublishResult {
        url: if video_state == "DRAFT" {
            None
        } else {
            Some(format!("https://www.facebook.com/reel/{video_id}"))
        },
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
    let share_to_feed =
        if matches!(input.visibility.as_deref(), Some(v) if v.eq_ignore_ascii_case("reels_only")) {
            "false"
        } else {
            "true"
        };

    let mut form = vec![
        ("media_type", "REELS".to_string()),
        ("upload_type", "resumable".to_string()),
        ("caption", caption.clone()),
        ("share_to_feed", share_to_feed.to_string()),
        ("access_token", tokens.access_token.clone()),
    ];
    let has_cover = input.custom_thumb_url.is_some();
    if let Some(ref cover_url) = input.custom_thumb_url {
        form.push(("cover_url", cover_url.clone()));
    }

    let container = match send_json(
        "Instagram container",
        client
            .post(graph(&format!("{external_id}/media")))
            .form(&form),
    )
    .await
    {
        Ok(v) => v,
        Err(e) if has_cover => {
            eprintln!("[instagram] container dengan cover_url gagal ({e}), mengulang tanpa cover_url");
            form.retain(|(k, _)| *k != "cover_url");
            send_json(
                "Instagram container",
                client
                    .post(graph(&format!("{external_id}/media")))
                    .form(&form),
            )
            .await?
        }
        Err(e) => return Err(e),
    };

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

fn first_insight_val(val: &serde_json::Value, names: &[&str]) -> Option<i64> {
    let data = val.get("data")?.as_array()?;
    for name in names {
        for item in data {
            if item.get("name").and_then(|x| x.as_str()) == Some(*name) {
                if let Some(v) = item
                    .get("values")
                    .and_then(|a| a.as_array())
                    .and_then(|a| a.first())
                    .and_then(|o| o.get("value"))
                    .and_then(|n| n.as_i64())
                {
                    return Some(v);
                }
            }
        }
    }
    None
}

pub async fn fetch_facebook_metrics_batch(
    client: &Client,
    tokens: &TokenSet,
    post_ids: &[String],
) -> Result<HashMap<String, PostMetrics>> {
    let mut out = HashMap::new();
    for id in post_ids {
        let info = send_json(
            "Facebook video metrics",
            client
                .get(graph(&format!(
                    "{id}?fields=id,views,likes.summary(true),comments.summary(true)"
                )))
                .bearer_auth(&tokens.access_token),
        )
        .await;

        match info {
            Ok(v) => {
                let mut views = v.get("views").and_then(|x| x.as_i64());
                let likes = v
                    .get("likes")
                    .and_then(|x| x.get("summary"))
                    .and_then(|s| s.get("total_count"))
                    .and_then(|n| n.as_i64());
                let comments = v
                    .get("comments")
                    .and_then(|x| x.get("summary"))
                    .and_then(|s| s.get("total_count"))
                    .and_then(|n| n.as_i64());

                if views.is_none() {
                    if let Ok(ins) = send_json(
                        "Facebook video insights",
                        client
                            .get(graph(&format!(
                                "{id}/video_insights?metric=total_video_views,fb_reels_total_plays"
                            )))
                            .bearer_auth(&tokens.access_token),
                    )
                    .await
                    {
                        views = first_insight_val(
                            &ins,
                            &["fb_reels_total_plays", "total_video_views"],
                        );
                    }
                }

                out.insert(
                    id.clone(),
                    PostMetrics {
                        views,
                        likes,
                        comments,
                        shares: None,
                        note: None,
                    },
                );
            }
            Err(e) => {
                out.insert(
                    id.clone(),
                    PostMetrics {
                        note: Some(e.to_string()),
                        ..Default::default()
                    },
                );
            }
        }
    }
    Ok(out)
}

pub async fn fetch_instagram_metrics_batch(
    client: &Client,
    tokens: &TokenSet,
    post_ids: &[String],
) -> Result<HashMap<String, PostMetrics>> {
    let mut out = HashMap::new();
    for id in post_ids {
        let base = send_json(
            "Instagram media metrics",
            client
                .get(graph(&format!("{id}?fields=id,like_count,comments_count")))
                .bearer_auth(&tokens.access_token),
        )
        .await;

        match base {
            Ok(v) => {
                let likes = v.get("like_count").and_then(|x| x.as_i64());
                let comments = v.get("comments_count").and_then(|x| x.as_i64());
                let mut views = None;
                let mut shares = None;
                let mut note = None;

                let ins = send_json(
                    "Instagram insights",
                    client
                        .get(graph(&format!("{id}/insights?metric=views,shares")))
                        .bearer_auth(&tokens.access_token),
                )
                .await;

                let ins = match ins {
                    Ok(val) => Ok(val),
                    Err(_) => {
                        send_json(
                            "Instagram insights plays",
                            client
                                .get(graph(&format!("{id}/insights?metric=plays,shares")))
                                .bearer_auth(&tokens.access_token),
                        )
                        .await
                    }
                };

                match ins {
                    Ok(val) => {
                        views = first_insight_val(&val, &["views", "plays"]);
                        shares = first_insight_val(&val, &["shares"]);
                    }
                    Err(_) => {
                        note = Some(
                            "Tayangan IG membutuhkan izin instagram_manage_insights; suka dan komentar tetap diperbarui."
                                .to_string(),
                        );
                    }
                }

                out.insert(
                    id.clone(),
                    PostMetrics {
                        views,
                        likes,
                        comments,
                        shares,
                        note,
                    },
                );
            }
            Err(e) => {
                out.insert(
                    id.clone(),
                    PostMetrics {
                        note: Some(e.to_string()),
                        ..Default::default()
                    },
                );
            }
        }
    }
    Ok(out)
}
