use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct AppConfig {
    pub app_url: String,
    pub port: u16,
    pub allowed_email: String,
    pub app_password: String,
    pub auth_secret: String,
    pub enc_key: [u8; 32],
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub ffmpeg_bin: String,
    pub ffprobe_bin: String,
    pub youtube_privacy: String,
    pub tiktok_privacy: String,
    pub contact_email: String,
    pub tiktok_site_verification: Option<String>,
}

pub fn env_opt(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let app_url = env_opt("APP_URL")
            .unwrap_or_else(|| "http://localhost:3100".to_string())
            .trim_end_matches('/')
            .to_string();

        let port = env_opt("PORT")
            .and_then(|p| p.parse::<u16>().ok())
            .or_else(|| {
                app_url
                    .rsplit_once(':')
                    .and_then(|(_, p)| p.split('/').next()?.parse::<u16>().ok())
            })
            .unwrap_or(3100);

        let allowed_email = env_opt("ALLOWED_EMAIL").unwrap_or_else(|| "owner@local".to_string());
        let app_password = env_opt("APP_PASSWORD")
            .ok_or_else(|| anyhow!("APP_PASSWORD belum diisi di file .env"))?;
        let auth_secret = env_opt("AUTH_SECRET")
            .ok_or_else(|| anyhow!("AUTH_SECRET belum diisi di file .env"))?;

        let key_hex = env_opt("TOKEN_ENCRYPTION_KEY")
            .ok_or_else(|| anyhow!("TOKEN_ENCRYPTION_KEY belum diisi di file .env"))?;
        let key_vec = hex::decode(&key_hex)
            .map_err(|_| anyhow!("TOKEN_ENCRYPTION_KEY harus berupa 64 karakter hex"))?;
        if key_vec.len() != 32 {
            bail!("TOKEN_ENCRYPTION_KEY harus tepat 32 byte (64 karakter hex)");
        }
        let mut enc_key = [0u8; 32];
        enc_key.copy_from_slice(&key_vec);

        let data_dir = env_opt("DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./data"));
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(data_dir.join("uploads"))?;
        std::fs::create_dir_all(data_dir.join("thumbs"))?;
        std::fs::create_dir_all(data_dir.join("tmp"))?;
        std::fs::create_dir_all(data_dir.join("public"))?;

        let db_path = env_opt("DATABASE_URL")
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("crossupload.db"));

        let contact_email = env_opt("CONTACT_EMAIL").unwrap_or_else(|| allowed_email.clone());
        let tiktok_site_verification = env_opt("TIKTOK_SITE_VERIFICATION");

        Ok(Self {
            app_url,
            port,
            allowed_email,
            app_password,
            auth_secret,
            enc_key,
            data_dir,
            db_path,
            ffmpeg_bin: env_opt("FFMPEG_PATH").unwrap_or_else(|| "ffmpeg".to_string()),
            ffprobe_bin: env_opt("FFPROBE_PATH").unwrap_or_else(|| "ffprobe".to_string()),
            youtube_privacy: env_opt("YOUTUBE_PRIVACY").unwrap_or_else(|| "private".to_string()),
            tiktok_privacy: env_opt("TIKTOK_PRIVACY").unwrap_or_else(|| "SELF_ONLY".to_string()),
            contact_email,
            tiktok_site_verification,
        })
    }

    pub fn encrypt(&self, plain: &str) -> Result<String> {
        let mut iv = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut iv);
        let cipher = <Aes256Gcm as KeyInit>::new_from_slice(&self.enc_key)
            .map_err(|_| anyhow!("invalid encryption key"))?;
        let ct = cipher
            .encrypt(Nonce::from_slice(&iv), plain.as_bytes())
            .map_err(|_| anyhow!("enkripsi gagal"))?;
        Ok(format!("v2.{}.{}", B64.encode(iv), B64.encode(ct)))
    }

    pub fn decrypt(&self, payload: &str) -> Result<String> {
        let parts: Vec<&str> = payload.split('.').collect();
        let cipher = <Aes256Gcm as KeyInit>::new_from_slice(&self.enc_key)
            .map_err(|_| anyhow!("invalid encryption key"))?;

        if parts.len() == 3 && parts[0] == "v2" {
            let iv = B64.decode(parts[1])?;
            let ct = B64.decode(parts[2])?;
            if iv.len() != 12 {
                bail!("IV tidak valid");
            }
            let pt = cipher
                .decrypt(Nonce::from_slice(&iv), ct.as_ref())
                .map_err(|_| anyhow!("dekripsi token gagal"))?;
            return Ok(String::from_utf8(pt)?);
        }

        // Kompatibilitas dengan format lama iv.tag.data (base64) bila ada
        if parts.len() == 3 {
            let iv = B64.decode(parts[0])?;
            let tag = B64.decode(parts[1])?;
            let mut data = B64.decode(parts[2])?;
            if iv.len() != 12 {
                bail!("IV lama tidak valid");
            }
            data.extend_from_slice(&tag);
            let pt = cipher
                .decrypt(Nonce::from_slice(&iv), data.as_ref())
                .map_err(|_| anyhow!("dekripsi token lama gagal"))?;
            return Ok(String::from_utf8(pt)?);
        }

        bail!("Format payload terenkripsi tidak dikenali")
    }

    pub fn sign(&self, value: &str) -> String {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(self.auth_secret.as_bytes())
            .expect("HMAC accepts any key size");
        mac.update(value.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    pub fn create_session(&self, user_id: &str) -> String {
        let exp_ms = chrono::Utc::now().timestamp_millis() + 30 * 24 * 3600 * 1000;
        let body = format!("{user_id}.{exp_ms}");
        let sig = self.sign(&body);
        format!("{body}.{sig}")
    }

    pub fn verify_session(&self, token: &str) -> Option<String> {
        let (body, sig) = token.rsplit_once('.')?;
        if !safe_equal(&self.sign(body), sig) {
            return None;
        }
        let (user_id, exp_str) = body.split_once('.')?;
        let exp_ms = exp_str.parse::<i64>().ok()?;
        if user_id.is_empty() || exp_ms < chrono::Utc::now().timestamp_millis() {
            return None;
        }
        Some(user_id.to_string())
    }

    pub fn create_state(&self) -> String {
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut nonce);
        let n_hex = hex::encode(nonce);
        let sig = self.sign(&n_hex);
        format!("{n_hex}.{sig}")
    }

    pub fn verify_state(&self, state: &str, cookie: Option<&str>) -> bool {
        let Some(ck) = cookie else { return false };
        if !safe_equal(state, ck) {
            return false;
        }
        let Some((n, sig)) = state.split_once('.') else {
            return false;
        };
        !n.is_empty() && safe_equal(&self.sign(n), sig)
    }

    pub fn create_thumb_token(&self, thumb_key: &str) -> String {
        let exp_ms = chrono::Utc::now().timestamp_millis() + 2 * 3600 * 1000;
        let key_hex = hex::encode(thumb_key.as_bytes());
        let body = format!("thumb.{key_hex}.{exp_ms}");
        let sig = self.sign(&body);
        format!("{key_hex}.{exp_ms}.{sig}")
    }

    pub fn verify_thumb_token(&self, token: &str) -> Option<String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let key_hex = parts[0];
        let exp_str = parts[1];
        let sig = parts[2];
        let body = format!("thumb.{key_hex}.{exp_str}");
        if !safe_equal(&self.sign(&body), sig) {
            return None;
        }
        let exp_ms = exp_str.parse::<i64>().ok()?;
        if exp_ms < chrono::Utc::now().timestamp_millis() {
            return None;
        }
        let bytes = hex::decode(key_hex).ok()?;
        let key = String::from_utf8(bytes).ok()?;
        let safe = !key.is_empty()
            && key.len() <= 128
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-');
        if !safe || key.contains("..") {
            return None;
        }
        Some(key)
    }
}

pub fn safe_equal(a: &str, b: &str) -> bool {
    let ha = Sha256::digest(a.as_bytes());
    let hb = Sha256::digest(b.as_bytes());
    let mut diff = 0u8;
    for (x, y) in ha.iter().zip(hb.iter()) {
        diff |= x ^ y;
    }
    diff == 0 && a.len() == b.len()
}

pub fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 4 {
        "••••••••".to_string()
    } else {
        let tail: String = chars[chars.len() - 4..].iter().collect();
        format!("••••••••{tail}")
    }
}
