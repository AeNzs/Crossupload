use crate::adapters::{self, PublishInput};
use crate::db::{ClaimedJob, Database, PublishedJobRef, TokenSet};
use crate::media::{provider_for_platform, transcode, validate_for_platform};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration as StdDuration;

const MAX_ATTEMPTS: i64 = 3;

enum JobError {
    Permanent(String),
    Transient(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    pub ok: bool,
    pub message: String,
    pub accounts_tested: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsRefreshResult {
    pub ok: bool,
    pub total: usize,
    pub updated: usize,
    pub failed: usize,
    pub message: String,
}

pub fn spawn_worker(db: Database, client: Client) {
    if let Err(e) = db.recover_stuck_jobs() {
        eprintln!("[worker] gagal memulihkan job menggantung: {e}");
    }
    tokio::spawn(async move {
        println!("[worker] aktif, memeriksa antrean setiap 5 detik");
        loop {
            loop {
                match process_next_job(&db, &client).await {
                    Ok(true) => continue,
                    Ok(false) => break,
                    Err(e) => {
                        eprintln!("[worker] error loop: {e}");
                        break;
                    }
                }
            }
            tokio::time::sleep(StdDuration::from_secs(5)).await;
        }
    });
}

pub async fn process_next_job(db: &Database, client: &Client) -> anyhow::Result<bool> {
    let Some(job) = db.claim_next_job()? else {
        return Ok(false);
    };
    run_job(db, client, job).await;
    Ok(true)
}

fn needs_refresh(expires_at: Option<&str>) -> bool {
    let Some(s) = expires_at else { return false };
    let Ok(dt) = DateTime::parse_from_rfc3339(s) else {
        return false;
    };
    dt.with_timezone(&Utc) <= Utc::now() + Duration::seconds(120)
}

async fn execute_job(db: &Database, client: &Client, job: &ClaimedJob) -> Result<(), JobError> {
    let video = db
        .get_video(None, &job.video_id)
        .map_err(|e| JobError::Transient(e.to_string()))?
        .ok_or_else(|| JobError::Permanent("Video sumber sudah dihapus".to_string()))?;

    let account = db
        .get_account_by_id(&job.account_id)
        .map_err(|e| JobError::Transient(e.to_string()))?
        .ok_or_else(|| JobError::Permanent("Akun tujuan sudah diputuskan".to_string()))?;

    let access_token = db
        .cfg
        .decrypt(&account.access_token_enc)
        .map_err(|e| JobError::Permanent(format!("Gagal mendekripsi token akun: {e}")))?;
    let refresh_token = match &account.refresh_token_enc {
        Some(enc) => Some(
            db.cfg
                .decrypt(enc)
                .map_err(|e| JobError::Permanent(format!("Gagal mendekripsi refresh token: {e}")))?,
        ),
        None => None,
    };

    let mut tokens = TokenSet {
        access_token,
        refresh_token,
        expires_at: account.expires_at.clone(),
    };

    if adapters::supports_refresh(&job.platform) && needs_refresh(tokens.expires_at.as_deref()) {
        let prov = provider_for_platform(&job.platform);
        let creds = db
            .resolve_credentials(&job.user_id, prov)
            .map_err(|e| JobError::Permanent(e.to_string()))?
            .ok_or_else(|| JobError::Permanent(format!("Kredensial {prov} belum disimpan")))?;
        println!("[worker] [{}] refresh token {}", job.job_id, job.platform);
        tokens = adapters::refresh_tokens(client, &job.platform, &tokens, &creds)
            .await
            .map_err(|e| JobError::Transient(e.to_string()))?;
        let _ = db.update_account_tokens(&account.id, &tokens);
    }

    let src_path = db.cfg.data_dir.join("uploads").join(&video.storage_key);
    let tmp_path = db
        .cfg
        .data_dir
        .join("tmp")
        .join(format!("{}.mp4", job.job_id));

    println!("[worker] [{}] transcode {}", job.job_id, video.name);
    let meta = transcode(&db.cfg.ffmpeg_bin, &db.cfg.ffprobe_bin, &src_path, &tmp_path)
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            JobError::Permanent(e.to_string())
        })?;

