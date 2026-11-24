use crate::adapters;
use crate::db::{CreatePostsInput, Database, PROVIDERS};
use crate::media::{extract_thumbnail, probe};
use crate::security::safe_equal;
use crate::worker;
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::TryStreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::Path as StdPath;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::{ReaderStream, StreamReader};
use uuid::Uuid;

const LOGIN_HTML: &str = include_str!("ui/login.html");
const INDEX_HTML: &str = include_str!("ui/index.html");
const APP_CSS: &str = include_str!("ui/app.css");
const APP_JS: &str = include_str!("ui/app.js");

const SESSION_COOKIE: &str = "cp_session";
const OAUTH_STATE_COOKIE: &str = "cp_oauth_state";

#[derive(Clone)]
pub struct AppContext {
    pub db: Database,
    pub http: Client,
}

pub fn build_router(ctx: AppContext) -> Router {
    Router::new()
        .route("/", get(page_app))
        .route("/queue", get(page_app))
        .route("/credentials", get(page_app))
        .route("/login", get(page_login))
        .route("/assets/app.css", get(asset_css))
        .route("/assets/app.js", get(asset_js))
        .route("/api/auth/login", post(api_login))
        .route("/api/auth/logout", post(api_logout))
        .route("/api/state", get(api_state))
        .route("/api/credentials", get(api_get_credentials).post(api_save_credential))
        .route("/api/credentials/:provider", delete(api_delete_credential))
        .route("/api/credentials/:provider/test", post(api_test_credential))
        .route("/api/connect/:provider", get(api_connect_provider))
        .route("/api/oauth/:provider/callback", get(api_oauth_callback))
        .route("/api/accounts/:id", delete(api_delete_account))
        .route(
            "/api/videos",
            post(api_upload_video).layer(DefaultBodyLimit::disable()),
        )
        .route("/api/videos/:id", delete(api_delete_video))
        .route("/api/videos/:id/thumb", get(api_video_thumb))
        .route("/api/videos/:id/stream", get(api_video_stream))
        .route("/api/posts", post(api_create_posts))
        .route("/api/jobs/:id", post(api_job_action))
        .route("/api/presets", post(api_save_preset))
        .route("/api/presets/:id", delete(api_delete_preset))
        .with_state(ctx)
}

fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let (k, v) = part.trim().split_once('=')?;
        if k == name {
            return Some(v.to_string());
        }
    }
    None
}

fn current_user_id(ctx: &AppContext, headers: &HeaderMap) -> Option<String> {
    let tok = get_cookie(headers, SESSION_COOKIE)?;
    ctx.db.cfg.verify_session(&tok)
}

fn json_err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

async fn page_login(State(ctx): State<AppContext>, headers: HeaderMap) -> Response {
    if current_user_id(&ctx, &headers).is_some() {
        return Redirect::to("/").into_response();
    }
    Html(LOGIN_HTML).into_response()
}

async fn page_app(State(ctx): State<AppContext>, headers: HeaderMap) -> Response {
    if current_user_id(&ctx, &headers).is_none() {
        return Redirect::to("/login").into_response();
    }
    Html(INDEX_HTML).into_response()
}

async fn asset_css() -> Response {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], APP_CSS).into_response()
}

async fn asset_js() -> Response {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        APP_JS,
    )
        .into_response()
}

#[derive(Deserialize)]
struct LoginReq {
    email: Option<String>,
    password: Option<String>,
}

