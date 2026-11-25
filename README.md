# CrossUpload

Aplikasi web *single-binary* berbasis Rust untuk mengunggah satu video sekali lalu mempublikasikan atau menjadwalkannya secara otomatis ke YouTube Shorts, TikTok, Instagram Reels, dan Facebook Reels.

Seluruh komponen (server web `Axum`, worker penjadwal latar belakang `Tokio`, database `SQLite` terintegrasi, enkripsi kredensial `AES-256-GCM`, serta antarmuka web) dikemas menjadi satu file executable ringan (~7 MB) tanpa memerlukan Node.js, Python, Docker, atau Redis.

---

## Daftar isi
1. [Fitur utama](#fitur-utama)
2. [Prasyarat sistem](#prasyarat-sistem)
3. [Instalasi dan menjalankan aplikasi](#instalasi-dan-menjalankan-aplikasi)
4. [Panduan setup kredensial platform (OAuth)](#panduan-setup-kredensial-platform-oauth)
5. [Cara penggunaan](#cara-penggunaan)
6. [Referensi konfigurasi `.env`](#referensi-konfigurasi-env)
7. [Deploy di VPS Linux (`systemd`)](#deploy-di-vps-linux-systemd)
8. [Troubleshooting](#troubleshooting)

---

## Fitur utama

- Satu file executable mandiri menjalankan antarmuka web sekaligus worker penjadwal di latar belakang.
- Halaman Upload (`/`) menyediakan area unggah video *drag and drop*, pemutar pratinjau (`HTTP 206 Range`), ekstraksi thumbnail otomatis, pilihan akun tujuan dengan peringatan dini batas durasi, kustomisasi judul dan caption per platform beserta penghitung karakter, preset hashtag tersimpan, serta pilihan jadwal cepat.
- Halaman Antrean (`/queue`) memperbarui status pengiriman setiap 5 detik, menyediakan filter status, menampilkan penjelasan error yang jelas beserta tombol ulangi atau batalkan, dan menautkan hasil video yang sudah terbit.
- Panel Kredensial pada tombol ikon kunci di pojok kanan atas menyimpan Client ID dan Client Secret secara terenkripsi (`AES-256-GCM`) di SQLite tanpa perlu mengedit `.env` berulang kali, menyembunyikan nilai rahasia di browser (hanya menampilkan 4 karakter terakhir), serta menyediakan tombol salin Redirect URI dan tes koneksi.
- Worker otomatis menormalisasi video ke format vertikal `1080x1920` (9:16, MP4 H.264/AAC) memakai FFmpeg sebelum mengirimkannya ke tiap platform.
- Seluruh tabel database sudah mengikat data ke kolom `user_id` sehingga siap dikembangkan ke mode multi-pengguna.

---

## Prasyarat sistem

Pastikan dua komponen berikut sudah terpasang di perangkat Anda sebelum mulai membangun kode sumber:

### 1. Rust dan Cargo (minimal Rust 1.80+)
- Windows (PowerShell):
  ```powershell
  winget install Rustlang.Rustup
  ```
  > Catatan Windows: Toolchain Rust di Windows membutuhkan Visual Studio C++ Build Tools (*Desktop development with C++*). Jika belum terpasang, unduh melalui [rustup.rs](https://rustup.rs/) atau installer Visual Studio Build Tools.
- Linux / macOS:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- Verifikasi instalasi:
  ```bash
  rustc --version
  cargo --version
  ```

### 2. FFmpeg dan FFprobe
Dibutuhkan untuk membaca metadata durasi dan resolusi, membuat thumbnail otomatis, serta menormalisasi video 9:16.
- Windows (PowerShell):
  ```powershell
  winget install Gyan.FFmpeg
  ```
  *(Tutup lalu buka kembali terminal setelah instalasi agar `ffmpeg` terbaca di `PATH`).*
- Ubuntu / Debian:
  ```bash
  sudo apt update && sudo apt install -y ffmpeg build-essential pkg-config
  ```
- macOS (Homebrew):
  ```bash
  brew install ffmpeg
  ```
- Verifikasi instalasi:
  ```bash
  ffmpeg -version
  ffprobe -version
  ```

---

## Instalasi dan menjalankan aplikasi

### Langkah 1: Clone repository
```bash
git clone https://github.com/AeNzs/Crossupload.git
cd Crossupload
```

### Langkah 2: Buat dan atur file `.env`
Salin contoh konfigurasi `.env.example` menjadi `.env`, lalu isi `APP_PASSWORD`, `AUTH_SECRET`, dan `TOKEN_ENCRYPTION_KEY` (wajib 64 karakter hex atau 32 byte).

#### Cara cepat di Windows (PowerShell)
Jalankan perintah berikut untuk membuat file `.env` lengkap dengan kunci enkripsi acak otomatis (ganti `ALLOWED_EMAIL` dan `APP_PASSWORD` sesuai kebutuhan Anda):

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

#### Cara cepat di Linux / macOS (Bash)
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

### Langkah 3: Kompilasi binary rilis
```bash
cargo build --release
```
Kompilasi pertama mengunduh dependency dan menghasilkan satu file binary di:
- Windows: `target\release\crossupload.exe`
- Linux / macOS: `target/release/crossupload`

### Langkah 4: Jalankan aplikasi
- Windows:
  ```powershell
  .\target\release\crossupload.exe
  ```
- Linux / macOS:
  ```bash
  ./target/release/crossupload
  ```

Buka browser ke `http://localhost:3100`, lalu masuk menggunakan `ALLOWED_EMAIL` dan `APP_PASSWORD` dari `.env`. Folder `data/` beserta database SQLite (`data/crossupload.db`) dibuat otomatis saat aplikasi pertama kali berjalan.

---

## Panduan setup kredensial platform (OAuth)

Anda tidak perlu menulis kredensial API ke `.env`. Setelah masuk ke studio web:
1. Klik tombol `Kredensial` (ikon kunci) di pojok kanan atas header.
2. Pilih tab provider, klik tombol `Salin` pada kotak Redirect URI, lalu daftarkan alamat tersebut di portal developer terkait:

### 1. YouTube (Google Cloud Console)
1. Buka [Google Cloud Console](https://console.cloud.google.com/) dan buat project baru.
2. Masuk ke menu APIs & Services > Library, cari YouTube Data API v3, lalu klik Enable.
3. Masuk ke menu APIs & Services > OAuth consent screen:
   - Pilih tipe *External*, isi nama aplikasi dan email kontak.
   - Tambahkan scope `.../auth/youtube.upload` dan `.../auth/youtube.readonly`.
   - Pada bagian Test users, tambahkan alamat email akun Google/YouTube Anda (wajib selama status aplikasi masih *Testing*).
4. Masuk ke menu APIs & Services > Credentials > Create Credentials > OAuth client ID:
   - Pilih Application type: Web application.
   - Pada Authorized redirect URIs, tempel Redirect URI dari panel Kredensial:
     `http://localhost:3100/api/oauth/youtube/callback`
5. Salin Client ID dan Client Secret ke tab YouTube di panel Kredensial CrossUpload, klik Simpan Kredensial, lalu klik Hubungkan via OAuth.

### 2. TikTok (TikTok for Developers)
1. Buka [TikTok for Developers](https://developers.tiktok.com/) dan buat aplikasi baru.
2. Tambahkan produk Login Kit dan Content Posting API (aktifkan *Direct Post*).
3. Aktifkan scope `user.info.basic`, `video.upload`, dan `video.publish`.
4. Masukkan Redirect URI dari panel Kredensial:
   `https://<domain-anda>/api/oauth/tiktok/callback`
   > Catatan HTTPS TikTok: Portal TikTok umumnya menolak `http://localhost`. Untuk pengujian lokal, jalankan tunnel seperti Cloudflare Tunnel (`cloudflared tunnel --url http://localhost:3100`) atau ngrok, ubah `APP_URL` di `.env` mengikuti alamat `https://...` dari tunnel tersebut, lalu nyalakan ulang `crossupload`.
5. Salin Client Key dan Client Secret ke tab TikTok, klik Simpan Kredensial, lalu klik Hubungkan via OAuth.

### 3. Meta: Facebook Reels dan Instagram Reels
1. Siapkan Facebook Page, lalu ubah akun Instagram Anda ke tipe Professional (Business atau Creator) dan tautkan ke Facebook Page tersebut.
2. Buka [Meta for Developers](https://developers.facebook.com/) dan buat aplikasi bertipe Business.
3. Tambahkan produk Facebook Login for Business, lalu pada Valid OAuth Redirect URIs masukkan:
   `http://localhost:3100/api/oauth/meta/callback`
4. Pastikan izin aplikasi mencakup: `pages_show_list`, `pages_read_engagement`, `pages_manage_posts`, `instagram_basic`, `instagram_content_publish`, `business_management`.
5. Salin App ID dan App Secret (dari menu App Settings > Basic) ke tab Meta (FB/IG) di CrossUpload, klik Simpan Kredensial, klik Tes Koneksi, lalu klik Hubungkan via OAuth. Satu kali login langsung menautkan Facebook Page dan akun Instagram yang terhubung.

---

## Cara penggunaan

1. Buka menu Upload, tarik file video (`.mp4`, `.mov`, `.webm`, `.mkv`) ke area unggah, atau pilih video yang sudah ada di daftar pustaka sebelah kiri.
2. Klik akun platform yang ingin dituju (bisa lebih dari satu akun sekaligus). Peringatan otomatis muncul jika durasi video melebihi batas durasi platform terpilih.
3. Isi judul, caption, dan hashtag pada tab default. Jika platform tertentu membutuhkan teks berbeda, buka tab platform tersebut untuk menulis override khusus, atau simpan kumpulan hashtag ke dalam preset agar mudah dipakai ulang.
4. Pilih Kirim Sekarang untuk langsung memproses antrean, atau pilih Jadwalkan untuk menentukan waktu terbit berikutnya.
5. Buka menu Antrean untuk memantau progres konversi dan unggah, membuka tautan postingan yang sudah terbit, atau mengulangi pengiriman yang gagal.

---

## Referensi konfigurasi `.env`

| Variabel | Wajib | Default | Keterangan |
|---|---|---|---|
| `APP_URL` | Tidak | `http://localhost:3100` | URL dasar aplikasi untuk membentuk Redirect URI OAuth dan port bawaan. |
| `PORT` | Tidak | Dari `APP_URL` / `3100` | Port HTTP tempat server menerima koneksi. |
| `ALLOWED_EMAIL` | Tidak | `owner@local` | Email login yang diizinkan masuk ke studio. |
| `APP_PASSWORD` | Ya | (kosong) | Password login ke dashboard web. |
| `AUTH_SECRET` | Ya | (kosong) | Kunci rahasia HMAC-SHA256 untuk menandatangani cookie sesi dan state OAuth. |
| `TOKEN_ENCRYPTION_KEY` | Ya | (kosong) | Kunci enkripsi AES-256-GCM (wajib tepat 64 karakter hex / 32 byte). |
| `YOUTUBE_PRIVACY` | Tidak | `private` | Privasi upload YouTube (`private`, `unlisted`, atau `public` setelah lolos audit). |
| `TIKTOK_PRIVACY` | Tidak | `SELF_ONLY` | Privasi upload TikTok (`SELF_ONLY`, `MUTUAL_FOLLOW_FRIENDS`, atau `PUBLIC_TO_EVERYONE` setelah lolos audit). |
| `DATA_DIR` | Tidak | `./data` | Folder penyimpanan database, file video sumber, dan thumbnail. |
| `DATABASE_URL` | Tidak | `./data/crossupload.db` | Path file database SQLite. |
| `FFMPEG_PATH` | Tidak | `ffmpeg` | Path lengkap ke binary `ffmpeg` bila belum terdaftar di `PATH` sistem. |
| `FFPROBE_PATH` | Tidak | `ffprobe` | Path lengkap ke binary `ffprobe` bila belum terdaftar di `PATH` sistem. |

---

## Deploy di VPS Linux (`systemd`)

Karena aplikasi hanya berupa satu binary mandiri, pemasangan di VPS Linux cukup menyalin binary rilis dan file `.env`:

1. Salin `target/release/crossupload` ke `/opt/crossupload/crossupload` bersama `/opt/crossupload/.env`.
2. Buat unit `/etc/systemd/system/crossupload.service`:
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
3. Aktifkan service dan arahkan reverse proxy HTTPS (seperti Caddy atau Nginx) ke port lokal aplikasi:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now crossupload
   ```

---

## Troubleshooting

- Tombol "Hubungkan via OAuth" belum muncul di panel Kredensial: klik tombol biru Simpan Kredensial terlebih dahulu setelah mengisi Client ID dan Secret hingga status berubah menjadi Siap dihubungkan.
- Pesan `TOKEN_ENCRYPTION_KEY harus berupa 64 karakter hex` saat aplikasi dijalankan: pastikan nilai di `.env` berisi tepat 64 karakter heksadesimal (`0-9`, `a-f`) menggunakan skrip PowerShell atau `openssl rand -hex 32` pada panduan instalasi di atas.
- Pesan `Gagal menjalankan ffmpeg / ffprobe` saat mengunggah video: pastikan FFmpeg terpasang dan terminal sudah dibuka ulang, atau tulis path lengkap pada `FFMPEG_PATH` dan `FFPROBE_PATH` di `.env` (contoh: `FFMPEG_PATH=C:\ffmpeg\bin\ffmpeg.exe`).
- Pesan `redirect_uri_mismatch` saat login OAuth: samakan alamat di bilah browser dengan `APP_URL` di `.env`, lalu pastikan Redirect URI dari panel Kredensial sudah tersimpan di portal developer platform tersebut.
- Video terunggah sebagai Private atau Hanya Saya: sebelum aplikasi lolos proses *App Review* resmi Google dan TikTok, API platform membatasi unggahan ke mode privat. Setelah lolos audit, ubah `YOUTUBE_PRIVACY=public` dan `TIKTOK_PRIVACY=PUBLIC_TO_EVERYONE` di `.env`.
