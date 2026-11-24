# CrossUpload (Rust Single-Binary)

Studio web mandiri (**1 file `.exe`**, tanpa Node.js / `node_modules`) untuk mengunggah satu video sekali lalu mempublikasikan atau menjadwalkannya ke **YouTube Shorts**, **TikTok**, **Instagram Reels**, dan **Facebook Reels**.

## Keunggulan Arsitektur Rust
- **Single-binary (`crossupload.exe`)**: server web (`Axum`), worker penjadwal latar belakang (`Tokio`), SQLite terintegrasi (`rusqlite` bundled), enkripsi **AES-256-GCM**, serta seluruh aset UI web (`HTML/CSS/JS`) dibundle langsung ke dalam satu executable ringan.
- **Streaming hemat memori**: upload video berukuran besar langsung di-stream ke disk tanpa memakan RAM, dilengkapi pemutar preview HTTP `206 Partial Content` (`Range`) dan ekstraksi thumbnail otomatis via FFmpeg.
- **Dua ruang kerja terpisah + Panel Kredensial ikon kunci**:
  - **Upload (`/`)**: dropzone drag & drop, pustaka video + preview, kartu platform berwarna, peringatan dini batas durasi, override judul/caption per platform + penghitung karakter, preset hashtag tersimpan, dan penjadwal cepat.
  - **Antrean (`/queue`)**: pantau status live setiap 5 detik, filter status, pesan error ramah manusia, tautan post terbit, serta tombol **Ulangi** dan **Batalkan**.
  - **Kredensial (ikon kunci di header & `/credentials`)**: simpan Client ID & Client Secret terenkripsi di database lokal tanpa perlu edit `.env`, salin Redirect URI sekali klik, tes koneksi, dan hubungkan/putuskan akun OAuth.
- **Siap multiuser**: seluruh tabel (`users`, `credentials`, `accounts`, `videos`, `posts`, `jobs`, `hashtag_presets`) sudah memakai `user_id`.

## Cara Menjalankan

```powershell
# Mode rilis (direkomendasikan):
cargo build --release
.\target\release\crossupload.exe

# Atau langsung lewat Cargo:
cargo run --release
```

- Buka alamat di `APP_URL` pada file `.env` (default `http://localhost:3100`, atau ubah lewat variabel `PORT` / `APP_URL`).
- Login menggunakan `ALLOWED_EMAIL` dan `APP_PASSWORD` yang tertera di `.env`.
- Pastikan `ffmpeg` dan `ffprobe` tersedia di `PATH` (atau atur `FFMPEG_PATH` / `FFPROBE_PATH` di `.env`).

## Menghubungkan Platform (Lewat Ikon Kunci)

1. Klik tombol **Kredensial** (ikon kunci) di kanan atas header.
2. Pilih tab **YouTube**, **TikTok**, atau **Meta (FB/IG)**, klik **Salin** pada kotak **Redirect URI**, dan daftarkan di portal developer terkait:
   - **YouTube**: Google Cloud Console → aktifkan *YouTube Data API v3* → buat OAuth 2.0 Client tipe *Web application*.
   - **TikTok**: TikTok for Developers → aktifkan *Login Kit* & *Content Posting API* (`user.info.basic`, `video.upload`, `video.publish`). Untuk uji lokal dengan syarat HTTPS, gunakan tunnel (mis. Cloudflare Tunnel / ngrok) dan sesuaikan `APP_URL`.
   - **Meta**: Meta for Developers → aplikasi tipe *Business* + *Facebook Login*. Akun Instagram wajib *Business/Creator* dan tertaut ke Facebook Page.
3. Isi Client ID & Secret → klik **Simpan Kredensial** → klik **Tes Koneksi** atau **Hubungkan via OAuth**.

## Struktur Kode
- `src/main.rs` — inisialisasi konfigurasi, database, worker latar belakang, dan server HTTP Axum.
- `src/security.rs` — enkripsi `AES-256-GCM`, tanda tangan `HMAC-SHA256` sesi & state OAuth, masking secret.
- `src/db.rs` — skema otomatis & repositori SQLite (`WAL` mode) berbasis `user_id`.
- `src/media.rs` — integrasi `ffprobe`, ekstraksi thumbnail JPEG, dan normalisasi video 9:16 `1080x1920` H.264/AAC.
- `src/adapters/` — OAuth, refresh token, verifikasi akun, dan streaming upload ke YouTube, TikTok, Facebook, dan Instagram.
- `src/worker.rs` — loop antrean & penjadwal setiap 5 detik dengan retry bertahap (maks 3x).
- `src/routes.rs` — seluruh endpoint HTTP & penyajian aset ter-bundle.
- `src/ui/` — antarmuka web modern (gelap/terang) yang dikompilasi masuk ke binary via `include_str!`.
