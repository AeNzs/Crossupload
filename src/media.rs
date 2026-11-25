use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct VideoMeta {
    pub duration_sec: f64,
    pub width: i64,
    pub height: i64,
    pub size_bytes: i64,
}

pub async fn probe(ffprobe_bin: &str, file: &Path) -> Result<VideoMeta> {
    let out = Command::new(ffprobe_bin)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(file)
        .output()
        .await
        .map_err(|e| anyhow!("Gagal menjalankan ffprobe ({ffprobe_bin}): {e}"))?;

    if !out.status.success() {
        bail!(
            "ffprobe gagal membaca file video: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let json: Value = serde_json::from_slice(&out.stdout)?;
    let streams = json
        .get("streams")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("Stream video tidak ditemukan"))?;

    let v_stream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|c| c.as_str()) == Some("video"))
        .ok_or_else(|| anyhow!("File tidak memiliki stream video"))?;

    let duration_sec = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            v_stream
                .get("duration")
                .and_then(|d| d.as_str())
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    let width = v_stream.get("width").and_then(|w| w.as_i64()).unwrap_or(0);
    let height = v_stream.get("height").and_then(|h| h.as_i64()).unwrap_or(0);
    let size_bytes = std::fs::metadata(file)?.len() as i64;

    Ok(VideoMeta {
        duration_sec,
        width,
        height,
        size_bytes,
    })
}

pub async fn extract_thumbnail(ffmpeg_bin: &str, input: &Path, output: &Path, seek_sec: f64) -> bool {
    let try_seek = Command::new(ffmpeg_bin)
        .args(["-y", "-ss", &format!("{:.2}", seek_sec.max(0.0)), "-i"])
        .arg(input)
        .args(["-frames:v", "1", "-vf", "scale=480:-2", "-q:v", "4"])
        .arg(output)
        .output()
        .await;

    if let Ok(o) = try_seek {
        if o.status.success() && std::fs::metadata(output).map(|m| m.len() > 0).unwrap_or(false) {
            return true;
        }
    }

    let fallback = Command::new(ffmpeg_bin)
        .args(["-y", "-i"])
        .arg(input)
        .args(["-frames:v", "1", "-vf", "scale=480:-2", "-q:v", "4"])
        .arg(output)
        .output()
        .await;

    matches!(fallback, Ok(o) if o.status.success() && std::fs::metadata(output).map(|m| m.len() > 0).unwrap_or(false))
}

pub async fn normalize_custom_thumbnail(
    ffmpeg_bin: &str,
    input: &Path,
    output: &Path,
) -> Result<()> {
    let first = Command::new(ffmpeg_bin)
        .args([
            "-y",
            "-i",
        ])
        .arg(input)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale='min(1920,iw)':'min(1920,ih)':force_original_aspect_ratio=decrease",
            "-q:v",
            "3",
        ])
        .arg(output)
        .output()
        .await
        .map_err(|e| anyhow!("Gagal menjalankan ffmpeg ({ffmpeg_bin}): {e}"))?;

    if !first.status.success() {
        bail!(
            "File gambar tidak valid atau gagal diproses FFmpeg: {}",
            String::from_utf8_lossy(&first.stderr)
                .chars()
                .take(240)
                .collect::<String>()
        );
    }

    let size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        bail!("Gagal menghasilkan thumbnail JPEG dari file gambar");
    }

    // Batas ukuran thumbnail YouTube Data API v3 adalah 2 MB.
    if size > 2 * 1024 * 1024 {
        let second = Command::new(ffmpeg_bin)
            .args(["-y", "-i"])
            .arg(input)
            .args([
                "-frames:v",
                "1",
                "-vf",
                "scale='min(1280,iw)':'min(1280,ih)':force_original_aspect_ratio=decrease",
                "-q:v",
                "8",
            ])
            .arg(output)
            .output()
            .await
            .map_err(|e| anyhow!("Gagal mengompresi thumbnail ({ffmpeg_bin}): {e}"))?;

        if !second.status.success() {
            bail!("Gagal mengompresi thumbnail agar di bawah 2 MB");
        }
        let size2 = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
        if size2 == 0 || size2 > 2 * 1024 * 1024 {
            bail!("Ukuran thumbnail masih melebihi batas 2 MB");
        }
    }

    Ok(())
}

pub async fn transcode(
    ffmpeg_bin: &str,
    ffprobe_bin: &str,
    input: &Path,
    output: &Path,
) -> Result<VideoMeta> {
    let out = Command::new(ffmpeg_bin)
        .args(["-y", "-i"])
        .arg(input)
        .args([
            "-vf",
            "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2,setsar=1",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "23",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-movflags",
            "+faststart",
        ])
        .arg(output)
        .output()
        .await
        .map_err(|e| anyhow!("Gagal menjalankan ffmpeg ({ffmpeg_bin}): {e}"))?;

    if !out.status.success() {
        bail!(
            "FFmpeg transcode gagal: {}",
            String::from_utf8_lossy(&out.stderr)
                .chars()
                .take(400)
                .collect::<String>()
        );
    }

    probe(ffprobe_bin, output).await
}

pub fn validate_for_platform(platform: &str, meta: &VideoMeta) -> Option<String> {
    let (max_dur, max_bytes): (f64, i64) = match platform {
        "youtube" => (180.0, 256 * 1024 * 1024 * 1024),
        "tiktok" => (600.0, 4 * 1024 * 1024 * 1024),
        "instagram" => (90.0, 1024 * 1024 * 1024),
        "facebook" => (90.0, 1024 * 1024 * 1024),
        _ => (180.0, 1024 * 1024 * 1024),
    };
    if meta.duration_sec > max_dur + 0.4 {
        return Some(format!(
            "{platform}: durasi {:.0} dtk melebihi batas {:.0} dtk",
            meta.duration_sec, max_dur
        ));
    }
    if meta.size_bytes > max_bytes {
        return Some(format!("{platform}: ukuran file melebihi batas maksimum"));
    }
    None
}

pub fn provider_for_platform(platform: &str) -> &'static str {
    match platform {
        "youtube" => "youtube",
        "tiktok" => "tiktok",
        _ => "meta",
    }
}

pub fn build_caption(caption: Option<&str>, hashtags: &[String], extra_tags: &[&str]) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut tags_vec = Vec::new();
    for raw in hashtags
        .iter()
        .map(|s| s.as_str())
        .chain(extra_tags.iter().copied())
    {
        let clean: String = raw
            .trim_start_matches('#')
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if !clean.is_empty() && seen.insert(clean.to_lowercase()) {
            tags_vec.push(format!("#{clean}"));
        }
    }
    let cap = caption.unwrap_or("").trim();
    let tags_line = tags_vec.join(" ");
    match (!cap.is_empty(), !tags_line.is_empty()) {
        (true, true) => format!("{cap}\n\n{tags_line}"),
        (true, false) => cap.to_string(),
        (false, true) => tags_line,
        (false, false) => String::new(),
    }
}