    if let Some(bad) = validate_for_platform(&job.platform, &meta) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(JobError::Permanent(bad));
    }

    let (custom_thumb_path, custom_thumb_url) = if video.thumb_custom {
        if let Some(ref tk) = video.thumb_key {
            let p = db.cfg.data_dir.join("thumbs").join(tk);
            if p.exists() {
                let url = if db.cfg.app_url.starts_with("https://") {
                    let tok = db.cfg.create_thumb_token(tk);
                    Some(format!("{}/api/public/thumbs/{tok}", db.cfg.app_url))
                } else {
                    None
                };
                (Some(p), url)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    println!("[worker] [{}] publish -> {}", job.job_id, job.platform);
    let input = PublishInput {
        video_path: tmp_path.clone(),
        title: job.title.clone(),
        caption: job.caption.clone(),
        hashtags: job.hashtags.clone(),
        visibility: job.visibility.clone(),
        custom_thumb_path,
        custom_thumb_url,
    };

    let res = adapters::publish(
        client,
        &db.cfg,
        &job.platform,
        &tokens,
        &account.external_id,
        &input,
    )
    .await;
    let _ = std::fs::remove_file(&tmp_path);

    let published = res.map_err(|e| JobError::Transient(e.to_string()))?;
    db.mark_job_published(
        &job.job_id,
        &published.platform_post_id,
        published.url.as_deref(),
    )
    .map_err(|e| JobError::Transient(e.to_string()))?;

    println!(
        "[worker] [{}] terbit -> {}",
        job.job_id,
        published
            .url
            .as_deref()
            .unwrap_or(&published.platform_post_id)
    );
    Ok(())
}

async fn run_job(db: &Database, client: &Client, job: ClaimedJob) {
    if let Err(err) = execute_job(db, client, &job).await {
        let attempts = job.attempts + 1;
        let (msg, permanent) = match err {
            JobError::Permanent(m) => (m, true),
            JobError::Transient(m) => (m, false),
        };
        let retry = !permanent && attempts < MAX_ATTEMPTS;
        eprintln!(
            "[worker] [{}] gagal ({attempts}/{MAX_ATTEMPTS}): {msg}",
            job.job_id
        );
        let _ = db.mark_job_failed(&job.job_id, &job.post_id, attempts, retry, &msg);
    }
}

pub async fn test_provider_connection(
    db: &Database,
    client: &Client,
    user_id: &str,
    provider: &str,
) -> ConnectionTestResult {
    let creds = match db.resolve_credentials(user_id, provider) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return ConnectionTestResult {
                ok: false,
                message: "Client ID dan Client Secret belum disimpan.".to_string(),
                accounts_tested: 0,
            }
        }
        Err(e) => {
            return ConnectionTestResult {
                ok: false,
                message: format!("Gagal membaca kredensial: {e}"),
                accounts_tested: 0,
            }
        }
    };

    let platforms: &[&str] = match provider {
        "youtube" => &["youtube"],
        "tiktok" => &["tiktok"],
        "meta" => &["facebook", "instagram"],
        _ => {
            return ConnectionTestResult {
                ok: false,
                message: "Provider tidak dikenal".to_string(),
                accounts_tested: 0,
            }
        }
    };

    let accs = db
        .get_accounts_by_platforms(user_id, platforms)
        .unwrap_or_default();

    if accs.is_empty() {
        if provider == "meta" {
            return match adapters::meta::verify_app_credentials(client, &creds).await {
                Ok(()) => ConnectionTestResult {
                    ok: true,
                    message: "App ID & App Secret Meta valid. Klik 'Hubungkan via OAuth' untuk menautkan Page & Instagram.".to_string(),
                    accounts_tested: 0,
                },
                Err(e) => ConnectionTestResult {
                    ok: false,
                    message: format!("Verifikasi App Meta gagal: {e}"),
                    accounts_tested: 0,
                },
            };
        }
        if provider == "youtube" && !creds.client_id.ends_with(".apps.googleusercontent.com") {
            return ConnectionTestResult {
                ok: false,
                message: "Format Client ID Google biasanya diakhiri '.apps.googleusercontent.com'. Periksa kembali.".to_string(),
                accounts_tested: 0,
            };
        }
        return ConnectionTestResult {
            ok: true,
            message: "Kredensial tersimpan. Klik 'Hubungkan via OAuth' untuk menautkan akun.".to_string(),
            accounts_tested: 0,
        };
    }

    let mut labels = Vec::new();
    for acc in &accs {
        let Ok(access_token) = db.cfg.decrypt(&acc.access_token_enc) else {
            return ConnectionTestResult {
                ok: false,
                message: "Gagal mendekripsi token akun. Hubungkan ulang via OAuth.".to_string(),
                accounts_tested: 0,
            };
        };
        let refresh_token = acc
            .refresh_token_enc
            .as_deref()
            .and_then(|e| db.cfg.decrypt(e).ok());

        let mut tokens = TokenSet {
            access_token,
            refresh_token,
            expires_at: acc.expires_at.clone(),
        };

        if adapters::supports_refresh(&acc.platform) && needs_refresh(tokens.expires_at.as_deref())
        {
            match adapters::refresh_tokens(client, &acc.platform, &tokens, &creds).await {
                Ok(new_tok) => {
                    let _ = db.update_account_tokens(&acc.id, &new_tok);
                    tokens = new_tok;
                }
                Err(e) => {
                    return ConnectionTestResult {
                        ok: false,
                        message: format!("Gagal me-refresh token {}: {e}", acc.platform),
                        accounts_tested: accs.len(),
                    }
                }
            }
        }

        match adapters::verify_account(client, provider, &tokens, &acc.external_id).await {
            Ok(name) => labels.push(format!("{}: {name}", acc.platform)),
            Err(e) => {
                return ConnectionTestResult {
                    ok: false,
                    message: format!("Verifikasi {} gagal: {e}", acc.platform),
                    accounts_tested: accs.len(),
                }
            }
        }
    }

    let has_fb = accs.iter().any(|a| a.platform == "facebook");
    let has_ig = accs.iter().any(|a| a.platform == "instagram");
    let extra_hint = if provider == "meta" && has_fb && !has_ig {
        ". Instagram belum terdeteksi pada Facebook Page ini. Pastikan akun IG bertipe Professional (Business/Creator) dan tertaut di Page Settings > Linked Accounts > Instagram, lalu klik Hubungkan akun lain."
    } else {
        ""
    };

    ConnectionTestResult {
        ok: true,
        message: format!("Koneksi aktif ({}){extra_hint}", labels.join(", ")),
        accounts_tested: accs.len(),
    }
}

