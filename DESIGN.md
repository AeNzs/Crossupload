# Design Direction: CrossUpload

Reading this as: studio publikasi video lintas platform untuk kreator mandiri dan tim kecil, dengan bahasa visual utilitas ruang siar yang terstruktur, dial ENERGY 1 / RHYTHM 3 / MOTION 1.

Dial: ENERGY 1 / RHYTHM 3 / MOTION 1

## Identitas & Motif

- **Wordmark**: Nama produk `CrossUpload` ditampilkan sebagai tipografi tegas tanpa ikon poligon asumsi dan tanpa lencana dekoratif teknologi.
- **Motif identitas**: Garis penanda vertikal setebal `3px` berwarna amber (`--accent`) seperti jarum *playhead* editor video, diulang secara konsisten pada penanda halaman aktif dan kepala tahapan kerja.
- **Tema bawaan & pilihan**: Gelap dipilih sebagai bawaan karena pengguna meninjau pratinjau video vertikal berlatar gelap, disertai sakelar terang/gelap penuh yang menjaga kontras WCAG AA di kedua mode.

## Palet Terukur (2 Netral + 1 Primer + 1 Aksen)

- **Permukaan netral**: Kanvas dasar (`#0b0e14` gelap / `#f3f5f8` terang) dan bidang kerja (`#121721` gelap / `#ffffff` terang) dengan batas kontrol interaktif minimal kontras 3:1 (`#64748b`).
- **Primer fungsional**: Biru kobalt tua `#1d4ed8` dengan teks putih `#ffffff` (rasio kontras `6.70:1`, lulus WCAG AA untuk seluruh ukuran teks tombol).
- **Satu aksen sengaja**: Amber ruang siar (`#f59e0b` gelap / `#b45309` terang) dipakai khusus pada motif *playhead* dan titik status kredensial, bukan disebar ke seluruh elemen.
- **Warna status**: Hijau, kuning tua, dan merah tua hanya muncul saat merepresentasikan status nyata antrean atau validasi, masing-masing lulus rasio >= 4.5:1 terhadap latarnya.

## Alasan Satu Baris untuk Keputusan Utama (R-31)

- **Mengapa palet ini**: Netral gelap menjaga fokus mata pada warna asli video pratinjau, sedangkan biru `#1d4ed8` menjamin kontras teks tombol utama `6.70:1` di kedua tema.
- **Mengapa tata letak RHYTHM 3**: Setiap bagian memiliki bentuk berbeda sesuai tugasnya (panel monitor lengket di kiri untuk video, kisi tombol terbuka di atas kanvas untuk tujuan, kartu solid untuk editor teks, bilah aksi bawah untuk jadwal, dan daftar baris kronologis tanpa kartu statistik kembar di halaman Antrean).
- **Mengapa tipografi ini**: Sistem *sans-serif* antarmuka (`Segoe UI` / sistem operasi) memuat instan tanpa unduhan font eksternal dan menggunakan skala fluida `clamp()` agar tetap nyaman dibaca dari layar 360px hingga monitor lebar.
- **Mengapa skala spasi ini**: Jarak antarbagian dibedakan antara pengelompokan rapat di dalam kontrol (`8px`–`12px`) dan jeda struktural antar-tahap (`24px`–`32px`) agar urutan kerja terbaca tanpa kotak pembungkus berulang.
- **Mengapa kartu dibatasi**: Latar kartu solid hanya dipakai pada area formulir konten dan baris antrean, sementara bayangan (`box-shadow`) dibatasi khusus pada panel melayang (drawer kredensial dan notifikasi toast).
- **Mengapa ikon dibatasi**: Hanya ikon merek platform (YouTube, TikTok, Instagram, Facebook) dan ikon kontrol utilitas pada bilah atas yang dipertahankan untuk pengenalan cepat tanpa satu pun emoji dekoratif atau panah hiasan.