async fn api_login(State(ctx): State<AppContext>, Json(body): Json<LoginReq>) -> Response {
    let email = body.email.unwrap_or_default().trim().to_lowercase();
    let password = body.password.unwrap_or_default();
    let expected_email = ctx.db.cfg.allowed_email.to_lowercase();

    let ok = safe_equal(&email, &expected_email) && safe_equal(&password, &ctx.db.cfg.app_password);
    if !ok {
        return json_err(StatusCode::UNAUTHORIZED, "Email atau password salah");
    }

    let uid = match ctx.db.upsert_user(&expected_email) {
        Ok(id) => id,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let tok = ctx.db.cfg.create_session(&uid);
    let cookie = format!(
        "{SESSION_COOKIE}={tok}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        30 * 24 * 3600
    );
    ([(header::SET_COOKIE, cookie)], Json(json!({ "ok": true }))).into_response()
}

async fn api_logout() -> Response {
    let cookie = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    ([(header::SET_COOKIE, cookie)], Json(json!({ "ok": true }))).into_response()
}

async fn api_state(State(ctx): State<AppContext>, headers: HeaderMap) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    match ctx.db.get_state(&uid) {
        Ok(st) => Json(st).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn api_get_credentials(State(ctx): State<AppContext>, headers: HeaderMap) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    match ctx.db.get_credentials_summary(&uid) {
        Ok(creds) => Json(json!({
            "credentials": creds,
            "appUrl": ctx.db.cfg.app_url
        }))
        .into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveCredentialReq {
    provider: String,
    client_id: String,
    client_secret: Option<String>,
}

async fn api_save_credential(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<SaveCredentialReq>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    if let Err(e) = ctx.db.save_credential(
        &uid,
        &body.provider,
        &body.client_id,
        body.client_secret.as_deref(),
    ) {
        return json_err(StatusCode::BAD_REQUEST, e.to_string());
    }
    match ctx.db.get_credentials_summary(&uid) {
        Ok(creds) => Json(json!({ "ok": true, "credentials": creds })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn api_delete_credential(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(provider): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    if !PROVIDERS.contains(&provider.as_str()) {
        return json_err(StatusCode::NOT_FOUND, "Provider tidak dikenal");
    }
    let _ = ctx.db.delete_credential(&uid, &provider);
    match ctx.db.get_credentials_summary(&uid) {
        Ok(creds) => Json(json!({ "ok": true, "credentials": creds })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn api_test_credential(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(provider): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    if !PROVIDERS.contains(&provider.as_str()) {
        return json_err(StatusCode::NOT_FOUND, "Provider tidak dikenal");
    }
    let res = worker::test_provider_connection(&ctx.db, &ctx.http, &uid, &provider).await;
    Json(res).into_response()
}

async fn api_connect_provider(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(provider): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return Redirect::to(&format!("{}/login", ctx.db.cfg.app_url)).into_response();
    };
    if !PROVIDERS.contains(&provider.as_str()) {
        return json_err(StatusCode::NOT_FOUND, "Provider tidak dikenal");
    }

    let creds = match ctx.db.resolve_credentials(&uid, &provider) {
        Ok(Some(c)) => c,
        _ => {
            let msg = format!("Isi dan simpan Client ID & Secret {provider} terlebih dahulu.");
            return Redirect::to(&format!(
                "{}/?creds={}&error={}",
                ctx.db.cfg.app_url,
                provider,
                urlencoding::encode(&msg)
            ))
            .into_response();
        }
    };

    let state = ctx.db.cfg.create_state();
    let redirect_uri = format!("{}/api/oauth/{provider}/callback", ctx.db.cfg.app_url);
    let Ok(url) = adapters::get_auth_url(&provider, &creds, &state, &redirect_uri) else {
        return json_err(StatusCode::BAD_REQUEST, "Gagal membuat URL OAuth");
    };

    let cookie = format!("{OAUTH_STATE_COOKIE}={state}; Path=/; HttpOnly; SameSite=Lax; Max-Age=600");
    ([(header::SET_COOKIE, cookie)], Redirect::to(&url)).into_response()
}

async fn api_oauth_callback(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(provider): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return Redirect::to(&format!("{}/login", ctx.db.cfg.app_url)).into_response();
    };
    let back = |extra: &str| {
        Redirect::to(&format!(
            "{}/?creds={}&{extra}",
            ctx.db.cfg.app_url,
            urlencoding::encode(&provider)
        ))
        .into_response()
    };

    if let Some(err) = q.get("error_description").or_else(|| q.get("error")) {
        return back(&format!("error={}", urlencoding::encode(err)));
    }

    let state = q.get("state").map(String::as_str).unwrap_or("");
    let cookie = get_cookie(&headers, OAUTH_STATE_COOKIE);
    let Some(code) = q.get("code") else {
        return back("error=kode-oauth-kosong");
    };
    if !ctx.db.cfg.verify_state(state, cookie.as_deref()) {
        return back("error=state-oauth-tidak-valid");
    }

    let Ok(Some(creds)) = ctx.db.resolve_credentials(&uid, &provider) else {
        return back("error=kredensial-belum-disimpan");
    };

    let redirect_uri = format!("{}/api/oauth/{provider}/callback", ctx.db.cfg.app_url);
    match adapters::connect_provider(&ctx.http, &provider, &creds, code, &redirect_uri).await {
        Ok(list) => {
            if let Err(e) = ctx.db.save_accounts(&uid, &list) {
                return back(&format!("error={}", urlencoding::encode(&e.to_string())));
            }
            let clear = format!("{OAUTH_STATE_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
            (
                [(header::SET_COOKIE, clear)],
                Redirect::to(&format!(
                    "{}/?creds={}&connected={}",
                    ctx.db.cfg.app_url,
                    urlencoding::encode(&provider),
                    list.len()
                )),
            )
                .into_response()
        }
        Err(e) => {
            let short: String = e.to_string().chars().take(280).collect();
            back(&format!("error={}", urlencoding::encode(&short)))
        }
    }
}

async fn api_delete_account(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    match ctx.db.delete_account(&uid, &id) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => json_err(StatusCode::CONFLICT, e.to_string()),
    }
}

async fn api_upload_video(State(ctx): State<AppContext>, req: Request) -> Response {
    let headers = req.headers().clone();
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };

    let raw_name = headers
        .get("x-filename")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("video.mp4");
    let decoded_name = urlencoding::decode(raw_name)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| "video.mp4".to_string());

    let ext = StdPath::new(&decoded_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();

    const ALLOWED: [&str; 5] = [".mp4", ".mov", ".m4v", ".webm", ".mkv"];
    if !ALLOWED.contains(&ext.as_str()) {
        return json_err(
            StatusCode::BAD_REQUEST,
            format!("Format file {} tidak didukung", if ext.is_empty() { "?" } else { &ext }),
        );
    }

    let base_id = Uuid::new_v4().to_string();
    let storage_key = format!("{base_id}{ext}");
    let thumb_key = format!("{base_id}.jpg");
    let file_path = ctx.db.cfg.data_dir.join("uploads").join(&storage_key);
    let thumb_path = ctx.db.cfg.data_dir.join("thumbs").join(&thumb_key);

    let mut out_file = match File::create(&file_path).await {
        Ok(f) => f,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let stream = req
        .into_body()
        .into_data_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let mut reader = StreamReader::new(stream);

    if let Err(e) = tokio::io::copy(&mut reader, &mut out_file).await {
        let _ = std::fs::remove_file(&file_path);
        return json_err(StatusCode::BAD_REQUEST, format!("Gagal menyimpan stream video: {e}"));
    }
    drop(out_file);

    let meta = match probe(&ctx.db.cfg.ffprobe_bin, &file_path).await {
        Ok(m) => m,
        Err(e) => {
            let _ = std::fs::remove_file(&file_path);
            return json_err(StatusCode::BAD_REQUEST, e.to_string());
        }
    };

    let has_thumb = extract_thumbnail(
        &ctx.db.cfg.ffmpeg_bin,
        &file_path,
        &thumb_path,
        (meta.duration_sec / 3.0).min(1.0),
    )
    .await;

    match ctx.db.add_video(
        &uid,
        &storage_key,
        if has_thumb { Some(thumb_key.as_str()) } else { None },
        &decoded_name,
        meta.duration_sec.round() as i64,
        meta.width,
        meta.height,
        meta.size_bytes,
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => {
            let _ = std::fs::remove_file(&file_path);
            let _ = std::fs::remove_file(&thumb_path);
            json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

async fn api_delete_video(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    match ctx.db.delete_video(&uid, &id) {
        Ok(Some(v)) => {
            let _ = std::fs::remove_file(ctx.db.cfg.data_dir.join("uploads").join(&v.storage_key));
            if let Some(tk) = v.thumb_key {
                let _ = std::fs::remove_file(ctx.db.cfg.data_dir.join("thumbs").join(tk));
            }
            Json(json!({ "ok": true })).into_response()
        }
        Ok(None) => json_err(StatusCode::NOT_FOUND, "Video tidak ditemukan"),
        Err(e) => json_err(StatusCode::CONFLICT, e.to_string()),
    }
}

async fn api_video_thumb(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(Some(v)) = ctx.db.get_video(Some(&uid), &id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let thumb_name = v.thumb_key.unwrap_or_else(|| format!("{}.jpg", v.id));
    let thumb_path = ctx.db.cfg.data_dir.join("thumbs").join(&thumb_name);
    if !thumb_path.exists() {
        let src_path = ctx.db.cfg.data_dir.join("uploads").join(&v.storage_key);
        if !src_path.exists()
            || !extract_thumbnail(&ctx.db.cfg.ffmpeg_bin, &src_path, &thumb_path, 0.5).await
        {
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    match tokio::fs::read(&thumb_path).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, "private, max-age=86400"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn api_video_stream(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(Some(v)) = ctx.db.get_video(Some(&uid), &id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let file_path = ctx.db.cfg.data_dir.join("uploads").join(&v.storage_key);
    let Ok(meta) = tokio::fs::metadata(&file_path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let size = meta.len();
    if size == 0 {
        return StatusCode::NOT_FOUND.into_response();
    }

    let mime = if v.storage_key.ends_with(".webm") {
        "video/webm"
    } else if v.storage_key.ends_with(".mov") {
        "video/quicktime"
    } else {
        "video/mp4"
    };

    if let Some(range_hdr) = headers.get(header::RANGE).and_then(|h| h.to_str().ok()) {
        if let Some(spec) = range_hdr.strip_prefix("bytes=") {
            let (s_str, e_str) = spec.split_once('-').unwrap_or((spec, ""));
            let start = s_str.parse::<u64>().unwrap_or(0);
            let end = if e_str.is_empty() {
                size - 1
            } else {
                e_str.parse::<u64>().unwrap_or(size - 1).min(size - 1)
            };
            if start >= size || start > end {
                return (
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    [(header::CONTENT_RANGE, format!("bytes */{size}"))],
                )
                    .into_response();
            }
            let Ok(mut f) = File::open(&file_path).await else {
                return StatusCode::NOT_FOUND.into_response();
            };
            let _ = f.seek(SeekFrom::Start(start)).await;
            let len = end - start + 1;
            let body = Body::from_stream(ReaderStream::new(f.take(len)));
            return (
                StatusCode::PARTIAL_CONTENT,
                [
                    (header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}")),
                    (header::ACCEPT_RANGES, "bytes".to_string()),
                    (header::CONTENT_LENGTH, len.to_string()),
                    (header::CONTENT_TYPE, mime.to_string()),
                ],
                body,
            )
                .into_response();
        }
    }

    let Ok(f) = File::open(&file_path).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let body = Body::from_stream(ReaderStream::new(f));
    (
        [
            (header::ACCEPT_RANGES, "bytes".to_string()),
            (header::CONTENT_LENGTH, size.to_string()),
            (header::CONTENT_TYPE, mime.to_string()),
        ],
        body,
    )
        .into_response()
}

async fn api_create_posts(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(input): Json<CreatePostsInput>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    match ctx.db.create_posts(&uid, input) {
        Ok(job_ids) => Json(json!({ "jobIds": job_ids })).into_response(),
        Err(e) => json_err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

#[derive(Deserialize)]
struct JobActionReq {
    action: String,
}

async fn api_job_action(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<JobActionReq>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    let res = match body.action.as_str() {
        "retry" => ctx.db.retry_job(&uid, &id),
        "cancel" => ctx.db.cancel_job(&uid, &id),
        _ => return json_err(StatusCode::BAD_REQUEST, "action harus retry atau cancel"),
    };
    match res {
        Ok(true) => Json(json!({ "ok": true })).into_response(),
        Ok(false) => json_err(StatusCode::CONFLICT, "Job tidak dapat diubah pada status ini"),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(Deserialize)]
struct PresetReq {
    name: String,
    hashtags: Vec<String>,
}

async fn api_save_preset(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Json(body): Json<PresetReq>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    if let Err(e) = ctx.db.save_preset(&uid, &body.name, &body.hashtags) {
        return json_err(StatusCode::BAD_REQUEST, e.to_string());
    }
    let presets = ctx.db.list_presets(&uid).unwrap_or_default();
    Json(json!({ "ok": true, "presets": presets })).into_response()
}

async fn api_delete_preset(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let Some(uid) = current_user_id(&ctx, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "unauthorized");
    };
    let _ = ctx.db.delete_preset(&uid, &id);
    let presets: Vec<Value> = ctx
        .db
        .list_presets(&uid)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| serde_json::to_value(p).ok())
        .collect();
    Json(json!({ "ok": true, "presets": presets })).into_response()
}
