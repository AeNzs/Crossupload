(function () {
  const PLATFORMS = ["youtube", "tiktok", "instagram", "facebook"];
  const SPECS = {
    youtube: { label: "YouTube Shorts", short: "YouTube", provider: "youtube", maxDur: 180, titleMax: 100, capMax: 4900 },
    tiktok: { label: "TikTok", short: "TikTok", provider: "tiktok", maxDur: 600, titleMax: 150, capMax: 2200 },
    instagram: { label: "Instagram Reels", short: "Instagram", provider: "meta", maxDur: 90, titleMax: 0, capMax: 2200 },
    facebook: { label: "Facebook Reels", short: "Facebook", provider: "meta", maxDur: 90, titleMax: 255, capMax: 2000 },
  };
  const PROVIDERS = [
    {
      id: "youtube",
      title: "YouTube",
      short: "YouTube",
      sub: "Google Cloud Console · YouTube Data API v3",
      idLabel: "Google Client ID",
      idPh: "nomor-aplikasi.apps.googleusercontent.com",
      secLabel: "Google Client Secret",
      hint: "Buat OAuth 2.0 Client tipe Web Application dan aktifkan YouTube Data API v3.",
      platforms: ["youtube"],
    },
    {
      id: "tiktok",
      title: "TikTok",
      short: "TikTok",
      sub: "TikTok for Developers · Login Kit dan Content Posting API",
      idLabel: "TikTok Client Key",
      idPh: "Masukkan Client Key dari TikTok",
      secLabel: "TikTok Client Secret",
      hint: "Aktifkan scope user.info.basic, video.upload, video.publish. Redirect URI umumnya wajib HTTPS.",
      platforms: ["tiktok"],
    },
    {
      id: "meta",
      title: "Meta (Facebook dan Instagram)",
      short: "Meta (FB/IG)",
      sub: "Meta for Developers · Facebook Login dan Instagram Graph API",
      idLabel: "Meta App ID",
      idPh: "Masukkan App ID dari Meta",
      secLabel: "Meta App Secret",
      hint: "Satu koneksi OAuth menautkan Facebook Page beserta akun Instagram Business/Creator yang terhubung.",
      platforms: ["facebook", "instagram"],
    },
  ];

  const GLOBAL_VIS_OPTIONS = [
    { value: "public", label: "Publik" },
    { value: "unlisted", label: "Tidak publik / Teman" },
    { value: "private", label: "Privat" },
    { value: "draft", label: "Draft / Inbox" },
  ];
  const PLATFORM_VIS_OPTIONS = {
    youtube: [
      { value: "public", label: "Publik (semua orang)" },
      { value: "unlisted", label: "Tidak publik (hanya yang punya link)" },
      { value: "private", label: "Privat (hanya Anda)" },
    ],
    tiktok: [
      { value: "PUBLIC_TO_EVERYONE", label: "Publik (semua orang)" },
      { value: "MUTUAL_FOLLOW_FRIENDS", label: "Teman (saling mengikuti)" },
      { value: "FOLLOWER_OF_CREATOR", label: "Pengikut" },
      { value: "SELF_ONLY", label: "Privat (hanya saya)" },
      { value: "DRAFT", label: "Draft (kirim ke Inbox TikTok)" },
    ],
    facebook: [
      { value: "PUBLISHED", label: "Publik (langsung terbit)" },
      { value: "DRAFT", label: "Draft (tersimpan di Meta)" },
    ],
    instagram: [
      { value: "public", label: "Publik (Feed dan Tab Reels)" },
      { value: "reels_only", label: "Hanya Tab Reels (tanpa Grid Profil)" },
    ],
  };

  const ui = {
    route: resolveRoute(location.pathname),
    state: null,
    loadError: null,
    selectedVideoId: "",
    pickedAccounts: new Set(),
    contentTab: "default",
    title: "",
    caption: "",
    visibility: "",
    overrides: {},
    hashtagInput: "",
    showPresetForm: false,
    presetName: "",
    scheduleMode: "now",
    scheduleAt: "",
    uploadProgress: null,
    thumbUploading: false,
    hidePrivacyBanner: false,
    queueFilter: "all",
    analyticsPlatform: "all",
    analyticsSort: "recent",
    analyticsBusy: false,
    analyticsBusyJob: "",
    expandedErrors: new Set(),
    drawerOpen: false,
    credProvider: "youtube",
    credTestResult: null,
    credDrafts: {},
    showSecret: false,
  };

  function thumbUrl(videoId, thumbKey) {
    const ver = encodeURIComponent(thumbKey || videoId || "0");
    return `/api/videos/${encodeURIComponent(videoId)}/thumb?v=${ver}`;
  }

  function resolvePlatformVisCode(platform, globalVis, overrideVis) {
    const raw = (overrideVis || "").trim() || (globalVis || "").trim();
    if (!raw) {
      if (platform === "youtube") return ui.state?.privacy?.youtube || "private";
      if (platform === "tiktok") return ui.state?.privacy?.tiktok || "SELF_ONLY";
      if (platform === "facebook") return "PUBLISHED";
      return "public";
    }
    const low = raw.toLowerCase();
    const up = raw.toUpperCase();
    if (platform === "youtube") {
      if (low === "public" || low === "unlisted" || low === "private") return low;
      return "private";
    }
    if (platform === "tiktok") {
      if (["PUBLIC_TO_EVERYONE", "MUTUAL_FOLLOW_FRIENDS", "FOLLOWER_OF_CREATOR", "SELF_ONLY", "DRAFT"].includes(up)) return up;
      if (low === "public") return "PUBLIC_TO_EVERYONE";
      if (low === "unlisted") return "MUTUAL_FOLLOW_FRIENDS";
      if (low === "draft") return "DRAFT";
      return "SELF_ONLY";
    }
    if (platform === "facebook") {
      if (up === "PUBLISHED" || up === "DRAFT") return up;
      if (low === "public" || low === "unlisted") return "PUBLISHED";
      return "DRAFT";
    }
    if (platform === "instagram") {
      if (low === "public" || low === "reels_only") return low;
      return "reels_only";
    }
    return raw;
  }

  function shortVisLabel(platform, code) {
    if (!code) return "";
    const low = String(code).toLowerCase();
    const up = String(code).toUpperCase();
    if (platform === "youtube") {
      if (low === "public") return "Publik";
      if (low === "unlisted") return "Tidak publik";
      if (low === "private") return "Privat";
    }
    if (platform === "tiktok") {
      if (up === "PUBLIC_TO_EVERYONE") return "Publik";
      if (up === "MUTUAL_FOLLOW_FRIENDS") return "Teman";
      if (up === "FOLLOWER_OF_CREATOR") return "Pengikut";
      if (up === "SELF_ONLY") return "Privat";
      if (up === "DRAFT") return "Draft Inbox";
    }
    if (platform === "facebook") {
      if (up === "PUBLISHED") return "Publik";
      if (up === "DRAFT") return "Draft Meta";
    }
    if (platform === "instagram") {
      if (low === "public") return "Feed & Reels";
      if (low === "reels_only") return "Hanya Reels";
    }
    return code;
  }

  function resolveRoute(p) {
    if (p.startsWith("/queue")) return "queue";
    if (p.startsWith("/analytics")) return "analytics";
    if (p.startsWith("/credentials")) return "credentials";
    return "upload";
  }

  function fmtNum(n) {
    if (n === null || n === undefined) return "-";
    return Number(n).toLocaleString("id-ID");
  }

  function esc(s) {
    return String(s ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function fmtBytes(n) {
    if (!n || n <= 0) return "-";
    if (n < 1048576) return Math.round(n / 1024) + " KB";
    return (n / 1048576).toFixed(1) + " MB";
  }

  function fmtDur(s) {
    if (s === null || s === undefined) return "-";
    const sec = Math.max(0, Math.round(s));
    const m = Math.floor(sec / 60);
    const r = sec % 60;
    return m > 0 ? m + ":" + String(r).padStart(2, "0") : r + " dtk";
  }

  function toLocalDt(d) {
    const pad = (x) => String(x).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function platformSvg(p) {
    if (p === "youtube") {
      return `<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M23.5 6.2a3 3 0 0 0-2.1-2.1C19.5 3.5 12 3.5 12 3.5s-7.5 0-9.4.6A3 3 0 0 0 .5 6.2 31.4 31.4 0 0 0 0 12a31.4 31.4 0 0 0 .5 5.8 3 3 0 0 0 2.1 2.1c1.9.6 9.4.6 9.4.6s7.5 0 9.4-.6a3 3 0 0 0 2.1-2.1A31.4 31.4 0 0 0 24 12a31.4 31.4 0 0 0-.5-5.8ZM9.6 15.6V8.4l6.3 3.6-6.3 3.6Z"/></svg>`;
    }
    if (p === "tiktok") {
      return `<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M19.6 6.7a4.8 4.8 0 0 1-3-4.2V2h-3.4v13.7a2.9 2.9 0 1 1-2-2.8V9.4a6.3 6.3 0 1 0 5.4 6.2V9a8.1 8.1 0 0 0 4.7 1.5V7.1c-.6 0-1.2-.2-1.7-.4Z"/></svg>`;
    }
    if (p === "instagram") {
      return `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="2.5" y="2.5" width="19" height="19" rx="5"/><circle cx="12" cy="12" r="4.2"/><circle cx="17.5" cy="6.5" r="1" fill="currentColor" stroke="none"/></svg>`;
    }
    return `<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M22 12a10 10 0 1 0-11.6 9.9v-7H7.9V12h2.5V9.8c0-2.5 1.5-3.9 3.8-3.9 1.1 0 2.2.2 2.2.2v2.5h-1.3c-1.2 0-1.6.8-1.6 1.6V12h2.8l-.4 2.9h-2.4v7A10 10 0 0 0 22 12Z"/></svg>`;
  }

  function describeError(raw) {
    if (!raw) return null;
    const low = raw.toLowerCase();
    if (low.includes("invalid_grant") || low.includes("refresh token") || low.includes("token expired") || low.includes("-> 401:") || low.includes("access_token")) {
      return { summary: "Token akses kedaluwarsa atau dicabut oleh platform.", hint: "Buka panel Kredensial (ikon kunci) lalu hubungkan ulang akun ini.", reconnect: true };
    }
    if (low.includes("kredensial") || low.includes("client_id") || low.includes("client_secret")) {
      return { summary: "Kredensial API belum lengkap atau berubah.", hint: "Periksa Client ID dan Secret di panel Kredensial.", reconnect: true };
    }
    if (low.includes("melebihi batas") || low.includes("durasi")) {
      return { summary: raw, hint: "Gunakan video dengan durasi lebih pendek sesuai batas platform." };
    }
    if (low.includes("quota") || low.includes("rate_limit") || low.includes("-> 429:")) {
      return { summary: "Kuota harian API platform tercapai.", hint: "Tunggu reset kuota harian lalu klik Ulangi." };
    }
    if (low.includes("gagal menghubungi") || low.includes("timed out") || low.includes("connection")) {
      return { summary: "Gagal menghubungi server platform karena masalah jaringan.", hint: "Worker mencoba ulang otomatis dengan jeda bertahap." };
    }
    return { summary: raw.length > 160 ? raw.slice(0, 160) + "..." : raw };
  }

  let toastTimer = null;
  function notify(kind, text) {
    const box = document.getElementById("toast-box");
    box.className = `alert toast-floating ${kind === "ok" ? "alert-ok" : "alert-bad"}`;
    box.innerHTML = `
      <span>${esc(text)}</span>
      <button type="button" class="btn btn-ghost btn-sm" id="toast-close" aria-label="Tutup pemberitahuan">Tutup</button>
    `;
    document.getElementById("toast-close").onclick = () => box.classList.add("hidden");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => box.classList.add("hidden"), 6000);
  }

  function navigate(path) {
    history.pushState(null, "", path);
    ui.route = resolveRoute(path);
    render();
  }

  let lastTriggerEl = null;
  function openCredentials(provider, triggerEl) {
    lastTriggerEl = triggerEl || document.activeElement;
    if (provider) {
      ui.credProvider = provider;
      ui.credTestResult = null;
    }
    ui.drawerOpen = true;
    renderDrawer();
    document.getElementById("btn-close-creds")?.focus();
  }

  function closeCredentials() {
    ui.drawerOpen = false;
    renderDrawer();
    if (lastTriggerEl && typeof lastTriggerEl.focus === "function") {
      lastTriggerEl.focus();
    } else {
      document.getElementById("btn-open-creds")?.focus();
    }
  }

  async function refreshState() {
    try {
      const r = await fetch("/api/state", { cache: "no-store" });
      if (r.status === 401) {
        location.href = "/login";
        return;
      }
      if (!r.ok) {
        ui.loadError = "Gagal memuat data studio dari server lokal.";
        render();
        return;
      }
      ui.loadError = null;
      const prevVideoId = ui.selectedVideoId;
      ui.state = await r.json();
      if (!ui.visibility) {
        const defaultPriv =
          ui.state.privacy?.youtube?.toLowerCase() === "private" ||
          ui.state.privacy?.tiktok?.toUpperCase() === "SELF_ONLY";
        ui.visibility = defaultPriv ? "private" : "public";
      }
      if (!prevVideoId && ui.state.videos.length > 0) {
        ui.selectedVideoId = ui.state.videos[0].id;
      } else if (prevVideoId && !ui.state.videos.some((v) => v.id === prevVideoId)) {
        ui.selectedVideoId = ui.state.videos[0]?.id || "";
      }
      updateTopbar();
      // Pertahankan fokus bila pengguna sedang mengetik atau memilih di kontrol form.
      const activeTag = document.activeElement?.tagName;
      if (activeTag !== "INPUT" && activeTag !== "TEXTAREA" && activeTag !== "SELECT") {
        render();
      }
      if (ui.drawerOpen && activeTag !== "INPUT" && activeTag !== "SELECT") {
        renderDrawer();
      }
    } catch {
      ui.loadError = "Koneksi ke server lokal terputus. Pastikan crossupload sedang berjalan.";
      render();
    }
  }

  function setCurrentNav(el, isCurrent) {
    if (!el) return;
    if (isCurrent) el.setAttribute("aria-current", "page");
    else el.removeAttribute("aria-current");
  }

  function updateTopbar() {
    const isUp = ui.route === "upload";
    const isQu = ui.route === "queue";
    const isAn = ui.route === "analytics";
    setCurrentNav(document.getElementById("nav-upload"), isUp);
    setCurrentNav(document.getElementById("mnav-upload"), isUp);
    setCurrentNav(document.getElementById("nav-queue"), isQu);
    setCurrentNav(document.getElementById("mnav-queue"), isQu);
    setCurrentNav(document.getElementById("nav-analytics"), isAn);
    setCurrentNav(document.getElementById("mnav-analytics"), isAn);

    if (!ui.state) return;
    const active = ui.state.jobs.filter((j) => j.status === "queued" || j.status === "processing").length;
    const failed = ui.state.jobs.filter((j) => j.status === "failed").length;

    for (const id of ["nav-queue-badge", "mnav-queue-badge"]) {
      const badge = document.getElementById(id);
      if (!badge) continue;
      if (failed > 0) {
        badge.textContent = `${failed} gagal`;
        badge.className = "nav-count alert";
      } else if (active > 0) {
        badge.textContent = String(active);
        badge.className = "nav-count";
      } else {
        badge.className = "nav-count hidden";
      }
    }

    const dot = document.getElementById("cred-status-dot");
    const anyConn = ui.state.accounts.length > 0;
    const anyCfg = Object.values(ui.state.credentials).some((c) => c.configured);
    dot.className = "dot " + (anyConn ? "dot-ok" : anyCfg ? "dot-warn" : "dot-mut");
  }

  function parsedHashtags() {
    const seen = new Set();
    const out = [];
    for (const raw of ui.hashtagInput.split(/[\s,]+/)) {
      const clean = raw.replace(/^#/, "").trim();
      if (clean && !seen.has(clean.toLowerCase())) {
        seen.add(clean.toLowerCase());
        out.push(clean);
      }
    }
    return out;
  }

  function render() {
    updateTopbar();
    const root = document.getElementById("app-root");
    if (ui.loadError && !ui.state) {
      root.innerHTML = `
        <div style="padding:48px 0;">
          <div class="alert alert-bad" role="alert">
            <div>
              <strong>Tidak dapat memuat studio</strong>
              <div style="margin-top:4px;">${esc(ui.loadError)}</div>
            </div>
            <button type="button" id="btn-retry-load" class="btn btn-outline btn-sm">Coba lagi</button>
          </div>
        </div>`;
      document.getElementById("btn-retry-load").onclick = () => refreshState();
      return;
    }
    if (!ui.state) {
      root.innerHTML = `<div style="padding:64px 0; text-align:center;" class="mut" role="status">Memuat data studio dan daftar video...</div>`;
      return;
    }
    if (ui.route === "queue") {
      renderQueue(root);
    } else if (ui.route === "analytics") {
      renderAnalytics(root);
    } else if (ui.route === "credentials") {
      root.innerHTML = `
        <div class="page-head">
          <div>
            <h1 class="page-title">Kredensial dan Koneksi Akun</h1>
            <p class="page-sub">Kelola Client ID, Secret, dan akun OAuth yang terhubung ke studio.</p>
          </div>
        </div>
        <div style="max-width:640px; padding:24px 0 48px;" id="creds-page-slot"></div>`;
      renderCredentialsInto(document.getElementById("creds-page-slot"));
    } else {
      renderUpload(root);
    }
  }

  function renderUpload(root) {
    const st = ui.state;
    const selVid = st.videos.find((v) => v.id === ui.selectedVideoId) || null;
    const selPlats = PLATFORMS.filter((p) =>
      st.accounts.some((a) => a.platform === p && ui.pickedAccounts.has(a.id))
    );
    if (ui.contentTab !== "default" && !selPlats.includes(ui.contentTab)) {
      ui.contentTab = "default";
    }

    const durWarnings = [];
    if (selVid && selVid.durationSec != null) {
      for (const p of selPlats) {
        if (selVid.durationSec > SPECS[p].maxDur) {
          durWarnings.push(`${SPECS[p].label}: durasi video (${fmtDur(selVid.durationSec)}) melebihi batas ${SPECS[p].maxDur} dtk.`);
        }
      }
    }

    const isPrivate =
      st.privacy.youtube.toLowerCase() === "private" || st.privacy.tiktok.toUpperCase() === "SELF_ONLY";
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone || "Lokal";
    const tags = parsedHashtags();

    root.innerHTML = `
      <div class="page-head">
        <div>
          <h1 class="page-title">Studio Publikasi Video</h1>
          <p class="page-sub">Unggah satu video sumber, pilih akun tujuan, lalu terbitkan atau jadwalkan sekaligus.</p>
        </div>
        <div>
          ${st.accounts.length > 0
            ? `<span class="badge badge-ok">${st.accounts.length} akun terhubung</span>`
            : `<button type="button" id="btn-head-cred" class="btn btn-outline btn-sm">Siapkan akun di Kredensial</button>`}
        </div>
      </div>

      ${isPrivate && !ui.hidePrivacyBanner ? `
        <div class="alert alert-warn" style="margin-top:16px;">
          <div>
            <strong>Catatan privasi sandbox:</strong>
            <span> Bawaan <code>.env</code> saat ini YouTube <code>${esc(st.privacy.youtube)}</code> dan TikTok <code>${esc(st.privacy.tiktok)}</code>. Anda dapat mengganti visibilitas per postingan di bawah, namun aplikasi yang belum lolos App Review tetap dapat dibatasi privat oleh platform.</span>
          </div>
          <button type="button" id="btn-hide-priv" class="btn btn-ghost btn-sm" aria-label="Sembunyikan info mode sandbox">Tutup</button>
        </div>
      ` : ""}

      <div class="studio-layout">
        <section class="studio-media-rail" aria-labelledby="stage-video-title">
          <div class="monitor-panel">
            <div style="display:flex; justify-content:space-between; align-items:baseline; gap:8px; margin-bottom:12px;">
              <h2 id="stage-video-title" class="stage-heading">1. Sumber video</h2>
              <span class="mut" style="font-size:0.75rem;">MP4, MOV, WEBM, MKV</span>
            </div>

            <input id="file-input" type="file" accept="video/*" class="sr-only" />
            <button type="button" id="dropzone" class="dropzone" ${ui.uploadProgress !== null ? "disabled" : ""}>
              <span style="display:block; font-weight:700;">Tarik dan lepas file video ke sini, atau klik untuk memilih</span>
              <span class="mut" style="display:block; font-size:0.8rem; margin-top:4px;">Otomatis dikonversi ke 1080x1920 (9:16 H.264/AAC) saat pengiriman</span>

              ${ui.uploadProgress !== null ? `
                <div style="margin-top:14px; text-align:left;">
                  <div style="display:flex; justify-content:space-between; font-size:0.8rem;">
                    <span>Mengunggah dan mengekstrak thumbnail...</span>
                    <strong>${ui.uploadProgress}%</strong>
                  </div>
                  <div class="progress-track"><div class="progress-fill" style="width:${ui.uploadProgress}%"></div></div>
                </div>
              ` : ""}
            </button>

            ${selVid ? `
              <div class="video-preview">
                <video controls preload="metadata" width="540" height="960" poster="${esc(thumbUrl(selVid.id, selVid.thumbKey))}" src="/api/videos/${esc(selVid.id)}/stream"></video>
                <div class="video-meta-bar">
                  <div style="min-width:0;">
                    <div class="truncate" style="font-weight:600;">${esc(selVid.name)}</div>
                    <div class="mut" style="font-size:0.75rem;">
                      ${fmtDur(selVid.durationSec)} · ${selVid.width && selVid.height ? `${selVid.width}x${selVid.height} · ` : ""}${fmtBytes(selVid.sizeBytes)}
                    </div>
                  </div>
                  <span class="badge">Aktif</span>
                </div>
              </div>

              <div class="thumb-editor">
                <input id="thumb-file-input" type="file" accept="image/jpeg,image/png,image/webp" class="sr-only" />
                <div class="thumb-editor-row">
                  <img class="thumb-preview-img" width="54" height="72" src="${esc(thumbUrl(selVid.id, selVid.thumbKey))}" alt="Pratinjau thumbnail video" onerror="this.style.visibility='hidden'" />
                  <div style="min-width:0; flex:1;">
                    <div style="display:flex; align-items:center; gap:8px; flex-wrap:wrap;">
                      <strong style="font-size:0.85rem;">Thumbnail kustom</strong>
                      <span class="badge ${selVid.thumbCustom ? "badge-ok" : ""}">${selVid.thumbCustom ? "Kustom" : "Otomatis"}</span>
                    </div>
                    <div class="mut" style="font-size:0.75rem; margin-top:4px;">
                      JPG, PNG, atau WEBP (maks 2 MB setelah kompresi). Diterapkan ke YouTube, Facebook, dan Instagram HTTPS.
                    </div>
                  </div>
                </div>
                <div class="thumb-actions">
                  <button type="button" id="btn-upload-thumb" class="btn btn-outline btn-sm" ${ui.thumbUploading ? "disabled" : ""}>
                    ${ui.thumbUploading ? "Memproses gambar..." : selVid.thumbCustom ? "Ganti thumbnail" : "Unggah thumbnail"}
                  </button>
                  ${selVid.thumbCustom ? `
                    <button type="button" id="btn-reset-thumb" class="btn btn-ghost btn-sm" ${ui.thumbUploading ? "disabled" : ""}>Reset otomatis</button>
                  ` : ""}
                </div>
              </div>
            ` : ""}

            <div class="lib-section">
              <h3 class="lib-heading">Pustaka video tersimpan (${st.videos.length})</h3>
              ${st.videos.length === 0 ? `
                <p class="mut" style="margin:8px 0 0; font-size:0.8125rem;">Belum ada video tersimpan. Unggah satu file video di atas untuk memulai.</p>
              ` : `
                <ul class="lib-list">
                  ${st.videos.map((v) => {
                    const active = v.id === ui.selectedVideoId;
                    return `
                      <li class="lib-row">
                        <button type="button" class="lib-select-btn" data-select-video="${esc(v.id)}" aria-pressed="${active}">
                          <img class="thumb-sm" width="40" height="54" src="${esc(thumbUrl(v.id, v.thumbKey))}" alt="" onerror="this.style.visibility='hidden'" />
                          <span style="min-width:0; display:block;">
                            <span class="truncate" style="display:block; font-size:0.85rem; font-weight:600;">${esc(v.name)}</span>
                            <span class="mut" style="display:block; font-size:0.75rem;">${fmtDur(v.durationSec)} · ${fmtBytes(v.sizeBytes)}${v.thumbCustom ? " · Thumb kustom" : ""}</span>
                          </span>
                        </button>
                        <button type="button" class="btn btn-ghost btn-sm" data-del-video="${esc(v.id)}" data-del-name="${esc(v.name)}" aria-label="Hapus video ${esc(v.name)}">Hapus</button>
                      </li>`;
                  }).join("")}
                </ul>
              `}
            </div>
          </div>
        </section>

        <div class="studio-main-flow stack" style="gap:24px;">
          <section class="open-band" aria-labelledby="stage-dest-title">
            <div style="display:flex; justify-content:space-between; align-items:flex-start; gap:10px;">
              <div>
                <h2 id="stage-dest-title" class="stage-heading">2. Akun tujuan</h2>
                <p class="stage-sub">Pilih satu atau beberapa akun platform untuk pengiriman serentak.</p>
              </div>
              ${st.accounts.length > 0 ? `
                <button type="button" id="btn-toggle-all" class="btn btn-ghost btn-sm">
                  ${ui.pickedAccounts.size === st.accounts.length ? "Batalkan semua" : "Pilih semua"}
                </button>
              ` : ""}
            </div>

            <div class="plat-grid">
              ${PLATFORMS.map((pl) => {
                const sp = SPECS[pl];
                const accs = st.accounts.filter((a) => a.platform === pl);
                if (accs.length === 0) {
                  return `
                    <div class="plat-card empty">
                      <div style="display:flex; align-items:center; gap:10px;">
                        <span class="plat-badge">${platformSvg(pl)}</span>
                        <div>
                          <div style="font-size:0.85rem; font-weight:600;">${sp.label}</div>
                          <div class="mut" style="font-size:0.75rem;">Belum terhubung</div>
                        </div>
                      </div>
                      <button type="button" class="btn btn-outline btn-sm" data-open-cred="${sp.provider}">Hubungkan</button>
                    </div>`;
                }
                return accs.map((a) => {
                  const sel = ui.pickedAccounts.has(a.id);
                  return `
                    <button type="button" class="plat-card ${sel ? "selected" : ""}" data-toggle-acc="${esc(a.id)}" aria-pressed="${sel}">
                      <span style="display:flex; align-items:center; gap:10px; min-width:0;">
                        <span class="plat-badge">${platformSvg(pl)}</span>
                        <span style="min-width:0; display:block;">
                          <span class="truncate" style="display:block; font-size:0.85rem; font-weight:600;">${esc(a.displayName || a.externalId)}</span>
                          <span class="mut" style="display:block; font-size:0.75rem;">${sp.label} · maks ${sp.maxDur} dtk</span>
                        </span>
                      </span>
                      <span class="check-box" aria-hidden="true">${sel ? "✓" : ""}</span>
                    </button>`;
                }).join("");
              }).join("")}
            </div>

            ${durWarnings.length > 0 ? `
              <div class="alert alert-warn" role="alert" style="margin-top:14px; flex-direction:column; gap:4px;">
                <strong>Durasi video melebihi batas platform terpilih:</strong>
                ${durWarnings.map((w) => `<div>${esc(w)}</div>`).join("")}
              </div>
            ` : ""}
          </section>

          <form id="post-form" class="studio-composer stack" style="gap:20px;">
            <section class="editor-surface stack" aria-labelledby="stage-content-title" style="gap:16px;">
              <div>
                <h2 id="stage-content-title" class="stage-heading">3. Konten, visibilitas, dan override</h2>
                <p class="stage-sub">Atur judul, caption, dan visibilitas umum, atau sesuaikan khusus per platform.</p>
              </div>

              ${selPlats.length > 0 ? `
                <div class="tab-row" role="tablist" aria-label="Pilihan teks dan visibilitas per platform">
                  <button type="button" role="tab" aria-selected="${ui.contentTab === "default"}" class="tab-chip ${ui.contentTab === "default" ? "active" : ""}" data-ctab="default">
                    Semua platform (Default)
                  </button>
                  ${selPlats.map((pl) => {
                    const hasOv = !!(
                      (ui.overrides[pl]?.title || "").trim() ||
                      (ui.overrides[pl]?.caption || "").trim() ||
                      (ui.overrides[pl]?.visibility || "").trim()
                    );
                    return `
                      <button type="button" role="tab" aria-selected="${ui.contentTab === pl}" class="tab-chip ${ui.contentTab === pl ? "active" : ""}" data-ctab="${pl}">
                        ${platformSvg(pl)}
                        <span>${SPECS[pl].short}${hasOv ? " (khusus)" : ""}</span>
                      </button>`;
                  }).join("")}
                </div>
              ` : ""}

              ${ui.contentTab === "default" ? `
                <div>
                  <label class="field-label" for="inp-title">
                    <span>Judul video</span>
                    <span class="mut" id="title-counter">${ui.title.length}/100 (batas YouTube)</span>
                  </label>
                  <input id="inp-title" class="input" maxlength="150" placeholder="Contoh: Tutorial singkat setup server Rust" value="${esc(ui.title)}" />
                </div>
                <div>
                  <label class="field-label" for="inp-caption">
                    <span>Caption atau deskripsi</span>
                    <span class="mut" id="cap-counter">${ui.caption.length}/2000 karakter</span>
                  </label>
                  <textarea id="inp-caption" class="textarea" placeholder="Tulis deskripsi utama untuk seluruh platform tujuan...">${esc(ui.caption)}</textarea>
                </div>
                <div>
                  <span class="field-label">
                    <span>Visibilitas publikasi</span>
                    <span class="mut">Dapat ditimpa di tab platform</span>
                  </span>
                  <div class="seg-group" role="group" aria-label="Visibilitas default postingan">
                    ${GLOBAL_VIS_OPTIONS.map((opt) => `
                      <button type="button" class="tab-chip ${ui.visibility === opt.value ? "active" : ""}" aria-pressed="${ui.visibility === opt.value}" data-global-vis="${opt.value}">
                        ${opt.label}
                      </button>
                    `).join("")}
                  </div>
                  <div class="vis-summary">
                    ${selPlats.length > 0
                      ? `Hasil per platform: ${selPlats.map((pl) => {
                          const code = resolvePlatformVisCode(pl, ui.visibility, ui.overrides[pl]?.visibility);
                          return `${SPECS[pl].short}: ${shortVisLabel(pl, code)}`;
                        }).join(" · ")}`
                      : "Pilih akun tujuan di atas untuk melihat pemetaan visibilitas tiap platform."}
                  </div>
                </div>
              ` : `
                <div class="uri-box stack" style="gap:12px;">
                  <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
                    <strong style="font-size:0.85rem;">Pengaturan khusus ${SPECS[ui.contentTab].label}</strong>
                    <button type="button" id="btn-reset-ov" class="btn btn-ghost btn-sm">Kembali ke default</button>
                  </div>
                  ${SPECS[ui.contentTab].titleMax > 0 ? `
                    <div>
                      <label class="field-label" for="inp-ov-title">
                        <span>Judul khusus ${SPECS[ui.contentTab].short}</span>
                        <span class="mut" id="ov-title-counter">${(ui.overrides[ui.contentTab]?.title || "").length}/${SPECS[ui.contentTab].titleMax}</span>
                      </label>
                      <input id="inp-ov-title" class="input" maxlength="${SPECS[ui.contentTab].titleMax}" placeholder="${esc(ui.title || "Kosongkan untuk memakai judul default")}" value="${esc(ui.overrides[ui.contentTab]?.title || "")}" />
                    </div>
                  ` : ""}
                  <div>
                    <label class="field-label" for="inp-ov-caption">
                      <span>Caption khusus ${SPECS[ui.contentTab].short}</span>
                      <span class="mut" id="ov-cap-counter">${(ui.overrides[ui.contentTab]?.caption || "").length}/${SPECS[ui.contentTab].capMax}</span>
                    </label>
                    <textarea id="inp-ov-caption" class="textarea" maxlength="${SPECS[ui.contentTab].capMax}" placeholder="${esc(ui.caption || "Kosongkan untuk memakai caption default")}">${esc(ui.overrides[ui.contentTab]?.caption || "")}</textarea>
                  </div>
                  <div>
                    <label class="field-label" for="inp-ov-vis">
                      <span>Visibilitas khusus ${SPECS[ui.contentTab].short}</span>
                      <span class="mut">Opsional</span>
                    </label>
                    <select id="inp-ov-vis" class="select">
                      <option value="">Ikuti default (${esc(shortVisLabel(ui.contentTab, resolvePlatformVisCode(ui.contentTab, ui.visibility, "")))})</option>
                      ${(PLATFORM_VIS_OPTIONS[ui.contentTab] || []).map((opt) => `
                        <option value="${opt.value}" ${(ui.overrides[ui.contentTab]?.visibility || "") === opt.value ? "selected" : ""}>${esc(opt.label)}</option>
                      `).join("")}
                    </select>
                  </div>
                </div>
              `}

              <div style="border-top:1px solid var(--line); padding-top:14px;">
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px; margin-bottom:6px;">
                  <label class="field-label" for="inp-hashtags" style="margin-bottom:0;">
                    Hashtag (<span id="tag-count">${tags.length}</span>)
                  </label>
                  <button type="button" id="btn-toggle-preset" class="btn btn-ghost btn-sm" aria-expanded="${ui.showPresetForm}">Simpan sebagai preset</button>
                </div>
                <input id="inp-hashtags" class="input" placeholder="Contoh: #tutorial #coding #shorts" value="${esc(ui.hashtagInput)}" />

                ${ui.showPresetForm ? `
                  <div style="display:flex; flex-wrap:wrap; gap:8px; margin-top:10px;">
                    <label class="sr-only" for="inp-preset-name">Nama preset hashtag</label>
                    <input id="inp-preset-name" class="input" style="flex:1; min-width:180px;" placeholder="Nama preset (contoh: Konten Edukasi)" value="${esc(ui.presetName)}" />
                    <button type="button" id="btn-save-preset" class="btn btn-sec">Simpan preset</button>
                  </div>
                ` : ""}

                ${st.presets.length > 0 ? `
                  <div class="preset-list" aria-label="Daftar preset hashtag">
                    <span class="mut" style="font-size:0.75rem;">Preset tersimpan:</span>
                    ${st.presets.map((pr) => `
                      <span class="preset-item">
                        <button type="button" class="preset-apply" data-apply-preset="${esc(pr.id)}" title="${esc(pr.hashtags.map((t) => "#" + t).join(" "))}">+ ${esc(pr.name)}</button>
                        <button type="button" class="preset-del" data-del-preset="${esc(pr.id)}" aria-label="Hapus preset ${esc(pr.name)}">×</button>
                      </span>
                    `).join("")}
                  </div>
                ` : ""}
              </div>
            </section>

            <section class="publish-bar stack" aria-labelledby="stage-sched-title" style="gap:16px;">
              <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:12px;">
                <div>
                  <h2 id="stage-sched-title" class="stage-heading">4. Waktu publikasi</h2>
                  <p class="stage-sub">Zona waktu perangkat: ${esc(tz)}</p>
                </div>
                <div class="seg-group" role="group" aria-label="Mode waktu pengiriman">
                  <button type="button" class="tab-chip ${ui.scheduleMode === "now" ? "active" : ""}" aria-pressed="${ui.scheduleMode === "now"}" data-sched-mode="now">Kirim sekarang</button>
                  <button type="button" class="tab-chip ${ui.scheduleMode === "later" ? "active" : ""}" aria-pressed="${ui.scheduleMode === "later"}" data-sched-mode="later">Jadwalkan</button>
                </div>
              </div>

              ${ui.scheduleMode === "later" ? `
                <div class="uri-box stack" style="gap:10px;">
                  <div style="display:flex; flex-wrap:wrap; gap:8px;">
                    <button type="button" class="btn btn-sec btn-sm" data-quick-sched="plus1h">+1 jam</button>
                    <button type="button" class="btn btn-sec btn-sm" data-quick-sched="tonight">Malam ini 19:00</button>
                    <button type="button" class="btn btn-sec btn-sm" data-quick-sched="tmrMorning">Besok pagi 08:00</button>
                    <button type="button" class="btn btn-sec btn-sm" data-quick-sched="tmrNight">Besok malam 19:00</button>
                  </div>
                  <div>
                    <label class="field-label" for="inp-sched-at">Tanggal dan jam terbit</label>
                    <input id="inp-sched-at" type="datetime-local" class="input" value="${esc(ui.scheduleAt)}" required />
                  </div>
                </div>
              ` : ""}

              <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:12px;">
                <a href="/queue" id="link-to-queue" class="btn btn-ghost btn-sm">Buka halaman Antrean</a>
                <button type="submit" id="btn-submit-post" class="btn btn-lg"
                  ${!ui.selectedVideoId || ui.pickedAccounts.size === 0 || durWarnings.length > 0 ? "disabled" : ""}>
                  ${ui.scheduleMode === "later"
                    ? `Jadwalkan ke ${ui.pickedAccounts.size} akun`
                    : `Publikasikan ke ${ui.pickedAccounts.size} akun`}
                </button>
              </div>
            </section>
          </form>
        </div>
      </div>
    `;

    bindUploadEvents();
  }

  function bindUploadEvents() {
    const headCred = document.getElementById("btn-head-cred");
    if (headCred) headCred.onclick = () => openCredentials("youtube", headCred);

    const privBtn = document.getElementById("btn-hide-priv");
    if (privBtn) privBtn.onclick = () => { ui.hidePrivacyBanner = true; render(); };

    const dz = document.getElementById("dropzone");
    const fi = document.getElementById("file-input");
    dz.onclick = () => { if (ui.uploadProgress === null) fi.click(); };
    dz.ondragover = (e) => { e.preventDefault(); dz.classList.add("drag"); };
    dz.ondragleave = () => dz.classList.remove("drag");
    dz.ondrop = (e) => {
      e.preventDefault();
      dz.classList.remove("drag");
      const f = e.dataTransfer.files?.[0];
      if (f) uploadVideoFile(f);
    };
    fi.onchange = () => {
      const f = fi.files?.[0];
      if (f) uploadVideoFile(f);
    };

    const thumbBtn = document.getElementById("btn-upload-thumb");
    const thumbInp = document.getElementById("thumb-file-input");
    if (thumbBtn && thumbInp) {
      thumbBtn.onclick = () => { if (!ui.thumbUploading) thumbInp.click(); };
      thumbInp.onchange = () => {
        const f = thumbInp.files?.[0];
        if (f && ui.selectedVideoId) uploadThumbFile(ui.selectedVideoId, f);
      };
    }

    const resetThumbBtn = document.getElementById("btn-reset-thumb");
    if (resetThumbBtn) {
      resetThumbBtn.onclick = () => {
        if (ui.selectedVideoId && !ui.thumbUploading) resetThumb(ui.selectedVideoId);
      };
    }

    document.querySelectorAll("[data-select-video]").forEach((el) => {
      el.onclick = () => {
        ui.selectedVideoId = el.getAttribute("data-select-video");
        render();
      };
    });

    document.querySelectorAll("[data-del-video]").forEach((btn) => {
      btn.onclick = async () => {
        const id = btn.getAttribute("data-del-video");
        const name = btn.getAttribute("data-del-name");
        if (!confirm(`Hapus video "${name}" dari pustaka?`)) return;
        const r = await fetch(`/api/videos/${id}`, { method: "DELETE" });
        if (r.ok) {
          notify("ok", "Video dihapus dari pustaka");
          await refreshState();
          render();
        } else {
          const j = await r.json().catch(() => ({}));
          notify("bad", j.error || "Gagal menghapus video");
        }
      };
    });

    const toggleAll = document.getElementById("btn-toggle-all");
    if (toggleAll) {
      toggleAll.onclick = () => {
        if (ui.pickedAccounts.size === ui.state.accounts.length) {
          ui.pickedAccounts.clear();
        } else {
          ui.state.accounts.forEach((a) => ui.pickedAccounts.add(a.id));
        }
        render();
      };
    }

    document.querySelectorAll("[data-toggle-acc]").forEach((btn) => {
      btn.onclick = () => {
        const id = btn.getAttribute("data-toggle-acc");
        if (ui.pickedAccounts.has(id)) ui.pickedAccounts.delete(id);
        else ui.pickedAccounts.add(id);
        render();
      };
    });

    document.querySelectorAll("[data-open-cred]").forEach((btn) => {
      btn.onclick = () => openCredentials(btn.getAttribute("data-open-cred"), btn);
    });

    document.querySelectorAll("[data-ctab]").forEach((btn) => {
      btn.onclick = () => {
        ui.contentTab = btn.getAttribute("data-ctab");
        render();
      };
    });

    const inpTitle = document.getElementById("inp-title");
    if (inpTitle) {
      inpTitle.oninput = () => {
        ui.title = inpTitle.value;
        document.getElementById("title-counter").textContent = `${ui.title.length}/100 (batas YouTube)`;
      };
    }

    const inpCap = document.getElementById("inp-caption");
    if (inpCap) {
      inpCap.oninput = () => {
        ui.caption = inpCap.value;
        document.getElementById("cap-counter").textContent = `${ui.caption.length}/2000 karakter`;
      };
    }

    const resetOv = document.getElementById("btn-reset-ov");
    if (resetOv) {
      resetOv.onclick = () => {
        delete ui.overrides[ui.contentTab];
        render();
      };
    }

    document.querySelectorAll("[data-global-vis]").forEach((btn) => {
      btn.onclick = () => {
        ui.visibility = btn.getAttribute("data-global-vis");
        render();
      };
    });

    const inpOvTitle = document.getElementById("inp-ov-title");
    if (inpOvTitle) {
      inpOvTitle.oninput = () => {
        const cur = ui.overrides[ui.contentTab] || { title: "", caption: "", visibility: "" };
        cur.title = inpOvTitle.value;
        ui.overrides[ui.contentTab] = cur;
        document.getElementById("ov-title-counter").textContent = `${cur.title.length}/${SPECS[ui.contentTab].titleMax}`;
      };
    }

    const inpOvCap = document.getElementById("inp-ov-caption");
    if (inpOvCap) {
      inpOvCap.oninput = () => {
        const cur = ui.overrides[ui.contentTab] || { title: "", caption: "", visibility: "" };
        cur.caption = inpOvCap.value;
        ui.overrides[ui.contentTab] = cur;
        document.getElementById("ov-cap-counter").textContent = `${cur.caption.length}/${SPECS[ui.contentTab].capMax}`;
      };
    }

    const inpOvVis = document.getElementById("inp-ov-vis");
    if (inpOvVis) {
      inpOvVis.onchange = () => {
        const cur = ui.overrides[ui.contentTab] || { title: "", caption: "", visibility: "" };
        cur.visibility = inpOvVis.value;
        ui.overrides[ui.contentTab] = cur;
        render();
      };
    }

    const inpTags = document.getElementById("inp-hashtags");
    if (inpTags) {
      inpTags.oninput = () => {
        ui.hashtagInput = inpTags.value;
        document.getElementById("tag-count").textContent = String(parsedHashtags().length);
      };
    }

    document.getElementById("btn-toggle-preset").onclick = () => {
      ui.showPresetForm = !ui.showPresetForm;
      render();
    };

    const inpPreset = document.getElementById("inp-preset-name");
    if (inpPreset) inpPreset.oninput = () => { ui.presetName = inpPreset.value; };

    const btnSavePreset = document.getElementById("btn-save-preset");
    if (btnSavePreset) {
      btnSavePreset.onclick = async () => {
        const tags = parsedHashtags();
        if (!ui.presetName.trim() || tags.length === 0) {
          notify("bad", "Isi hashtag terlebih dahulu lalu beri nama preset");
          return;
        }
        const r = await fetch("/api/presets", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: ui.presetName.trim(), hashtags: tags }),
        });
        const j = await r.json().catch(() => ({}));
        if (r.ok) {
          ui.presetName = "";
          ui.showPresetForm = false;
          notify("ok", "Preset hashtag disimpan");
          await refreshState();
          render();
        } else {
          notify("bad", j.error || "Gagal menyimpan preset");
        }
      };
    }

    document.querySelectorAll("[data-apply-preset]").forEach((btn) => {
      btn.onclick = () => {
        const pr = ui.state.presets.find((p) => p.id === btn.getAttribute("data-apply-preset"));
        if (!pr) return;
        const merged = [...new Set([...parsedHashtags(), ...pr.hashtags])];
        ui.hashtagInput = merged.map((t) => "#" + t).join(" ");
        render();
      };
    });

    document.querySelectorAll("[data-del-preset]").forEach((btn) => {
      btn.onclick = async () => {
        await fetch(`/api/presets/${btn.getAttribute("data-del-preset")}`, { method: "DELETE" });
        await refreshState();
        render();
      };
    });

    document.querySelectorAll("[data-sched-mode]").forEach((btn) => {
      btn.onclick = () => {
        ui.scheduleMode = btn.getAttribute("data-sched-mode");
        render();
      };
    });

    document.querySelectorAll("[data-quick-sched]").forEach((btn) => {
      btn.onclick = () => {
        const kind = btn.getAttribute("data-quick-sched");
        const d = new Date();
        if (kind === "plus1h") d.setHours(d.getHours() + 1);
        else if (kind === "tonight") {
          d.setHours(19, 0, 0, 0);
          if (d.getTime() <= Date.now()) d.setDate(d.getDate() + 1);
        } else if (kind === "tmrMorning") {
          d.setDate(d.getDate() + 1);
          d.setHours(8, 0, 0, 0);
        } else {
          d.setDate(d.getDate() + 1);
          d.setHours(19, 0, 0, 0);
        }
        ui.scheduleAt = toLocalDt(d);
        render();
      };
    });

    const inpSched = document.getElementById("inp-sched-at");
    if (inpSched) inpSched.onchange = () => { ui.scheduleAt = inpSched.value; };

    const queueLink = document.getElementById("link-to-queue");
    if (queueLink) {
      queueLink.onclick = (e) => {
        e.preventDefault();
        navigate("/queue");
      };
    }

    document.getElementById("post-form").onsubmit = async (e) => {
      e.preventDefault();
      if (ui.scheduleMode === "later" && !ui.scheduleAt) {
        notify("bad", "Pilih tanggal dan waktu jadwal terlebih dahulu");
        return;
      }
      const btn = document.getElementById("btn-submit-post");
      btn.disabled = true;
      try {
        const cleanOverrides = {};
        for (const [k, v] of Object.entries(ui.overrides)) {
          if ((v.title && v.title.trim()) || (v.caption && v.caption.trim()) || (v.visibility && v.visibility.trim())) {
            cleanOverrides[k] = {
              title: v.title?.trim() || undefined,
              caption: v.caption?.trim() || undefined,
              visibility: v.visibility?.trim() || undefined,
            };
          }
        }
        const r = await fetch("/api/posts", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            videoId: ui.selectedVideoId,
            accountIds: [...ui.pickedAccounts],
            title: ui.title.trim() || undefined,
            caption: ui.caption.trim() || undefined,
            visibility: ui.visibility || undefined,
            hashtags: parsedHashtags(),
            scheduleAt: ui.scheduleMode === "later" ? new Date(ui.scheduleAt).toISOString() : null,
            overrides: cleanOverrides,
          }),
        });
        const j = await r.json().catch(() => ({}));
        if (r.ok) {
          notify("ok", `${j.jobIds.length} antrean berhasil dibuat`);
          ui.pickedAccounts.clear();
          ui.title = "";
          ui.caption = "";
          ui.overrides = {};
          ui.scheduleMode = "now";
          ui.scheduleAt = "";
          await refreshState();
          navigate("/queue");
        } else {
          notify("bad", j.error || "Gagal membuat antrean");
          btn.disabled = false;
        }
      } catch {
        notify("bad", "Gagal menghubungi server lokal");
        btn.disabled = false;
      }
    };
  }

  function uploadVideoFile(file) {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", "/api/videos");
    xhr.setRequestHeader("x-filename", encodeURIComponent(file.name));
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) {
        ui.uploadProgress = Math.round((e.loaded / e.total) * 100);
        render();
      }
    };
    xhr.onload = async () => {
      ui.uploadProgress = null;
      const j = JSON.parse(xhr.responseText || "{}");
      if (xhr.status === 200) {
        ui.selectedVideoId = j.id;
        notify("ok", `Video "${file.name}" berhasil diunggah`);
        await refreshState();
        render();
      } else {
        notify("bad", j.error || "Upload video gagal");
        render();
      }
    };
    xhr.onerror = () => {
      ui.uploadProgress = null;
      notify("bad", "Gagal mengunggah video");
      render();
    };
    ui.uploadProgress = 0;
    render();
    xhr.send(file);
  }

  async function uploadThumbFile(videoId, file) {
    ui.thumbUploading = true;
    render();
    try {
      const r = await fetch(`/api/videos/${encodeURIComponent(videoId)}/thumb`, {
        method: "POST",
        headers: { "x-filename": encodeURIComponent(file.name) },
        body: file,
      });
      const j = await r.json().catch(() => ({}));
      ui.thumbUploading = false;
      if (r.ok) {
        notify("ok", "Thumbnail kustom berhasil disimpan");
        await refreshState();
        render();
      } else {
        notify("bad", j.error || "Gagal mengunggah thumbnail");
        render();
      }
    } catch {
      ui.thumbUploading = false;
      notify("bad", "Gagal mengunggah thumbnail");
      render();
    }
  }

  async function resetThumb(videoId) {
    ui.thumbUploading = true;
    render();
    try {
      const r = await fetch(`/api/videos/${encodeURIComponent(videoId)}/thumb`, { method: "DELETE" });
      const j = await r.json().catch(() => ({}));
      ui.thumbUploading = false;
      if (r.ok) {
        notify("ok", "Thumbnail dikembalikan ke frame otomatis");
        await refreshState();
        render();
      } else {
        notify("bad", j.error || "Gagal mereset thumbnail");
        render();
      }
    } catch {
      ui.thumbUploading = false;
      notify("bad", "Gagal mereset thumbnail");
      render();
    }
  }

  function renderQueue(root) {
    const jobs = ui.state.jobs;
    const counts = {
      all: jobs.length,
      scheduled: jobs.filter((j) => j.status === "queued").length,
      processing: jobs.filter((j) => j.status === "processing").length,
      published: jobs.filter((j) => j.status === "published").length,
      failed: jobs.filter((j) => j.status === "failed").length,
    };

    const list = jobs.filter((j) => {
      if (ui.queueFilter === "scheduled") return j.status === "queued";
      if (ui.queueFilter === "processing") return j.status === "processing";
      if (ui.queueFilter === "published") return j.status === "published";
      if (ui.queueFilter === "failed") return j.status === "failed";
      return true;
    });

    const statusBadge = (j) => {
      const future = j.status === "queued" && j.scheduleAt && new Date(j.scheduleAt).getTime() > Date.now();
      if (j.status === "published") return `<span class="badge badge-ok">Terbit</span>`;
      if (j.status === "processing") return `<span class="badge">Diproses</span>`;
      if (j.status === "failed") return `<span class="badge badge-bad">Gagal</span>`;
      if (j.status === "cancelled") return `<span class="badge">Dibatalkan</span>`;
      return `<span class="badge badge-warn">${future ? "Terjadwal" : "Menunggu"}</span>`;
    };

    const filters = [
      ["all", "Semua", counts.all],
      ["scheduled", "Terjadwal / Antre", counts.scheduled],
      ["processing", "Diproses", counts.processing],
      ["published", "Selesai", counts.published],
      ["failed", "Gagal", counts.failed],
    ];

    root.innerHTML = `
      <div class="page-head">
        <div>
          <h1 class="page-title">Antrean dan Riwayat Publikasi</h1>
          <p class="page-sub">Diperbarui otomatis setiap 5 detik saat worker memproses pengiriman di latar belakang.</p>
        </div>
        <a href="/" id="btn-go-upload" class="btn">Buat upload baru</a>
      </div>

      <div class="stack" style="padding:20px 0 48px; gap:18px;">
        <div class="seg-group" role="tablist" aria-label="Filter status antrean">
          ${filters.map(([id, label, cnt]) => `
            <button type="button" role="tab" aria-selected="${ui.queueFilter === id}" class="tab-chip ${ui.queueFilter === id ? "active" : ""}" data-qfilter="${id}">
              ${label} (${cnt})
            </button>
          `).join("")}
        </div>

        ${list.length === 0 ? `
          <div class="queue-empty">
            <div style="font-weight:700;">Belum ada pengiriman pada filter ini</div>
            <p class="mut" style="font-size:0.85rem; margin:6px 0 16px;">
              ${jobs.length === 0
                ? "Unggah video di Studio Publikasi lalu pilih akun tujuan untuk membuat antrean pertama."
                : "Tidak ada item yang cocok dengan filter status yang dipilih."}
            </p>
            ${jobs.length === 0 ? `<a href="/" id="btn-empty-upload" class="btn btn-outline">Buka Studio Upload</a>` : ""}
          </div>
        ` : `
          <div class="queue-ledger">
            ${list.map((j) => {
              const sp = SPECS[j.platform] || SPECS.youtube;
              const fr = describeError(j.error);
              const showRaw = ui.expandedErrors.has(j.id);
              return `
                <article class="queue-row">
                  <div style="display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:14px;">
                    <div style="display:flex; align-items:flex-start; gap:12px; min-width:0;">
                      <img class="thumb-sm" width="40" height="54" src="${esc(thumbUrl(j.videoId, j.thumbKey))}" alt="" onerror="this.style.visibility='hidden'" />
                      <div style="min-width:0;">
                        <div style="display:flex; flex-wrap:wrap; align-items:center; gap:8px;">
                          <span class="badge">${platformSvg(j.platform)} ${sp.short}</span>
                          <span class="mut" style="font-size:0.8125rem;">${esc(j.account || "-")}</span>
                          ${statusBadge(j)}
                          ${j.visibility ? `<span class="badge">${esc(shortVisLabel(j.platform, j.visibility))}</span>` : ""}
                          ${j.attempts > 0 && j.status !== "published" ? `<span class="mut" style="font-size:0.75rem;">Percobaan ${j.attempts}/3</span>` : ""}
                        </div>
                        <h2 class="truncate" style="margin:6px 0 2px; font-size:0.95rem; font-weight:700;">${esc(j.title || j.video)}</h2>
                        <div class="mut" style="font-size:0.78rem;">
                          File: ${esc(j.video)}
                          ${j.scheduleAt ? ` · Jadwal: ${esc(new Date(j.scheduleAt).toLocaleString())}` : ""}
                          ${j.publishedAt ? ` · Terbit: ${esc(new Date(j.publishedAt).toLocaleString())}` : ""}
                        </div>
                      </div>
                    </div>

                    <div style="display:flex; align-items:center; flex-wrap:wrap; gap:8px;">
                      ${j.publishedUrl ? `<a class="btn btn-ok btn-sm" href="${esc(j.publishedUrl)}" target="_blank" rel="noreferrer">Buka postingan</a>` : ""}
                      ${j.status === "failed" || j.status === "cancelled" ? `<button type="button" class="btn btn-sm" data-job-act="retry" data-job-id="${esc(j.id)}">Ulangi</button>` : ""}
                      ${j.status === "queued" ? `<button type="button" class="btn btn-outline btn-sm" data-job-act="cancel" data-job-id="${esc(j.id)}">Batalkan</button>` : ""}
                    </div>
                  </div>

                  ${fr ? `
                    <div class="alert alert-bad" role="alert" style="margin-top:12px; flex-direction:column; align-items:stretch;">
                      <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:8px;">
                        <div>
                          <div style="font-weight:700;">${esc(fr.summary)}</div>
                          ${fr.hint ? `<div style="font-size:0.8rem; margin-top:2px;">${esc(fr.hint)}</div>` : ""}
                        </div>
                        <div style="display:flex; align-items:center; flex-wrap:wrap; gap:8px;">
                          ${fr.reconnect ? `<button type="button" class="btn btn-outline btn-sm" data-open-cred="${sp.provider}">Buka kredensial</button>` : ""}
                          ${j.error ? `<button type="button" class="btn btn-ghost btn-sm" data-toggle-raw="${esc(j.id)}" aria-expanded="${showRaw}">${showRaw ? "Tutup detail" : "Detail teknis"}</button>` : ""}
                        </div>
                      </div>
                      ${showRaw && j.error ? `<pre style="margin:8px 0 0; padding:10px; background:var(--bg); color:var(--fg); border:1px solid var(--border); border-radius:6px; overflow-x:auto; font-size:0.75rem;">${esc(j.error)}</pre>` : ""}
                    </div>
                  ` : ""}
                </article>`;
            }).join("")}
          </div>
        `}
      </div>
    `;

    document.getElementById("btn-go-upload").onclick = (e) => { e.preventDefault(); navigate("/"); };
    const emptyUp = document.getElementById("btn-empty-upload");
    if (emptyUp) emptyUp.onclick = (e) => { e.preventDefault(); navigate("/"); };

    document.querySelectorAll("[data-qfilter]").forEach((btn) => {
      btn.onclick = () => {
        ui.queueFilter = btn.getAttribute("data-qfilter");
        render();
      };
    });
    document.querySelectorAll("[data-open-cred]").forEach((btn) => {
      btn.onclick = () => openCredentials(btn.getAttribute("data-open-cred"), btn);
    });
    document.querySelectorAll("[data-toggle-raw]").forEach((btn) => {
      btn.onclick = () => {
        const id = btn.getAttribute("data-toggle-raw");
        if (ui.expandedErrors.has(id)) ui.expandedErrors.delete(id);
        else ui.expandedErrors.add(id);
        render();
      };
    });
    document.querySelectorAll("[data-job-act]").forEach((btn) => {
      btn.onclick = async () => {
        const id = btn.getAttribute("data-job-id");
        const action = btn.getAttribute("data-job-act");
        const r = await fetch(`/api/jobs/${id}`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ action }),
        });
        if (r.ok) {
          notify("ok", action === "retry" ? "Job dimasukkan ulang ke antrean" : "Job dibatalkan");
          await refreshState();
          render();
        } else {
          const j = await r.json().catch(() => ({}));
          notify("bad", j.error || "Gagal memperbarui status job");
        }
      };
    });
  }

  async function refreshMetrics(opts) {
    const singleJobId = opts?.jobId || "";
    if (singleJobId) ui.analyticsBusyJob = singleJobId;
    else ui.analyticsBusy = true;
    render();
    try {
      const r = await fetch("/api/analytics/refresh", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          jobId: singleJobId || undefined,
          platform: singleJobId ? undefined : ui.analyticsPlatform,
        }),
      });
      const j = await r.json().catch(() => ({ ok: false, message: "Gagal membaca respons server" }));
      ui.analyticsBusy = false;
      ui.analyticsBusyJob = "";
      if (r.ok) {
        notify(j.ok ? "ok" : "bad", j.message || "Performa diperbarui");
        await refreshState();
        render();
      } else {
        notify("bad", j.error || j.message || "Gagal memperbarui performa");
        render();
      }
    } catch {
      ui.analyticsBusy = false;
      ui.analyticsBusyJob = "";
      notify("bad", "Gagal menghubungi server lokal");
      render();
    }
  }

  function renderAnalytics(root) {
    const published = ui.state.jobs.filter((j) => j.status === "published");
    const counts = {
      all: published.length,
      youtube: published.filter((j) => j.platform === "youtube").length,
      tiktok: published.filter((j) => j.platform === "tiktok").length,
      instagram: published.filter((j) => j.platform === "instagram").length,
      facebook: published.filter((j) => j.platform === "facebook").length,
    };

    const filtered = published.filter((j) =>
      ui.analyticsPlatform === "all" ? true : j.platform === ui.analyticsPlatform
    );

    const sorted = [...filtered].sort((a, b) => {
      if (ui.analyticsSort === "views") return (b.views ?? -1) - (a.views ?? -1);
      if (ui.analyticsSort === "likes") return (b.likes ?? -1) - (a.likes ?? -1);
      if (ui.analyticsSort === "comments") return (b.comments ?? -1) - (a.comments ?? -1);
      const ta = new Date(a.publishedAt || a.createdAt).getTime();
      const tb = new Date(b.publishedAt || b.createdAt).getTime();
      return tb - ta;
    });

    const totalViews = filtered.reduce((acc, j) => acc + (j.views || 0), 0);
    const totalLikes = filtered.reduce((acc, j) => acc + (j.likes || 0), 0);
    const totalComments = filtered.reduce((acc, j) => acc + (j.comments || 0), 0);
    const hasAnyViews = filtered.some((j) => j.views !== null && j.views !== undefined);
    const hasAnyLikes = filtered.some((j) => j.likes !== null && j.likes !== undefined);
    const hasAnyComments = filtered.some((j) => j.comments !== null && j.comments !== undefined);

    const platFilters = [
      ["all", "Semua platform", counts.all],
      ["youtube", "YouTube", counts.youtube],
      ["tiktok", "TikTok", counts.tiktok],
      ["instagram", "Instagram", counts.instagram],
      ["facebook", "Facebook", counts.facebook],
    ];

    root.innerHTML = `
      <div class="page-head">
        <div>
          <h1 class="page-title">Performa Video</h1>
          <p class="page-sub">Cek jumlah tayangan, suka, dan komentar video yang sudah terbit di setiap platform.</p>
        </div>
        <button type="button" id="btn-refresh-perf" class="btn" ${ui.analyticsBusy || published.length === 0 ? "disabled" : ""}>
          ${ui.analyticsBusy ? "Memperbarui metrik..." : "Perbarui semua metrik"}
        </button>
      </div>

      <div class="stack" style="padding:16px 0 48px; gap:18px;">
        <section class="perf-summary" aria-label="Ringkasan performa video terbit">
          <div class="perf-sum-cell lead">
            <div class="perf-sum-lbl">Video terbit</div>
            <div class="perf-sum-val">${fmtNum(filtered.length)}</div>
          </div>
          <div class="perf-sum-cell">
            <div class="perf-sum-lbl">Total tayangan</div>
            <div class="perf-sum-val">${hasAnyViews ? fmtNum(totalViews) : "-"}</div>
          </div>
          <div class="perf-sum-cell">
            <div class="perf-sum-lbl">Total suka</div>
            <div class="perf-sum-val">${hasAnyLikes ? fmtNum(totalLikes) : "-"}</div>
          </div>
          <div class="perf-sum-cell">
            <div class="perf-sum-lbl">Total komentar</div>
            <div class="perf-sum-val">${hasAnyComments ? fmtNum(totalComments) : "-"}</div>
          </div>
        </section>

        <div class="perf-toolbar">
          <div class="seg-group" role="tablist" aria-label="Filter platform performa">
            ${platFilters.map(([id, label, cnt]) => `
              <button type="button" role="tab" aria-selected="${ui.analyticsPlatform === id}" class="tab-chip ${ui.analyticsPlatform === id ? "active" : ""}" data-pfilter="${id}">
                ${label} (${cnt})
              </button>
            `).join("")}
          </div>

          <div style="display:flex; align-items:center; gap:8px;">
            <label class="mut" for="sel-perf-sort" style="font-size:0.8rem;">Urutkan:</label>
            <select id="sel-perf-sort" class="select" style="width:auto; min-width:170px;">
              <option value="recent" ${ui.analyticsSort === "recent" ? "selected" : ""}>Terbaru terbit</option>
              <option value="views" ${ui.analyticsSort === "views" ? "selected" : ""}>Tayangan terbanyak</option>
              <option value="likes" ${ui.analyticsSort === "likes" ? "selected" : ""}>Suka terbanyak</option>
              <option value="comments" ${ui.analyticsSort === "comments" ? "selected" : ""}>Komentar terbanyak</option>
            </select>
          </div>
        </div>

        ${sorted.length === 0 ? `
          <div class="queue-empty">
            <div style="font-weight:700;">Belum ada video terbit pada filter ini</div>
            <p class="mut" style="font-size:0.85rem; margin:6px 0 16px;">
              ${published.length === 0
                ? "Publikasikan video pertama Anda di Studio Upload, lalu klik Perbarui semua metrik untuk memantau tayangan, suka, dan komentarnya."
                : "Belum ada video terbit untuk platform yang dipilih."}
            </p>
            ${published.length === 0 ? `<a href="/" id="btn-perf-upload" class="btn btn-outline">Buka Studio Upload</a>` : ""}
          </div>
        ` : `
          <div class="queue-ledger">
            ${sorted.map((j) => {
              const sp = SPECS[j.platform] || SPECS.youtube;
              const isRowBusy = ui.analyticsBusy || ui.analyticsBusyJob === j.id;
              const hasShares = j.shares !== null && j.shares !== undefined;
              return `
                <article class="queue-row">
                  <div style="display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:14px;">
                    <div style="display:flex; align-items:flex-start; gap:12px; min-width:0; flex:1;">
                      <img class="thumb-sm" width="40" height="54" src="${esc(thumbUrl(j.videoId, j.thumbKey))}" alt="" onerror="this.style.visibility='hidden'" />
                      <div style="min-width:0; flex:1;">
                        <div style="display:flex; flex-wrap:wrap; align-items:center; gap:8px;">
                          <span class="badge">${platformSvg(j.platform)} ${sp.short}</span>
                          <span class="mut" style="font-size:0.8125rem;">${esc(j.account || "-")}</span>
                          ${j.visibility ? `<span class="badge">${esc(shortVisLabel(j.platform, j.visibility))}</span>` : ""}
                        </div>
                        <h2 class="truncate" style="margin:6px 0 2px; font-size:0.95rem; font-weight:700;">${esc(j.title || j.video)}</h2>
                        <div class="mut" style="font-size:0.78rem;">
                          Terbit: ${esc(new Date(j.publishedAt || j.createdAt).toLocaleString())}
                          ${j.metricsUpdatedAt ? ` · Cek terakhir: ${esc(new Date(j.metricsUpdatedAt).toLocaleString())}` : " · Belum pernah dicek"}
                        </div>
                      </div>
                    </div>

                    <div style="display:flex; flex-wrap:wrap; align-items:center; gap:10px;">
                      <div class="perf-metrics ${hasShares ? "has-shares" : ""}" aria-label="Statistik performa video">
                        <div>
                          <div class="perf-metric-lbl">Tayangan</div>
                          <div class="perf-metric-val">${fmtNum(j.views)}</div>
                        </div>
                        <div>
                          <div class="perf-metric-lbl">Suka</div>
                          <div class="perf-metric-val">${fmtNum(j.likes)}</div>
                        </div>
                        <div>
                          <div class="perf-metric-lbl">Komentar</div>
                          <div class="perf-metric-val">${fmtNum(j.comments)}</div>
                        </div>
                        ${hasShares ? `
                          <div>
                            <div class="perf-metric-lbl">Bagikan</div>
                            <div class="perf-metric-val">${fmtNum(j.shares)}</div>
                          </div>
                        ` : ""}
                      </div>

                      <div style="display:flex; align-items:center; gap:8px;">
                        <button type="button" class="btn btn-outline btn-sm" data-refresh-job="${esc(j.id)}" ${isRowBusy ? "disabled" : ""}>
                          ${ui.analyticsBusyJob === j.id ? "Cek..." : "Cek"}
                        </button>
                        ${j.publishedUrl ? `<a class="btn btn-ghost btn-sm" href="${esc(j.publishedUrl)}" target="_blank" rel="noreferrer">Buka</a>` : ""}
                      </div>
                    </div>
                  </div>
                  ${j.metricsError ? `<div class="perf-note">${esc(j.metricsError)}</div>` : ""}
                </article>`;
            }).join("")}
          </div>
        `}
      </div>
    `;

    const refAll = document.getElementById("btn-refresh-perf");
    if (refAll) refAll.onclick = () => refreshMetrics();

    const emptyBtn = document.getElementById("btn-perf-upload");
    if (emptyBtn) emptyBtn.onclick = (e) => { e.preventDefault(); navigate("/"); };

    document.querySelectorAll("[data-pfilter]").forEach((btn) => {
      btn.onclick = () => {
        ui.analyticsPlatform = btn.getAttribute("data-pfilter");
        render();
      };
    });

    const sortSel = document.getElementById("sel-perf-sort");
    if (sortSel) {
      sortSel.onchange = () => {
        ui.analyticsSort = sortSel.value;
        render();
      };
    }

    document.querySelectorAll("[data-refresh-job]").forEach((btn) => {
      btn.onclick = () => refreshMetrics({ jobId: btn.getAttribute("data-refresh-job") });
    });
  }

  function renderDrawer() {
    const dr = document.getElementById("cred-drawer");
    dr.classList.toggle("hidden", !ui.drawerOpen);
    if (!ui.drawerOpen || !ui.state) return;
    renderCredentialsInto(document.getElementById("cred-drawer-body"));
  }

  function renderCredentialsInto(container) {
    if (!container || !ui.state) return;
    const st = ui.state;
    const meta = PROVIDERS.find((p) => p.id === ui.credProvider) || PROVIDERS[0];
    const cred = st.credentials[meta.id] || { configured: false, clientId: "", secretHint: null, source: null };
    const linked = st.accounts.filter((a) => meta.platforms.includes(a.platform));
    const redirectUri = `${st.appUrl}/api/oauth/${meta.id}/callback`;
    const draft = ui.credDrafts[meta.id] || { clientId: cred.clientId || "", clientSecret: "" };

    container.innerHTML = `
      <div class="stack" style="gap:16px;">
        <div class="alert alert-ok">
          <div>
            <strong>Tersimpan terenkripsi di SQLite (AES-256-GCM)</strong>
            <div style="font-size:0.8rem; margin-top:2px;">
              Client ID, Secret, dan token OAuth dikelola langsung dari panel ini tanpa perlu mengubah file <code>.env</code>.
            </div>
          </div>
        </div>

        <div class="prov-tabs" role="tablist" aria-label="Pilih provider platform">
          ${PROVIDERS.map((p) => {
            const c = st.credentials[p.id];
            const cnt = st.accounts.filter((a) => p.platforms.includes(a.platform)).length;
            const dotCls = cnt > 0 ? "dot-ok" : c?.configured ? "dot-warn" : "dot-mut";
            return `
              <button type="button" role="tab" aria-selected="${p.id === meta.id}" class="prov-tab ${p.id === meta.id ? "active" : ""}" data-prov-tab="${p.id}">
                <span style="display:flex; width:100%; justify-content:space-between; align-items:center;">
                  <span style="display:inline-flex; gap:6px;">
                    ${p.platforms.map((pl) => `<span class="plat-badge" style="width:28px; height:28px;">${platformSvg(pl)}</span>`).join("")}
                  </span>
                  <span class="dot ${dotCls}" aria-hidden="true"></span>
                </span>
                <span style="font-size:0.85rem; font-weight:700;">${p.short}</span>
                <span class="mut" style="font-size:0.75rem;">${cnt > 0 ? `${cnt} akun aktif` : c?.configured ? "Siap OAuth" : "Belum diisi"}</span>
              </button>`;
          }).join("")}
        </div>

        <div class="editor-surface stack" style="gap:16px;">
          <div style="display:flex; justify-content:space-between; align-items:flex-start; gap:10px;">
            <div>
              <div style="display:flex; align-items:center; gap:8px; flex-wrap:wrap;">
                <h3 style="margin:0; font-size:1rem; font-weight:700;">${meta.title}</h3>
                ${linked.length > 0
                  ? `<span class="badge badge-ok">Terhubung (${linked.length})</span>`
                  : cred.configured
                    ? `<span class="badge">Siap dihubungkan</span>`
                    : `<span class="badge">Belum diisi</span>`}
                ${cred.source === "env" ? `<span class="badge badge-warn">Dari .env</span>` : ""}
              </div>
              <p class="stage-sub">${meta.sub}</p>
            </div>
            ${cred.source === "db" ? `<button type="button" id="btn-reset-cred" class="btn btn-ghost btn-sm">Reset</button>` : ""}
          </div>

          <div class="uri-box">
            <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
              <span style="font-size:0.8rem; font-weight:600;">Redirect URI (salin ke portal developer)</span>
              <button type="button" id="btn-copy-uri" class="btn btn-sec btn-sm">Salin</button>
            </div>
            <code style="display:block; margin-top:6px; word-break:break-all; font-size:0.8rem;">${esc(redirectUri)}</code>
            <div class="mut" style="font-size:0.75rem; margin-top:6px;">${esc(meta.hint)}</div>
            <div style="margin-top:10px; padding-top:10px; border-top:1px solid var(--line); display:flex; flex-direction:column; gap:6px; font-size:0.75rem;">
              <div style="display:flex; justify-content:space-between; align-items:center; gap:8px; flex-wrap:wrap;">
                <span class="mut">Terms of Service: <code style="color:var(--fg);">${esc(st.appUrl)}/terms</code></span>
                <button type="button" class="btn btn-ghost btn-sm" data-copy-text="${esc(st.appUrl)}/terms">Salin TOS</button>
              </div>
              <div style="display:flex; justify-content:space-between; align-items:center; gap:8px; flex-wrap:wrap;">
                <span class="mut">Privacy Policy: <code style="color:var(--fg);">${esc(st.appUrl)}/privacy</code></span>
                <button type="button" class="btn btn-ghost btn-sm" data-copy-text="${esc(st.appUrl)}/privacy">Salin Privacy</button>
              </div>
              ${meta.id === "meta" ? `
                <div style="display:flex; justify-content:space-between; align-items:center; gap:8px; flex-wrap:wrap;">
                  <span class="mut">Data Deletion: <code style="color:var(--fg);">${esc(st.appUrl)}/data-deletion</code></span>
                  <button type="button" class="btn btn-ghost btn-sm" data-copy-text="${esc(st.appUrl)}/data-deletion">Salin Deletion</button>
                </div>
              ` : ""}
              ${meta.id === "tiktok" ? `
                <div class="mut">Verifikasi domain TikTok: letakkan file <code>tiktok*.txt</code> di <code>data/public/</code> atau isi <code>TIKTOK_SITE_VERIFICATION</code> di <code>.env</code>.</div>
              ` : ""}
            </div>
          </div>

          <form id="cred-form" class="stack" style="gap:14px;">
            <div>
              <label class="field-label" for="inp-cid">${meta.idLabel}</label>
              <input id="inp-cid" class="input" placeholder="${esc(meta.idPh)}" value="${esc(draft.clientId)}" autocomplete="off" required />
            </div>
            <div>
              <label class="field-label" for="inp-csec">
                <span>${meta.secLabel}</span>
                ${cred.secretHint ? `<span class="mut" style="font-weight:400; font-size:0.75rem;">Tersimpan: ${esc(cred.secretHint)}</span>` : ""}
              </label>
              <div class="secret-wrap">
                <input id="inp-csec" class="input" type="${ui.showSecret ? "text" : "password"}"
                  placeholder="${cred.configured ? "Kosongkan jika tidak ingin mengganti secret" : "Tempel Client Secret di sini"}"
                  value="${esc(draft.clientSecret)}" autocomplete="new-password" ${cred.configured ? "" : "required"} />
                <button type="button" id="btn-eye" class="secret-toggle" aria-pressed="${ui.showSecret}">${ui.showSecret ? "Sembunyi" : "Tampil"}</button>
              </div>
            </div>

            <div style="display:flex; flex-wrap:wrap; align-items:center; gap:8px; padding-top:2px;">
              <button type="submit" id="btn-save-cred" class="btn">Simpan kredensial</button>
              <button type="button" id="btn-test-cred" class="btn btn-outline" ${cred.configured ? "" : "disabled"}>Tes koneksi</button>
              ${cred.configured
                ? `<a href="/api/connect/${meta.id}" class="btn btn-ok">${linked.length > 0 ? "Hubungkan akun lain" : "Hubungkan via OAuth"}</a>`
                : `<span class="mut" style="font-size:0.8rem;">Simpan kredensial terlebih dahulu untuk mengaktifkan OAuth.</span>`}
            </div>
          </form>

          ${ui.credTestResult ? `
            <div role="status" class="alert ${ui.credTestResult.ok ? "alert-ok" : "alert-bad"}">
              <span>${esc(ui.credTestResult.message)}</span>
            </div>
          ` : ""}

          <div style="border-top:1px solid var(--line); padding-top:14px;">
            <div style="font-size:0.8125rem; font-weight:700; margin-bottom:8px;">Akun ${meta.short} yang terhubung (${linked.length})</div>
            ${linked.length === 0
              ? `<div class="mut" style="font-size:0.8125rem;">Belum ada akun terhubung pada provider ini.</div>`
              : `<ul class="lib-list">
                  ${linked.map((a) => `
                    <li class="lib-row" style="justify-content:space-between;">
                      <div style="display:flex; align-items:center; gap:10px; min-width:0;">
                        <span class="plat-badge">${platformSvg(a.platform)}</span>
                        <div style="min-width:0;">
                          <div class="truncate" style="font-weight:600; font-size:0.85rem;">${esc(a.displayName || a.externalId)}</div>
                          <div class="mut" style="font-size:0.75rem;">${SPECS[a.platform].label}</div>
                        </div>
                      </div>
                      <button type="button" class="btn btn-bad btn-sm" data-disc-acc="${esc(a.id)}" data-disc-name="${esc(a.displayName || a.platform)}">Putuskan</button>
                    </li>
                  `).join("")}
                </ul>`}
          </div>
        </div>
      </div>
    `;

    container.querySelectorAll("[data-prov-tab]").forEach((btn) => {
      btn.onclick = () => {
        ui.credProvider = btn.getAttribute("data-prov-tab");
        ui.credTestResult = null;
        renderCredentialsInto(container);
      };
    });

    const copyBtn = container.querySelector("#btn-copy-uri");
    copyBtn.onclick = async () => {
      await navigator.clipboard.writeText(redirectUri);
      copyBtn.textContent = "Tersalin";
      setTimeout(() => { copyBtn.textContent = "Salin"; }, 1600);
    };

    container.querySelectorAll("[data-copy-text]").forEach((btn) => {
      btn.onclick = async () => {
        const val = btn.getAttribute("data-copy-text");
        const orig = btn.textContent;
        await navigator.clipboard.writeText(val);
        btn.textContent = "Tersalin";
        setTimeout(() => { btn.textContent = orig; }, 1600);
      };
    });

    const inpCid = container.querySelector("#inp-cid");
    const inpCsec = container.querySelector("#inp-csec");
    const syncDraft = () => {
      ui.credDrafts[meta.id] = { clientId: inpCid.value, clientSecret: inpCsec.value };
    };
    inpCid.oninput = syncDraft;
    inpCsec.oninput = syncDraft;

    container.querySelector("#btn-eye").onclick = () => {
      syncDraft();
      ui.showSecret = !ui.showSecret;
      renderCredentialsInto(container);
    };

    const resetBtn = container.querySelector("#btn-reset-cred");
    if (resetBtn) {
      resetBtn.onclick = async () => {
        if (!confirm(`Hapus Client ID dan Secret ${meta.title} dari database?`)) return;
        await fetch(`/api/credentials/${meta.id}`, { method: "DELETE" });
        delete ui.credDrafts[meta.id];
        ui.credTestResult = null;
        notify("ok", `Kredensial ${meta.title} dihapus`);
        await refreshState();
        renderCredentialsInto(container);
      };
    }

    container.querySelector("#cred-form").onsubmit = async (e) => {
      e.preventDefault();
      const saveBtn = container.querySelector("#btn-save-cred");
      saveBtn.disabled = true;
      ui.credTestResult = null;
      const r = await fetch("/api/credentials", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          provider: meta.id,
          clientId: inpCid.value,
          clientSecret: inpCsec.value || undefined,
        }),
      });
      const j = await r.json().catch(() => ({}));
      saveBtn.disabled = false;
      if (r.ok) {
        delete ui.credDrafts[meta.id];
        notify("ok", `Kredensial ${meta.title} berhasil disimpan`);
        await refreshState();
        renderCredentialsInto(container);
      } else {
        notify("bad", j.error || "Gagal menyimpan kredensial");
      }
    };

    container.querySelector("#btn-test-cred").onclick = async () => {
      const tbtn = container.querySelector("#btn-test-cred");
      tbtn.disabled = true;
      tbtn.textContent = "Menguji...";
      const r = await fetch(`/api/credentials/${meta.id}/test`, { method: "POST" });
      const j = await r.json().catch(() => ({ ok: false, message: "Gagal menghubungi server lokal" }));
      ui.credTestResult = { ok: !!j.ok, message: j.message || "Selesai" };
      renderCredentialsInto(container);
    };

    container.querySelectorAll("[data-disc-acc]").forEach((btn) => {
      btn.onclick = async () => {
        const id = btn.getAttribute("data-disc-acc");
        const nm = btn.getAttribute("data-disc-name");
        if (!confirm(`Putuskan akun "${nm}"? Riwayat pengiriman pada akun ini juga akan dibersihkan.`)) return;
        const r = await fetch(`/api/accounts/${id}`, { method: "DELETE" });
        if (r.ok) {
          notify("ok", `Akun ${nm} diputuskan`);
          await refreshState();
          renderCredentialsInto(container);
          render();
        }
      };
    });
  }

  const savedTheme = localStorage.getItem("cp_theme");
  document.documentElement.classList.toggle("dark", savedTheme ? savedTheme === "dark" : true);

  document.getElementById("btn-theme").onclick = () => {
    const nextDark = !document.documentElement.classList.contains("dark");
    document.documentElement.classList.toggle("dark", nextDark);
    localStorage.setItem("cp_theme", nextDark ? "dark" : "light");
  };

  document.getElementById("btn-logout").onclick = async () => {
    await fetch("/api/auth/logout", { method: "POST" });
    location.href = "/login";
  };

  const bindNavLink = (id, path) => {
    const el = document.getElementById(id);
    if (el) {
      el.onclick = (e) => {
        e.preventDefault();
        navigate(path);
      };
    }
  };
  bindNavLink("brand-link", "/");
  bindNavLink("nav-upload", "/");
  bindNavLink("nav-queue", "/queue");
  bindNavLink("nav-analytics", "/analytics");
  bindNavLink("mnav-upload", "/");
  bindNavLink("mnav-queue", "/queue");
  bindNavLink("mnav-analytics", "/analytics");

  const openCredBtn = document.getElementById("btn-open-creds");
  openCredBtn.onclick = () => openCredentials(null, openCredBtn);
  document.getElementById("btn-close-creds").onclick = closeCredentials;
  document.getElementById("cred-drawer").onclick = (e) => {
    if (e.target.id === "cred-drawer") closeCredentials();
  };
  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && ui.drawerOpen) closeCredentials();
  });
  window.addEventListener("popstate", () => {
    ui.route = resolveRoute(location.pathname);
    render();
  });

  // Buka drawer otomatis bila kembali dari redirect callback OAuth (?creds=...).
  const q = new URLSearchParams(location.search);
  const credsParam = q.get("creds");
  if (credsParam) {
    if (["youtube", "tiktok", "meta"].includes(credsParam)) ui.credProvider = credsParam;
    ui.drawerOpen = true;
  }
  if (q.get("error")) notify("bad", q.get("error"));
  else if (q.get("connected")) notify("ok", `${q.get("connected")} akun berhasil dihubungkan`);
  if (q.toString()) history.replaceState(null, "", location.pathname);

  refreshState().then(() => {
    render();
    if (ui.drawerOpen) renderDrawer();
  });
  setInterval(refreshState, 5000);
})();
