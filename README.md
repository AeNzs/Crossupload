# CrossUpload

Aplikasi web **single-binary berbasis Rust** untuk mengunggah satu video sekali lalu mempublikasikan atau menjadwalkannya secara otomatis ke **YouTube Shorts**, **TikTok**, **Instagram Reels**, dan **Facebook Reels**.

Seluruh komponen—server web (`Axum`), worker penjadwal latar belakang (`Tokio`), database `SQLite` terintegrasi, enkripsi kredensial `AES-256-GCM`, serta antarmuka web modern—dikemas menjadi **1 file executable ringan (~7 MB)** tanpa memerlukan Node.js, Python, Docker, atau Redis.

---

## Daftar Isi
1. [Fitur Utama](#fitur-utama)
2. [Prasyarat Sistem](#prasyarat-sistem)
3. [Instalasi & Menjalankan Aplikasi](#instalasi--menjalankan-aplikasi)
4. [Panduan Setup Kredensial Platform (OAuth)](#panduan-setup-kredensial-platform-oauth)
5. [Cara Penggunaan](#cara-penggunaan)
6. [Referensi Konfigurasi `.env`](#referensi-konfigurasi-env)
7. [Deploy di VPS Linux (`systemd`)](#deploy-di-vps-linux-systemd)
8. [Troubleshooting](#troubleshooting)

---

## Fitur Utama

- **1 File Executable Mandiri**: Jalankan cukup satu binary untuk menyalakan dashboard web sekaligus worker penjadwal di latar belakang.
- **Ruang Kerja Terpisah & Rapi**:
  - **Upload (`/`)**: Dropzone video *drag & drop*, preview video player (`HTTP 206 Range`), ekstraksi thumbnail otomatis, kartu pilihan platform, peringatan dini batas durasi, kustomisasi judul/caption per platform beserta penghitung karakter, preset hashtag tersimpan, dan tombol jadwal cepat.
  - **Antrean (`/queue`)**: Pemantauan status *real-time* tiap 5 detik, filter status, terjemahan pesan error yang mudah dipahami, tautan ke postingan yang sudah terbit, serta tombol **Ulangi** / **Batalkan**.
- **Panel Kredensial Terpusat (Tombol Ikon Kunci)**:
  - Kelola **Client ID & Client Secret** langsung dari UI tanpa perlu mengedit `.env` berulang kali.
  - Disimpan terenkripsi **AES-256-GCM** di SQLite; secret tidak pernah dikirim utuh kembali ke browser (hanya menampilkan mask 4 karakter terakhir).
  - Dilengkapi kotak salin **Redirect URI**, tombol **Tes Koneksi**, serta **Hubungkan / Putuskan Akun OAuth**.
- **Normalisasi Video Otomatis**: Worker otomatis mengonversi video ke format standar vertikal `1080×1920` (9:16, MP4 H.264/AAC) menggunakan FFmpeg sebelum dikirim ke tiap platform.
- **Siap Dikembangkan ke Multiuser**: Seluruh tabel database sudah terikat ke kolom `user_id`.

---

## Prasyarat Sistem

Sebelum menginstal, pastikan dua komponen berikut sudah terpasang di perangkat Anda:

### 1. Rust & Cargo (minimal Rust 1.80+)
- **Windows (PowerShell)**:
  ```powershell
  winget install Rustlang.Rustup
  ```
  > **Catatan Windows**: Toolchain Rust di Windows membutuhkan **Visual Studio C++ Build Tools** (*Desktop development with C++*). Jika belum ada, unduh melalui [rustup.rs](https://rustup.rs/) atau installer Visual Studio Build Tools.
- **Linux / macOS**:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- Verifikasi instalasi:
  ```bash
  rustc --version
  cargo --version
  ```

### 2. FFmpeg & FFprobe
Dibutuhkan untuk membaca metadata durasi/resolusi, membuat thumbnail otomatis, dan normalisasi video 9:16.
- **Windows (PowerShell)**:
  ```powershell
  winget install Gyan.FFmpeg
  ```
  *(Tutup lalu buka kembali terminal setelah instalasi agar `ffmpeg` masuk ke `PATH`).*
- **Ubuntu / Debian**:
  ```bash
  sudo apt update && sudo apt install -y ffmpeg build-essential pkg-config
  ```
- **macOS (Homebrew)**:
  ```bash
  brew install ffmpeg
  ```
- Verifikasi instalasi:
  ```bash
  ffmpeg -version
  ffprobe -version
  ```

---

## Instalasi & Menjalankan Aplikasi

### Langkah 1 — Clone Repository
```bash
git clone https://github.com/AeNzs/Crossupload.git
cd Crossupload
```

### Langkah 2 — Buat & Atur File `.env`
Salin contoh konfigurasi `.env.example` menjadi `.env`, lalu isi `APP_PASSWORD`, `AUTH_SECRET`, dan `TOKEN_ENCRYPTION_KEY` (wajib **64 karakter hex** / 32 byte).

#### Cara Cepat di Windows (PowerShell)
Jalankan perintah berikut untuk membuat file `.env` lengkap dengan kunci enkripsi acak otomatis (ganti `ALLOWED_EMAIL` dan `APP_PASSWORD` sesuai keinginan Anda):

```powershell
$auth = -join ((1..32) | ForEach-Object { '{0:x2}' -f (Get-Random -Maximum 256) })
$enc  = -join ((1..32) | ForEach-Object { '{0:x2}' -f (Get-Random -Maximum 256) })
@"
APP_URL=http://localhost:3100
ALLOWED_EMAIL=admin@local
APP_PASSWORD=ganti-password-kuat-anda
AUTH_SECRET=$auth
TOKEN_ENCRYPTION_KEY=$enc
YOUTUBE_PRIVACY=private
TIKTOK_PRIVACY=SELF_ONLY
"@ | Set-Content .env -Encoding UTF8
```

#### Cara Cepat di Linux / macOS (Bash)
```bash
cat <<EOF > .env
APP_URL=http://localhost:3100
ALLOWED_EMAIL=admin@local
APP_PASSWORD=ganti-password-kuat-anda
AUTH_SECRET=$(openssl rand -hex 32)
TOKEN_ENCRYPTION_KEY=$(openssl rand -hex 32)
YOUTUBE_PRIVACY=private
TIKTOK_PRIVACY=SELF_ONLY
EOF
```

### Langkah 3 — Kompilasi Binary Rilis
```bash
cargo build --release
```
Kompilasi pertama akan mengunduh dependency dan menghasilkan 1 file binary di:
- **Windows**: `target\release\crossupload.exe`
- **Linux / macOS**: `target/release/crossupload`

### Langkah 4 — Jalankan Aplikasi
- **Windows**:
  ```powershell
  .\target\release\crossupload.exe
  ```
- **Linux / macOS**:
  ```bash
  ./target/release/crossupload
  ```

Buka browser ke **`http://localhost:3100`**, lalu masuk menggunakan `ALLOWED_EMAIL` dan `APP_PASSWORD` yang Anda atur di `.env`. Folder `data/` beserta database SQLite (`data/crossupload.db`) akan dibuat secara otomatis.

---

## Panduan Setup Kredensial Platform (OAuth)

Anda **tidak perlu** memasukkan kredensial API ke `.env`. Setelah login ke web studio:
1. Klik tombol **`Kredensial` (ikon kunci)** di pojok kanan atas header.
2. Pilih tab provider, klik tombol **`Salin`** pada kotak **Redirect URI**, dan daftarkan di portal developer masing-masing:

### 1. YouTube (Google Cloud Console)
1. Buka [Google Cloud Console](https://console.cloud.google.com/) dan buat project baru.
2. Masuk ke **APIs & Services → Library**, cari **YouTube Data API v3**, lalu klik **Enable**.
3. Masuk ke **APIs & Services → OAuth consent screen**:
   - Pilih tipe *External*, isi nama aplikasi dan email.
   - Tambahkan scope `.../auth/youtube.upload` dan `.../auth/youtube.readonly`.
   - Pada bagian **Test users**, tambahkan alamat email akun Google/YouTube Anda (wajib selama aplikasi masih berstatus *Testing*).
4. Masuk ke **APIs & Services → Credentials → Create Credentials → OAuth client ID**:
   - Pilih Application type: **Web application**.
   - Pada **Authorized redirect URIs**, tempel Redirect URI dari panel Kredensial:
     `http://localhost:3100/api/oauth/youtube/callback`
5. Salin **Client ID** dan **Client Secret** ke tab **YouTube** di panel Kredensial CrossUpload → klik **Simpan Kredensial** → klik **Hubungkan via OAuth**.

### 2. TikTok (TikTok for Developers)
1. Buka [TikTok for Developers](https://developers.tiktok.com/) dan buat aplikasi baru.
2. Tambahkan produk **Login Kit** dan **Content Posting API** (aktifkan *Direct Post*).
3. Pastikan scope berikut diaktifkan: `user.info.basic`, `video.upload`, `video.publish`.
4. Masukkan **Redirect URI** dari panel Kredensial:
   `https://<domain-anda>/api/oauth/tiktok/callback`
   > **Catatan HTTPS TikTok**: Portal TikTok umumnya menolak `http://localhost`. Untuk pengujian di PC lokal, gunakan tunnel gratis seperti **Cloudflare Tunnel** (`cloudflared tunnel --url http://localhost:3100`) atau **ngrok**, lalu ubah `APP_URL` di `.env` mengikuti alamat `https://...` dari tunnel tersebut dan restart `crossupload`.
5. Salin **Client Key** dan **Client Secret** ke tab **TikTok** → klik **Simpan Kredensial** → klik **Hubungkan via OAuth**.

### 3. Meta: Facebook Reels & Instagram Reels
1. Pastikan Anda memiliki **Facebook Page**, dan akun Instagram Anda sudah diubah ke akun **Professional (Business / Creator)** serta ditautkan ke Facebook Page tersebut.
2. Buka [Meta for Developers](https://developers.facebook.com/), buat aplikasi bertipe **Business**.
3. Tambahkan produk **Facebook Login for Business**, lalu pada **Valid OAuth Redirect URIs** masukkan:
   `http://localhost:3100/api/oauth/meta/callback`
4. Pastikan izin aplikasi mencakup: `pages_show_list`, `pages_read_engagement`, `pages_manage_posts`, `publish_video`, `instagram_basic`, `instagram_content_publish`, `business_management`.
5. Salin **App ID** dan **App Secret** (dari menu *App Settings → Basic*) ke tab **Meta (FB/IG)** di CrossUpload → klik **Simpan Kredensial** → klik **Tes Koneksi** → klik **Hubungkan via OAuth**. Satu kali login akan menautkan Facebook Page dan Instagram sekaligus.

---

## Cara Penggunaan

1. **Unggah Video**: Buka menu **Upload**, tarik file video (`.mp4`, `.mov`, `.webm`, `.mkv`) ke kotak upload, atau pilih video yang sudah ada di **Pustaka Video**.
2. **Pilih Tujuan**: Klik kartu akun platform yang ingin dituju (bisa memilih beberapa platform sekaligus). Sistem otomatis memberi peringatan dini jika durasi video melebihi batas platform (misal >90 detik untuk IG/FB Reels atau >180 detik untuk YouTube Shorts).
3. **Isi Konten & Override**:
   - Isi **Judul**, **Caption**, dan **Hashtag** pada tab *Semua Platform (Default)*.
   - Jika ingin caption berbeda di platform tertentu, klik tab platform terkait (misal *TikTok* atau *Instagram*) untuk mengisi override khusus.
   - Klik **+ Simpan Preset** untuk menyimpan kombinasi hashtag favorit agar bisa dipakai ulang sekali klik.
4. **Kirim atau Jadwalkan**:
   - Pilih **Kirim Sekarang** untuk langsung diproses worker, atau pilih **Jadwalkan** (tersedia tombol cepat *+1 Jam*, *Malam Ini 19:00*, *Besok Pagi 08:00*, *Besok Malam 19:00*).
5. **Pantau Status**: Buka menu **Antrean** untuk melihat progres transcode & upload, membuka tautan hasil post, atau menekan **Ulangi** bila ada job yang gagal.

---

## Referensi Konfigurasi `.env`

| Variabel | Wajib | Default | Keterangan |
|---|---|---|---|
| `APP_URL` | Tidak | `http://localhost:3100` | URL dasar aplikasi (dipakai untuk membentuk Redirect URI OAuth & port default). |
| `PORT` | Tidak | Dari `APP_URL` / `3100` | Port HTTP tempat server mendengarkan koneksi. |
| `ALLOWED_EMAIL` | Tidak | `owner@local` | Email login yang diizinkan masuk ke studio. |
| `APP_PASSWORD` | **Ya** | — | Password login ke dashboard web. |
| `AUTH_SECRET` | **Ya** | — | Kunci rahasia HMAC-SHA256 untuk menandatangani cookie sesi & state OAuth. |
| `TOKEN_ENCRYPTION_KEY` | **Ya** | — | Kunci enkripsi AES-256-GCM (**wajib tepat 64 karakter hex** / 32 byte). |
| `YOUTUBE_PRIVACY` | Tidak | `private` | Status privasi upload YouTube (`private`, `unlisted`, atau `public` setelah lolos audit). |
| `TIKTOK_PRIVACY` | Tidak | `SELF_ONLY` | Privasi upload TikTok (`SELF_ONLY`, `MUTUAL_FOLLOW_FRIENDS`, atau `PUBLIC_TO_EVERYONE` setelah lolos audit). |
| `DATA_DIR` | Tidak | `./data` | Lokasi folder penyimpanan database, file video upload, dan thumbnail. |
| `DATABASE_URL` | Tidak | `./data/crossupload.db` | Lokasi file database SQLite. |
| `FFMPEG_PATH` | Tidak | `ffmpeg` | Path lengkap ke binary `ffmpeg` bila tidak ada di `PATH` sistem. |
| `FFPROBE_PATH` | Tidak | `ffprobe` | Path lengkap ke binary `ffprobe` bila tidak ada di `PATH` sistem. |

---

## Deploy di VPS Linux (`systemd`)

Karena hanya berupa satu binary mandiri, deployment di VPS Linux sangat ringan:

1. Build binary `target/release/crossupload` dan taruh di `/opt/crossupload/crossupload` bersama file `/opt/crossupload/.env`.
2. Buat file `/etc/systemd/system/crossupload.service`:
   ```ini
   [Unit]
   Description=CrossUpload Multi-Platform Video Studio
   After=network.target

   [Service]
   Type=simple
   WorkingDirectory=/opt/crossupload
   ExecStart=/opt/crossupload/crossupload
   Restart=on-failure
   RestartSec=5

   [Install]
   WantedBy=multi-user.target
   ```
3. Aktifkan service dan pasang reverse proxy HTTPS (seperti Caddy atau Nginx) ke port lokal aplikasi:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now crossupload
   ```

---

## Troubleshooting

- **Tombol "Hubungkan via OAuth" belum muncul di panel Kredensial**  
  Pastikan Anda sudah mengklik tombol biru **Simpan Kredensial** setelah mengetik Client ID & Secret hingga badge berubah menjadi **Siap dihubungkan**.
- **Error `TOKEN_ENCRYPTION_KEY harus berupa 64 karakter hex` saat dijalankan**  
  Nilai `TOKEN_ENCRYPTION_KEY` di `.env` harus berisi tepat 64 karakter heksadesimal (`0-9`, `a-f`). Gunakan perintah generator PowerShell atau `openssl rand -hex 32` pada bagian Instalasi di atas.
- **Error `Gagal menjalankan ffmpeg / ffprobe` saat upload video**  
  Pastikan `ffmpeg` dan `ffprobe` sudah terinstal. Jika sudah diinstal tetapi belum terbaca, restart terminal Anda atau isi path lengkapnya pada `FFMPEG_PATH` dan `FFPROBE_PATH` di `.env` (contoh: `FFMPEG_PATH=C:\ffmpeg\bin\ffmpeg.exe`).
- **Error `redirect_uri_mismatch` saat login OAuth**  
  Pastikan URL di address bar browser sama persis dengan `APP_URL` di `.env`, dan Redirect URI yang disalin dari panel Kredensial sudah disimpan di portal developer platform terkait.
- **Video terunggah sebagai Private / Hanya Saya**  
  Sebelum aplikasi developer lolos proses *App Review / Audit* resmi dari Google dan TikTok, API memang membatasi publikasi ke mode privat. Setelah lolos audit, ubah `YOUTUBE_PRIVACY=public` dan `TIKTOK_PRIVACY=PUBLIC_TO_EVERYONE` di `.env`.