pub async fn refresh_published_metrics(
    db: &Database,
    client: &Client,
    user_id: &str,
    job_id: Option<&str>,
    platform: Option<&str>,
) -> MetricsRefreshResult {
    let jobs = match db.list_published_jobs_for_metrics(user_id, job_id, platform) {
        Ok(list) => list,
        Err(e) => {
            return MetricsRefreshResult {
                ok: false,
                total: 0,
                updated: 0,
                failed: 0,
                message: format!("Gagal membaca daftar postingan terbit: {e}"),
            }
        }
    };

    if jobs.is_empty() {
        return MetricsRefreshResult {
            ok: true,
            total: 0,
            updated: 0,
            failed: 0,
            message: "Belum ada video terbit untuk diperiksa performanya.".to_string(),
        };
    }

    let total = jobs.len();
    let mut by_account: HashMap<String, Vec<PublishedJobRef>> = HashMap::new();
    for j in jobs {
        by_account.entry(j.account_id.clone()).or_default().push(j);
    }

    let mut updated = 0usize;
    let mut failed = 0usize;

    for (acc_id, group) in by_account {
        let plat = group[0].platform.clone();
        let Ok(Some(acc)) = db.get_account_by_id(&acc_id) else {
            for j in &group {
                let _ = db.update_job_metrics(
                    &j.job_id,
                    None,
                    None,
                    None,
                    None,
                    Some("Akun sudah diputuskan."),
                );
                failed += 1;
            }
            continue;
        };

        let Ok(access_token) = db.cfg.decrypt(&acc.access_token_enc) else {
            for j in &group {
                let _ = db.update_job_metrics(
                    &j.job_id,
                    None,
                    None,
                    None,
                    None,
                    Some("Gagal mendekripsi token akun."),
                );
                failed += 1;
            }
            continue;
        };
        let refresh_token = acc
            .refresh_token_enc
            .as_deref()
            .and_then(|e| db.cfg.decrypt(e).ok());

        let mut tokens = TokenSet {
            access_token,
            refresh_token,
            expires_at: acc.expires_at.clone(),
        };

        if adapters::supports_refresh(&plat) && needs_refresh(tokens.expires_at.as_deref()) {
            let prov = provider_for_platform(&plat);
            if let Ok(Some(creds)) = db.resolve_credentials(user_id, prov) {
                match adapters::refresh_tokens(client, &plat, &tokens, &creds).await {
                    Ok(new_tok) => {
                        let _ = db.update_account_tokens(&acc.id, &new_tok);
                        tokens = new_tok;
                    }
                    Err(e) => {
                        let msg = format!("Gagal refresh token {plat}: {e}");
                        for j in &group {
                            let _ = db.update_job_metrics(
                                &j.job_id,
                                None,
                                None,
                                None,
                                None,
                                Some(&msg),
                            );
                            failed += 1;
                        }
                        continue;
                    }
                }
            }
        }

        let post_ids: Vec<String> = group.iter().map(|j| j.platform_post_id.clone()).collect();
        match adapters::fetch_metrics_batch(client, &plat, &tokens, &post_ids).await {
            Ok(map) => {
                for j in &group {
                    if let Some(m) = map.get(&j.platform_post_id) {
                        let has_any = m.views.is_some()
                            || m.likes.is_some()
                            || m.comments.is_some()
                            || m.shares.is_some();
                        let _ = db.update_job_metrics(
                            &j.job_id,
                            m.views,
                            m.likes,
                            m.comments,
                            m.shares,
                            m.note.as_deref(),
                        );
                        if has_any {
                            updated += 1;
                        } else {
                            failed += 1;
                        }
                    } else {
                        let missing_msg = if plat == "tiktok" {
                            "Data belum tersedia (video masih draft/privat atau belum terindeks)."
                        } else {
                            "Video tidak ditemukan di platform (mungkin sudah dihapus)."
                        };
                        let _ = db.update_job_metrics(
                            &j.job_id,
                            None,
                            None,
                            None,
                            None,
                            Some(missing_msg),
                        );
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                for j in &group {
                    let _ = db.update_job_metrics(
                        &j.job_id,
                        None,
                        None,
                        None,
                        None,
                        Some(&msg),
                    );
                    failed += 1;
                }
            }
        }
    }

    let message = if failed == 0 {
        format!("Performa {updated} video berhasil diperbarui.")
    } else if updated > 0 {
        format!("{updated} video diperbarui, {failed} video belum dapat diambil metriknya.")
    } else {
        format!("Gagal memperbarui metrik ({failed} video). Cek catatan di tiap baris.")
    };

    MetricsRefreshResult {
        ok: updated > 0 || failed == 0,
        total,
        updated,
        failed,
        message,
    }
}
