use crate::security::{env_opt, mask_secret, AppConfig};
use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub const PROVIDERS: [&str; 3] = ["youtube", "tiktok", "meta"];

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    pub cfg: AppConfig,
}

#[derive(Debug, Clone)]
pub struct ClientCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub source: &'static str,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConnectedAccount {
    pub platform: String,
    pub external_id: String,
    pub display_name: Option<String>,
    pub tokens: TokenSet,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSummary {
    pub provider: String,
    pub configured: bool,
    pub source: Option<String>,
    pub client_id: String,
    pub secret_hint: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub platform: String,
    pub display_name: Option<String>,
    pub external_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct AccountRecord {
    pub id: String,
    pub platform: String,
    pub external_id: String,
    pub access_token_enc: String,
    pub refresh_token_enc: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoView {
    pub id: String,
    pub name: String,
    pub storage_key: String,
    pub thumb_key: Option<String>,
    pub thumb_custom: bool,
    pub duration_sec: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub size_bytes: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: String,
    pub status: String,
    pub error: Option<String>,
    pub attempts: i64,
    pub platform_post_id: Option<String>,
    pub published_url: Option<String>,
    pub published_at: Option<String>,
    pub platform: String,
    pub title: Option<String>,
    pub caption: Option<String>,
    pub visibility: Option<String>,
    pub schedule_at: Option<String>,
    pub created_at: String,
    pub account: Option<String>,
    pub video_id: String,
    pub video: String,
    pub thumb_key: Option<String>,
    pub views: Option<i64>,
    pub likes: Option<i64>,
    pub comments: Option<i64>,
    pub shares: Option<i64>,
    pub metrics_updated_at: Option<String>,
    pub metrics_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PublishedJobRef {
    pub job_id: String,
    pub account_id: String,
    pub platform: String,
    pub platform_post_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetView {
    pub id: String,
    pub name: String,
    pub hashtags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyView {
    pub youtube: String,
    pub tiktok: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateView {
    pub accounts: Vec<AccountView>,
    pub videos: Vec<VideoView>,
    pub jobs: Vec<JobView>,
    pub credentials: HashMap<String, CredentialSummary>,
    pub presets: Vec<PresetView>,
    pub privacy: PrivacyView,
    pub app_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformOverride {
    pub title: Option<String>,
    pub caption: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostsInput {
    pub video_id: String,
    pub account_ids: Vec<String>,
    pub title: Option<String>,
    pub caption: Option<String>,
    pub visibility: Option<String>,
    pub hashtags: Option<Vec<String>>,
    pub schedule_at: Option<String>,
    pub overrides: Option<HashMap<String, PlatformOverride>>,
}

#[derive(Debug, Clone)]
pub struct ClaimedJob {
    pub job_id: String,
    pub user_id: String,
    pub post_id: String,
    pub attempts: i64,
    pub video_id: String,
    pub account_id: String,
    pub platform: String,
    pub title: Option<String>,
    pub caption: Option<String>,
    pub visibility: Option<String>,
    pub hashtags: Vec<String>,
}

fn has_column(conn: &Connection, table: &str, col: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt.query_map([], |r| r.get::<_, String>(1))?;
    for n in names {
        if n?.eq_ignore_ascii_case(col) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn resolve_platform_visibility(
    platform: &str,
    raw_override: Option<&str>,
    raw_global: Option<&str>,
    cfg: &AppConfig,
) -> Result<String> {
    let chosen = raw_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| raw_global.map(str::trim).filter(|s| !s.is_empty()));

    match platform {
        "youtube" => {
            let v = match chosen {
                Some(s) => match s.to_ascii_lowercase().as_str() {
                    "public" => "public",
                    "unlisted" => "unlisted",
                    "private" | "draft" => "private",
                    other => bail!("Visibilitas YouTube tidak valid: {other}"),
                },
                None => cfg.youtube_privacy.as_str(),
            };
            Ok(v.to_string())
        }
        "tiktok" => {
            let v = match chosen {
                Some(s) => match s.to_ascii_uppercase().as_str() {
                    "PUBLIC" | "PUBLIC_TO_EVERYONE" => "PUBLIC_TO_EVERYONE",
                    "UNLISTED" | "MUTUAL_FOLLOW_FRIENDS" => "MUTUAL_FOLLOW_FRIENDS",
                    "FOLLOWER_OF_CREATOR" => "FOLLOWER_OF_CREATOR",
                    "PRIVATE" | "SELF_ONLY" => "SELF_ONLY",
                    "DRAFT" => "DRAFT",
                    other => bail!("Visibilitas TikTok tidak valid: {other}"),
                },
                None => cfg.tiktok_privacy.as_str(),
            };
            Ok(v.to_string())
        }
        "facebook" => {
            let v = match chosen {
                Some(s) => match s.to_ascii_uppercase().as_str() {
                    "PUBLIC" | "UNLISTED" | "PUBLISHED" => "PUBLISHED",
                    "PRIVATE" | "DRAFT" => "DRAFT",
                    other => bail!("Visibilitas Facebook tidak valid: {other}"),
                },
                None => "PUBLISHED",
            };
            Ok(v.to_string())
        }
        "instagram" => {
            let v = match chosen {
                Some(s) => match s.to_ascii_lowercase().as_str() {
                    "public" => "public",
                    "reels_only" | "unlisted" | "private" | "draft" => "reels_only",
                    other => bail!("Visibilitas Instagram tidak valid: {other}"),
                },
                None => "public",
            };
            Ok(v.to_string())
        }
        _ => Ok(chosen.unwrap_or("public").to_string()),
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

impl Database {
    pub fn open(cfg: AppConfig) -> Result<Self> {
        let conn = Connection::open(&cfg.db_path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                email TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS credentials (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                provider TEXT NOT NULL,
                client_id TEXT NOT NULL,
                client_secret_enc TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(user_id, provider)
            );

            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                platform TEXT NOT NULL,
                external_id TEXT NOT NULL,
                display_name TEXT,
                access_token_enc TEXT NOT NULL,
                refresh_token_enc TEXT,
                expires_at TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_accounts_user ON accounts(user_id);

            CREATE TABLE IF NOT EXISTS videos (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                storage_key TEXT NOT NULL,
                thumb_key TEXT,
                thumb_custom INTEGER NOT NULL DEFAULT 0,
                original_name TEXT NOT NULL,
                duration_sec INTEGER,
                width INTEGER,
                height INTEGER,
                size_bytes INTEGER,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_videos_user ON videos(user_id);

            CREATE TABLE IF NOT EXISTS posts (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                video_id TEXT NOT NULL REFERENCES videos(id),
                account_id TEXT NOT NULL REFERENCES accounts(id),
                platform TEXT NOT NULL,
                title TEXT,
                caption TEXT,
                visibility TEXT,
                hashtags_json TEXT NOT NULL DEFAULT '[]',
                schedule_at TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_posts_user ON posts(user_id);

            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                post_id TEXT NOT NULL REFERENCES posts(id),
                status TEXT NOT NULL DEFAULT 'queued',
                attempts INTEGER NOT NULL DEFAULT 0,
                error TEXT,
                platform_post_id TEXT,
                published_url TEXT,
                published_at TEXT,
                metric_views INTEGER,
                metric_likes INTEGER,
                metric_comments INTEGER,
                metric_shares INTEGER,
                metrics_updated_at TEXT,
                metrics_error TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_jobs_user ON jobs(user_id);
            CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);

            CREATE TABLE IF NOT EXISTS hashtag_presets (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL REFERENCES users(id),
                name TEXT NOT NULL,
                hashtags_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_presets_user ON hashtag_presets(user_id);
            ",
        )?;

        if !has_column(&conn, "videos", "thumb_custom")? {
            conn.execute(
                "ALTER TABLE videos ADD COLUMN thumb_custom INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        if !has_column(&conn, "posts", "visibility")? {
            conn.execute("ALTER TABLE posts ADD COLUMN visibility TEXT", [])?;
        }
        for (col, ty) in [
            ("metric_views", "INTEGER"),
            ("metric_likes", "INTEGER"),
            ("metric_comments", "INTEGER"),
            ("metric_shares", "INTEGER"),
            ("metrics_updated_at", "TEXT"),
            ("metrics_error", "TEXT"),
        ] {
            if !has_column(&conn, "jobs", col)? {
                conn.execute(&format!("ALTER TABLE jobs ADD COLUMN {col} {ty}"), [])?;
            }
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            cfg,
        })
    }

    pub fn upsert_user(&self, email: &str) -> Result<String> {
        let g = self.conn.lock().unwrap();
        if let Some(id) = g
            .query_row(
                "SELECT id FROM users WHERE email = ?1",
                params![email],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(id);
        }
        let id = new_id();
        g.execute(
            "INSERT INTO users (id, email, created_at) VALUES (?1, ?2, ?3)",
            params![id, email, now_iso()],
        )?;
        Ok(id)
    }

    pub fn resolve_credentials(&self, user_id: &str, provider: &str) -> Result<Option<ClientCredentials>> {
        let row = {
            let g = self.conn.lock().unwrap();
            g.query_row(
                "SELECT client_id, client_secret_enc, updated_at FROM credentials WHERE user_id = ?1 AND provider = ?2",
                params![user_id, provider],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
            )
            .optional()?
        };
        if let Some((cid, sec_enc, upd)) = row {
            let sec = self.cfg.decrypt(&sec_enc)?;
            return Ok(Some(ClientCredentials {
                client_id: cid,
                client_secret: sec,
                source: "db",
                updated_at: Some(upd),
            }));
        }

        let (id_env, sec_env) = match provider {
            "youtube" => ("GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"),
            "tiktok" => ("TIKTOK_CLIENT_KEY", "TIKTOK_CLIENT_SECRET"),
            "meta" => ("META_APP_ID", "META_APP_SECRET"),
            _ => return Ok(None),
        };
        if let (Some(cid), Some(sec)) = (env_opt(id_env), env_opt(sec_env)) {
            return Ok(Some(ClientCredentials {
                client_id: cid,
                client_secret: sec,
                source: "env",
                updated_at: None,
            }));
        }
        Ok(None)
    }

    pub fn get_credentials_summary(&self, user_id: &str) -> Result<HashMap<String, CredentialSummary>> {
        let mut map = HashMap::new();
        for p in PROVIDERS {
            let summary = match self.resolve_credentials(user_id, p)? {
                Some(c) => CredentialSummary {
                    provider: p.to_string(),
                    configured: true,
                    source: Some(c.source.to_string()),
                    client_id: c.client_id,
                    secret_hint: Some(mask_secret(&c.client_secret)),
                    updated_at: c.updated_at,
                },
                None => CredentialSummary {
                    provider: p.to_string(),
                    configured: false,
                    source: None,
                    client_id: String::new(),
                    secret_hint: None,
                    updated_at: None,
                },
            };
            map.insert(p.to_string(), summary);
        }
        Ok(map)
    }

    pub fn save_credential(
        &self,
        user_id: &str,
        provider: &str,
        client_id: &str,
        client_secret: Option<&str>,
    ) -> Result<()> {
        if !PROVIDERS.contains(&provider) {
            bail!("Provider tidak dikenal");
        }
        let cid = client_id.trim();
        if cid.is_empty() {
            bail!("Client ID tidak boleh kosong");
        }
        let sec_trim = client_secret.unwrap_or("").trim();

        let existing: Option<(String, String)> = {
            let g = self.conn.lock().unwrap();
            g.query_row(
                "SELECT id, client_secret_enc FROM credentials WHERE user_id = ?1 AND provider = ?2",
                params![user_id, provider],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
        };

        let secret_enc = if !sec_trim.is_empty() {
            self.cfg.encrypt(sec_trim)?
        } else if let Some((_, ref prev_enc)) = existing {
            prev_enc.clone()
        } else if let Some(fallback) = self.resolve_credentials(user_id, provider)? {
            self.cfg.encrypt(&fallback.client_secret)?
        } else {
            bail!("Client Secret wajib diisi saat pertama kali menyimpan");
        };

        let now = now_iso();
        let g = self.conn.lock().unwrap();
        if let Some((row_id, _)) = existing {
            g.execute(
                "UPDATE credentials SET client_id = ?1, client_secret_enc = ?2, updated_at = ?3 WHERE id = ?4",
                params![cid, secret_enc, now, row_id],
            )?;
        } else {
            g.execute(
                "INSERT INTO credentials (id, user_id, provider, client_id, client_secret_enc, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![new_id(), user_id, provider, cid, secret_enc, now],
            )?;
        }
        Ok(())
    }

    pub fn delete_credential(&self, user_id: &str, provider: &str) -> Result<()> {
        let g = self.conn.lock().unwrap();
        g.execute(
            "DELETE FROM credentials WHERE user_id = ?1 AND provider = ?2",
            params![user_id, provider],
        )?;
        Ok(())
    }

    pub fn save_accounts(&self, user_id: &str, list: &[ConnectedAccount]) -> Result<()> {
        let g = self.conn.lock().unwrap();
        for a in list {
            let access_enc = self.cfg.encrypt(&a.tokens.access_token)?;
            let refresh_enc = match &a.tokens.refresh_token {
                Some(rt) => Some(self.cfg.encrypt(rt)?),
                None => None,
            };
            let existing: Option<String> = g
                .query_row(
                    "SELECT id FROM accounts WHERE user_id = ?1 AND platform = ?2 AND external_id = ?3",
                    params![user_id, a.platform, a.external_id],
                    |r| r.get(0),
                )
                .optional()?;

            if let Some(acc_id) = existing {
                g.execute(
                    "UPDATE accounts SET display_name = ?1, access_token_enc = ?2, refresh_token_enc = ?3, expires_at = ?4 WHERE id = ?5",
                    params![a.display_name, access_enc, refresh_enc, a.tokens.expires_at, acc_id],
                )?;
            } else {
                g.execute(
                    "INSERT INTO accounts (id, user_id, platform, external_id, display_name, access_token_enc, refresh_token_enc, expires_at, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        new_id(),
                        user_id,
                        a.platform,
                        a.external_id,
                        a.display_name,
                        access_enc,
                        refresh_enc,
                        a.tokens.expires_at,
                        now_iso()
                    ],
                )?;
            }
        }
        Ok(())
    }

    pub fn update_account_tokens(&self, account_id: &str, tokens: &TokenSet) -> Result<()> {
        let access_enc = self.cfg.encrypt(&tokens.access_token)?;
        let refresh_enc = match &tokens.refresh_token {
            Some(rt) => Some(self.cfg.encrypt(rt)?),
            None => None,
        };
        let g = self.conn.lock().unwrap();
        g.execute(
            "UPDATE accounts SET access_token_enc = ?1, refresh_token_enc = ?2, expires_at = ?3 WHERE id = ?4",
            params![access_enc, refresh_enc, tokens.expires_at, account_id],
        )?;
        Ok(())
    }

    pub fn get_accounts_by_platforms(
        &self,
        user_id: &str,
        platforms: &[&str],
    ) -> Result<Vec<AccountRecord>> {
        let g = self.conn.lock().unwrap();
        let mut stmt = g.prepare(
            "SELECT id, platform, external_id, access_token_enc, refresh_token_enc, expires_at
             FROM accounts WHERE user_id = ?1",
        )?;
        let rows = stmt.query_map(params![user_id], |r| {
            Ok(AccountRecord {
                id: r.get(0)?,
                platform: r.get(1)?,
                external_id: r.get(2)?,
                access_token_enc: r.get(3)?,
                refresh_token_enc: r.get(4)?,
                expires_at: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            let rec = r?;
            if platforms.contains(&rec.platform.as_str()) {
                out.push(rec);
            }
        }
        Ok(out)
    }

    pub fn get_account_by_id(&self, account_id: &str) -> Result<Option<AccountRecord>> {
        let g = self.conn.lock().unwrap();
        g.query_row(
            "SELECT id, platform, external_id, access_token_enc, refresh_token_enc, expires_at
             FROM accounts WHERE id = ?1",
            params![account_id],
            |r| {
                Ok(AccountRecord {
                    id: r.get(0)?,
                    platform: r.get(1)?,
                    external_id: r.get(2)?,
                    access_token_enc: r.get(3)?,
                    refresh_token_enc: r.get(4)?,
                    expires_at: r.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn delete_account(&self, user_id: &str, account_id: &str) -> Result<()> {
        let mut g = self.conn.lock().unwrap();
        let tx = g.transaction()?;
        tx.execute(
            "DELETE FROM jobs WHERE post_id IN (SELECT id FROM posts WHERE account_id = ?1 AND user_id = ?2)",
            params![account_id, user_id],
        )?;
        tx.execute(
            "DELETE FROM posts WHERE account_id = ?1 AND user_id = ?2",
            params![account_id, user_id],
        )?;
        tx.execute(
            "DELETE FROM accounts WHERE id = ?1 AND user_id = ?2",
            params![account_id, user_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_video(
        &self,
        user_id: &str,
        storage_key: &str,
        thumb_key: Option<&str>,
        original_name: &str,
        duration_sec: i64,
        width: i64,
        height: i64,
        size_bytes: i64,
    ) -> Result<VideoView> {
        let id = new_id();
        let created_at = now_iso();
        let g = self.conn.lock().unwrap();
        g.execute(
            "INSERT INTO videos (id, user_id, storage_key, thumb_key, thumb_custom, original_name, duration_sec, width, height, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                user_id,
                storage_key,
                thumb_key,
                original_name,
                duration_sec,
                width,
                height,
                size_bytes,
                created_at
            ],
        )?;
        Ok(VideoView {
            id,
            name: original_name.to_string(),
            storage_key: storage_key.to_string(),
            thumb_key: thumb_key.map(String::from),
            thumb_custom: false,
            duration_sec: Some(duration_sec),
            width: Some(width),
            height: Some(height),
            size_bytes: Some(size_bytes),
            created_at,
        })
    }

    pub fn update_video_thumb(
        &self,
        user_id: &str,
        video_id: &str,
        new_thumb_key: Option<&str>,
        is_custom: bool,
    ) -> Result<Option<(VideoView, Option<String>)>> {
        let mut g = self.conn.lock().unwrap();
        let tx = g.transaction()?;
        let prev_key: Option<Option<String>> = tx
            .query_row(
                "SELECT thumb_key FROM videos WHERE id = ?1 AND user_id = ?2",
                params![video_id, user_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?;
        let Some(old_thumb_key) = prev_key else {
            return Ok(None);
        };
        tx.execute(
            "UPDATE videos SET thumb_key = ?1, thumb_custom = ?2 WHERE id = ?3 AND user_id = ?4",
            params![new_thumb_key, if is_custom { 1i64 } else { 0i64 }, video_id, user_id],
        )?;
        let updated = tx.query_row(
            "SELECT id, original_name, storage_key, thumb_key, thumb_custom, duration_sec, width, height, size_bytes, created_at
             FROM videos WHERE id = ?1 AND user_id = ?2",
            params![video_id, user_id],
            |r| {
                Ok(VideoView {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    storage_key: r.get(2)?,
                    thumb_key: r.get(3)?,
                    thumb_custom: r.get::<_, i64>(4)? != 0,
                    duration_sec: r.get(5)?,
                    width: r.get(6)?,
                    height: r.get(7)?,
                    size_bytes: r.get(8)?,
                    created_at: r.get(9)?,
                })
            },
        )?;
        tx.commit()?;
        Ok(Some((updated, old_thumb_key)))
    }

    pub fn get_video(&self, user_id: Option<&str>, video_id: &str) -> Result<Option<VideoView>> {
        let g = self.conn.lock().unwrap();
        let map_row = |r: &rusqlite::Row<'_>| -> rusqlite::Result<VideoView> {
            Ok(VideoView {
                id: r.get(0)?,
                name: r.get(1)?,
                storage_key: r.get(2)?,
                thumb_key: r.get(3)?,
                thumb_custom: r.get::<_, i64>(4)? != 0,
                duration_sec: r.get(5)?,
                width: r.get(6)?,
                height: r.get(7)?,
                size_bytes: r.get(8)?,
                created_at: r.get(9)?,
            })
        };
        if let Some(uid) = user_id {
            g.query_row(
                "SELECT id, original_name, storage_key, thumb_key, thumb_custom, duration_sec, width, height, size_bytes, created_at
                 FROM videos WHERE id = ?1 AND user_id = ?2",
                params![video_id, uid],
                map_row,
            )
            .optional()
            .map_err(Into::into)
        } else {
            g.query_row(
                "SELECT id, original_name, storage_key, thumb_key, thumb_custom, duration_sec, width, height, size_bytes, created_at
                 FROM videos WHERE id = ?1",
                params![video_id],
                map_row,
            )
            .optional()
            .map_err(Into::into)
        }
    }

    pub fn delete_video(&self, user_id: &str, video_id: &str) -> Result<Option<VideoView>> {
        let mut g = self.conn.lock().unwrap();
        let tx = g.transaction()?;
        let v: Option<VideoView> = tx
            .query_row(
                "SELECT id, original_name, storage_key, thumb_key, thumb_custom, duration_sec, width, height, size_bytes, created_at
                 FROM videos WHERE id = ?1 AND user_id = ?2",
                params![video_id, user_id],
                |r| {
                    Ok(VideoView {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        storage_key: r.get(2)?,
                        thumb_key: r.get(3)?,
                        thumb_custom: r.get::<_, i64>(4)? != 0,
                        duration_sec: r.get(5)?,
                        width: r.get(6)?,
                        height: r.get(7)?,
                        size_bytes: r.get(8)?,
                        created_at: r.get(9)?,
                    })
                },
            )
            .optional()?;
        let Some(video) = v else { return Ok(None) };

        let busy: Option<String> = tx
            .query_row(
                "SELECT j.id FROM jobs j INNER JOIN posts p ON j.post_id = p.id
                 WHERE p.video_id = ?1 AND j.status = 'processing' LIMIT 1",
                params![video_id],
                |r| r.get(0),
            )
            .optional()?;
        if busy.is_some() {
            bail!("Video sedang diproses worker, tunggu hingga selesai");
        }

        tx.execute(
            "DELETE FROM jobs WHERE post_id IN (SELECT id FROM posts WHERE video_id = ?1 AND user_id = ?2)",
            params![video_id, user_id],
        )?;
        tx.execute(
            "DELETE FROM posts WHERE video_id = ?1 AND user_id = ?2",
            params![video_id, user_id],
        )?;
        tx.execute(
            "DELETE FROM videos WHERE id = ?1 AND user_id = ?2",
            params![video_id, user_id],
        )?;
        tx.commit()?;
        Ok(Some(video))
    }

    pub fn list_presets(&self, user_id: &str) -> Result<Vec<PresetView>> {
        let g = self.conn.lock().unwrap();
        let mut stmt = g.prepare(
            "SELECT id, name, hashtags_json FROM hashtag_presets WHERE user_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![user_id], |r| {
            let raw: String = r.get(2)?;
            let hashtags: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
            Ok(PresetView {
                id: r.get(0)?,
                name: r.get(1)?,
                hashtags,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    pub fn save_preset(&self, user_id: &str, name: &str, tags: &[String]) -> Result<()> {
        let clean_name = name.trim();
        let mut seen = HashSet::new();
        let mut clean_tags = Vec::new();
        for t in tags {
            let s = t.trim().trim_start_matches('#').trim().to_string();
            if !s.is_empty() && seen.insert(s.to_lowercase()) {
                clean_tags.push(s);
            }
        }
        if clean_name.is_empty() || clean_tags.is_empty() {
            bail!("Nama preset dan minimal 1 hashtag wajib diisi");
        }
        let g = self.conn.lock().unwrap();
        g.execute(
            "INSERT INTO hashtag_presets (id, user_id, name, hashtags_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![new_id(), user_id, clean_name, serde_json::to_string(&clean_tags)?, now_iso()],
        )?;
        Ok(())
    }

    pub fn delete_preset(&self, user_id: &str, preset_id: &str) -> Result<()> {
        let g = self.conn.lock().unwrap();
        g.execute(
            "DELETE FROM hashtag_presets WHERE id = ?1 AND user_id = ?2",
            params![preset_id, user_id],
        )?;
        Ok(())
    }

    pub fn create_posts(&self, user_id: &str, input: CreatePostsInput) -> Result<Vec<String>> {
        if input.account_ids.is_empty() {
            bail!("Pilih minimal satu akun tujuan");
        }
        if self.get_video(Some(user_id), &input.video_id)?.is_none() {
            bail!("Video tidak ditemukan");
        }
        if let Some(ref sched) = input.schedule_at {
            chrono::DateTime::parse_from_rfc3339(sched)
                .map_err(|_| anyhow!("Format waktu jadwal tidak valid"))?;
        }

        let unique_ids: HashSet<String> = input.account_ids.iter().cloned().collect();
        let mut g = self.conn.lock().unwrap();
        let tx = g.transaction()?;

        let mut accs = Vec::new();
        for aid in &unique_ids {
            let row: Option<(String, String)> = tx
                .query_row(
                    "SELECT id, platform FROM accounts WHERE id = ?1 AND user_id = ?2",
                    params![aid, user_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let Some(a) = row else {
                bail!("Salah satu akun tujuan tidak valid");
            };
            accs.push(a);
        }

        let tags_json = serde_json::to_string(&input.hashtags.unwrap_or_default())?;
        let now = now_iso();
        let mut job_ids = Vec::new();

        for (acc_id, platform) in accs {
            let ov = input.overrides.as_ref().and_then(|m| m.get(&platform));
            let title_val = ov
                .and_then(|o| o.title.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or_else(|| input.title.as_deref().map(str::trim).filter(|s| !s.is_empty()))
                .map(String::from);
            let caption_val = ov
                .and_then(|o| o.caption.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or_else(|| input.caption.as_deref().map(str::trim).filter(|s| !s.is_empty()))
                .map(String::from);
            let visibility_val = resolve_platform_visibility(
                &platform,
                ov.and_then(|o| o.visibility.as_deref()),
                input.visibility.as_deref(),
                &self.cfg,
            )?;

            let post_id = new_id();
            let job_id = new_id();
            tx.execute(
                "INSERT INTO posts (id, user_id, video_id, account_id, platform, title, caption, visibility, hashtags_json, schedule_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    post_id,
                    user_id,
                    input.video_id,
                    acc_id,
                    platform,
                    title_val,
                    caption_val,
                    visibility_val,
                    tags_json,
                    input.schedule_at,
                    now
                ],
            )?;
            tx.execute(
                "INSERT INTO jobs (id, user_id, post_id, status, attempts, created_at) VALUES (?1, ?2, ?3, 'queued', 0, ?4)",
                params![job_id, user_id, post_id, now],
            )?;
            job_ids.push(job_id);
        }

        tx.commit()?;
        Ok(job_ids)
    }

    pub fn retry_job(&self, user_id: &str, job_id: &str) -> Result<bool> {
        let mut g = self.conn.lock().unwrap();
        let tx = g.transaction()?;
        let row: Option<(String, String)> = tx
            .query_row(
                "SELECT post_id, status FROM jobs WHERE id = ?1 AND user_id = ?2",
                params![job_id, user_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((post_id, status)) = row else { return Ok(false) };
        if status != "failed" && status != "cancelled" {
            return Ok(false);
        }
        tx.execute(
            "UPDATE jobs SET status = 'queued', error = NULL, attempts = 0 WHERE id = ?1",
            params![job_id],
        )?;
        tx.execute(
            "UPDATE posts SET schedule_at = NULL WHERE id = ?1",
            params![post_id],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn cancel_job(&self, user_id: &str, job_id: &str) -> Result<bool> {
        let g = self.conn.lock().unwrap();
        let n = g.execute(
            "UPDATE jobs SET status = 'cancelled' WHERE id = ?1 AND user_id = ?2 AND status = 'queued'",
            params![job_id, user_id],
        )?;
        Ok(n > 0)
    }

    pub fn get_state(&self, user_id: &str) -> Result<AppStateView> {
        let credentials = self.get_credentials_summary(user_id)?;
        let presets = self.list_presets(user_id)?;

        let (accounts, videos, jobs) = {
            let g = self.conn.lock().unwrap();

            let mut acc_stmt = g.prepare(
                "SELECT id, platform, display_name, external_id, created_at FROM accounts WHERE user_id = ?1 ORDER BY created_at ASC",
            )?;
            let accounts = acc_stmt
                .query_map(params![user_id], |r| {
                    Ok(AccountView {
                        id: r.get(0)?,
                        platform: r.get(1)?,
                        display_name: r.get(2)?,
                        external_id: r.get(3)?,
                        created_at: r.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            let mut vid_stmt = g.prepare(
                "SELECT id, original_name, storage_key, thumb_key, thumb_custom, duration_sec, width, height, size_bytes, created_at
                 FROM videos WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 50",
            )?;
            let videos = vid_stmt
                .query_map(params![user_id], |r| {
                    Ok(VideoView {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        storage_key: r.get(2)?,
                        thumb_key: r.get(3)?,
                        thumb_custom: r.get::<_, i64>(4)? != 0,
                        duration_sec: r.get(5)?,
                        width: r.get(6)?,
                        height: r.get(7)?,
                        size_bytes: r.get(8)?,
                        created_at: r.get(9)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            let mut job_stmt = g.prepare(
                "SELECT j.id, j.status, j.error, j.attempts, j.platform_post_id, j.published_url, j.published_at,
                        p.platform, p.title, p.caption, p.visibility, p.schedule_at, j.created_at,
                        a.display_name, v.id, v.original_name, v.thumb_key,
                        j.metric_views, j.metric_likes, j.metric_comments, j.metric_shares,
                        j.metrics_updated_at, j.metrics_error
                 FROM jobs j
                 INNER JOIN posts p ON j.post_id = p.id
                 INNER JOIN accounts a ON p.account_id = a.id
                 INNER JOIN videos v ON p.video_id = v.id
                 WHERE j.user_id = ?1
                 ORDER BY j.created_at DESC
                 LIMIT 100",
            )?;
            let jobs = job_stmt
                .query_map(params![user_id], |r| {
                    Ok(JobView {
                        id: r.get(0)?,
                        status: r.get(1)?,
                        error: r.get(2)?,
                        attempts: r.get(3)?,
                        platform_post_id: r.get(4)?,
                        published_url: r.get(5)?,
                        published_at: r.get(6)?,
                        platform: r.get(7)?,
                        title: r.get(8)?,
                        caption: r.get(9)?,
                        visibility: r.get(10)?,
                        schedule_at: r.get(11)?,
                        created_at: r.get(12)?,
                        account: r.get(13)?,
                        video_id: r.get(14)?,
                        video: r.get(15)?,
                        thumb_key: r.get(16)?,
                        views: r.get(17)?,
                        likes: r.get(18)?,
                        comments: r.get(19)?,
                        shares: r.get(20)?,
                        metrics_updated_at: r.get(21)?,
                        metrics_error: r.get(22)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            (accounts, videos, jobs)
        };

        Ok(AppStateView {
            accounts,
            videos,
            jobs,
            credentials,
            presets,
            privacy: PrivacyView {
                youtube: self.cfg.youtube_privacy.clone(),
                tiktok: self.cfg.tiktok_privacy.clone(),
            },
            app_url: self.cfg.app_url.clone(),
        })
    }

    pub fn recover_stuck_jobs(&self) -> Result<()> {
        let g = self.conn.lock().unwrap();
        g.execute("UPDATE jobs SET status = 'queued' WHERE status = 'processing'", [])?;
        Ok(())
    }

    pub fn claim_next_job(&self) -> Result<Option<ClaimedJob>> {
        let now = Utc::now();
        let mut g = self.conn.lock().unwrap();
        let tx = g.transaction()?;

        let mut candidate: Option<ClaimedJob> = None;
        {
            let mut stmt = tx.prepare(
                "SELECT j.id, j.user_id, j.post_id, j.attempts, p.video_id, p.account_id, p.platform,
                        p.title, p.caption, p.visibility, p.hashtags_json, p.schedule_at
                 FROM jobs j
                 INNER JOIN posts p ON j.post_id = p.id
                 WHERE j.status = 'queued'
                 ORDER BY j.created_at ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, String>(10)?,
                    r.get::<_, Option<String>>(11)?,
                ))
            })?;

            for row in rows {
                let (jid, uid, pid, attempts, vid, aid, plat, title, caption, visibility, h_json, sched) = row?;
                let due = match sched {
                    None => true,
                    Some(ref s) if s.trim().is_empty() => true,
                    Some(ref s) => chrono::DateTime::parse_from_rfc3339(s)
                        .map(|dt| dt.with_timezone(&Utc) <= now)
                        .unwrap_or(true),
                };
                if due {
                    let hashtags: Vec<String> = serde_json::from_str(&h_json).unwrap_or_default();
                    candidate = Some(ClaimedJob {
                        job_id: jid,
                        user_id: uid,
                        post_id: pid,
                        attempts,
                        video_id: vid,
                        account_id: aid,
                        platform: plat,
                        title,
                        caption,
                        visibility,
                        hashtags,
                    });
                    break;
                }
            }
        }

        let Some(job) = candidate else {
            return Ok(None);
        };

        let changed = tx.execute(
            "UPDATE jobs SET status = 'processing' WHERE id = ?1 AND status = 'queued'",
            params![job.job_id],
        )?;
        tx.commit()?;

        if changed == 0 {
            Ok(None)
        } else {
            Ok(Some(job))
        }
    }

    pub fn mark_job_published(
        &self,
        job_id: &str,
        platform_post_id: &str,
        published_url: Option<&str>,
    ) -> Result<()> {
        let g = self.conn.lock().unwrap();
        g.execute(
            "UPDATE jobs SET status = 'published', platform_post_id = ?1, published_url = ?2, error = NULL, published_at = ?3 WHERE id = ?4",
            params![platform_post_id, published_url, now_iso(), job_id],
        )?;
        Ok(())
    }

    pub fn mark_job_failed(
        &self,
        job_id: &str,
        post_id: &str,
        attempts: i64,
        retry: bool,
        err_msg: &str,
    ) -> Result<()> {
        let status = if retry { "queued" } else { "failed" };
        let truncated: String = err_msg.chars().take(1000).collect();
        let mut g = self.conn.lock().unwrap();
        let tx = g.transaction()?;
        tx.execute(
            "UPDATE jobs SET status = ?1, attempts = ?2, error = ?3 WHERE id = ?4",
            params![status, attempts, truncated, job_id],
        )?;
        if retry {
            let next_run = (Utc::now() + chrono::Duration::minutes(attempts * 2)).to_rfc3339();
            tx.execute(
                "UPDATE posts SET schedule_at = ?1 WHERE id = ?2",
                params![next_run, post_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_published_jobs_for_metrics(
        &self,
        user_id: &str,
        job_id: Option<&str>,
        platform: Option<&str>,
    ) -> Result<Vec<PublishedJobRef>> {
        let g = self.conn.lock().unwrap();
        let mut stmt = g.prepare(
            "SELECT j.id, p.account_id, p.platform, j.platform_post_id
             FROM jobs j
             INNER JOIN posts p ON j.post_id = p.id
             WHERE j.user_id = ?1
               AND j.status = 'published'
               AND j.platform_post_id IS NOT NULL
               AND TRIM(j.platform_post_id) != ''
             ORDER BY j.published_at DESC, j.created_at DESC
             LIMIT 100",
        )?;
        let rows = stmt.query_map(params![user_id], |r| {
            Ok(PublishedJobRef {
                job_id: r.get(0)?,
                account_id: r.get(1)?,
                platform: r.get(2)?,
                platform_post_id: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            let item = row?;
            if let Some(jid) = job_id {
                if item.job_id != jid {
                    continue;
                }
            }
            if let Some(pl) = platform {
                if !pl.is_empty() && pl != "all" && item.platform != pl {
                    continue;
                }
            }
            out.push(item);
        }
        Ok(out)
    }

    pub fn update_job_metrics(
        &self,
        job_id: &str,
        views: Option<i64>,
        likes: Option<i64>,
        comments: Option<i64>,
        shares: Option<i64>,
        err: Option<&str>,
    ) -> Result<()> {
        let trunc_err = err.map(|s| s.chars().take(400).collect::<String>());
        let g = self.conn.lock().unwrap();
        g.execute(
            "UPDATE jobs
             SET metric_views = COALESCE(?1, metric_views),
                 metric_likes = COALESCE(?2, metric_likes),
                 metric_comments = COALESCE(?3, metric_comments),
                 metric_shares = COALESCE(?4, metric_shares),
                 metrics_updated_at = ?5,
                 metrics_error = ?6
             WHERE id = ?7",
            params![views, likes, comments, shares, now_iso(), trunc_err, job_id],
        )?;
        Ok(())
    }
}
