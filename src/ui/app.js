(function () {
  const PLATFORMS = ["youtube", "tiktok", "instagram", "facebook"];
  const SPECS = {
    youtube: { label: "YouTube Shorts", short: "YouTube", provider: "youtube", maxDur: 180, titleMax: 100, capMax: 4900, cls: "plat-youtube" },
    tiktok: { label: "TikTok", short: "TikTok", provider: "tiktok", maxDur: 600, titleMax: 150, capMax: 2200, cls: "plat-tiktok" },
    instagram: { label: "Instagram Reels", short: "Instagram", provider: "meta", maxDur: 90, titleMax: 0, capMax: 2200, cls: "plat-instagram" },
    facebook: { label: "Facebook Reels", short: "Facebook", provider: "meta", maxDur: 90, titleMax: 255, capMax: 2000, cls: "plat-facebook" },
  };
  const PROVIDERS = [
    {
      id: "youtube",
      title: "YouTube",
      short: "YouTube",
      sub: "Google Cloud Console · YouTube Data API v3",
      idLabel: "Google Client ID",
      idPh: "123456789-xxxx.apps.googleusercontent.com",
      secLabel: "Google Client Secret",
      hint: "Buat OAuth 2.0 Client tipe Web Application dan aktifkan YouTube Data API v3.",
      platforms: ["youtube"],
    },
    {
      id: "tiktok",
      title: "TikTok",
      short: "TikTok",
      sub: "TikTok for Developers · Login Kit & Content Posting API",
      idLabel: "TikTok Client Key",
      idPh: "awxxxxxxxxxxxxxx",
      secLabel: "TikTok Client Secret",
      hint: "Aktifkan scope user.info.basic, video.upload, video.publish. Redirect URI biasanya wajib HTTPS.",
      platforms: ["tiktok"],
    },
    {
      id: "meta",
      title: "Meta (Facebook + Instagram)",
      short: "Meta (FB/IG)",
      sub: "Meta for Developers · Facebook Login & Instagram Graph API",
      idLabel: "Meta App ID",
      idPh: "123456789012345",
      secLabel: "Meta App Secret",
      hint: "Satu koneksi OAuth otomatis menautkan Facebook Page serta akun Instagram Business/Creator yang terhubung.",
      platforms: ["facebook", "instagram"],
    },
  ];

  const ui = {
    route: resolveRoute(location.pathname),
    state: null,
    selectedVideoId: "",
    pickedAccounts: new Set(),
    contentTab: "default",
    title: "",
    caption: "",
    overrides: {},
    hashtagInput: "",
    showPresetForm: false,
    presetName: "",
    scheduleMode: "now",
    scheduleAt: "",
    uploadProgress: null,
    hidePrivacyBanner: false,
    queueFilter: "all",
    expandedErrors: new Set(),
    drawerOpen: false,
    credProvider: "youtube",
    credTestResult: null,
    credDrafts: {},
    showSecret: false,
  };

  function resolveRoute(p) {
    if (p.startsWith("/queue")) return "queue";
    if (p.startsWith("/credentials")) return "credentials";
    return "upload";
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
      return `<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M23.5 6.2a3 3 0 0 0-2.1-2.1C19.5 3.5 12 3.5 12 3.5s-7.5 0-9.4.6A3 3 0 0 0 .5 6.2 31.4 31.4 0 0 0 0 12a31.4 31.4 0 0 0 .5 5.8 3 3 0 0 0 2.1 2.1c1.9.6 9.4.6 9.4.6s7.5 0 9.4-.6a3 3 0 0 0 2.1-2.1A31.4 31.4 0 0 0 24 12a31.4 31.4 0 0 0-.5-5.8ZM9.6 15.6V8.4l6.3 3.6-6.3 3.6Z"/></svg>`;
    }
    if (p === "tiktok") {
      return `<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M19.6 6.7a4.8 4.8 0 0 1-3-4.2V2h-3.4v13.7a2.9 2.9 0 1 1-2-2.8V9.4a6.3 6.3 0 1 0 5.4 6.2V9a8.1 8.1 0 0 0 4.7 1.5V7.1c-.6 0-1.2-.2-1.7-.4Z"/></svg>`;
    }
    if (p === "instagram") {
      return `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2.5" y="2.5" width="19" height="19" rx="5"/><circle cx="12" cy="12" r="4.2"/><circle cx="17.5" cy="6.5" r="1" fill="currentColor" stroke="none"/></svg>`;
    }
    return `<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M22 12a10 10 0 1 0-11.6 9.9v-7H7.9V12h2.5V9.8c0-2.5 1.5-3.9 3.8-3.9 1.1 0 2.2.2 2.2.2v2.5h-1.3c-1.2 0-1.6.8-1.6 1.6V12h2.8l-.4 2.9h-2.4v7A10 10 0 0 0 22 12Z"/></svg>`;
  }

  function describeError(raw) {
    if (!raw) return null;
    const low = raw.toLowerCase();
    if (low.includes("invalid_grant") || low.includes("refresh token") || low.includes("token expired") || low.includes("-> 401:") || low.includes("access_token")) {
      return { summary: "Token akses kedaluwarsa atau dicabut oleh platform.", hint: "Buka panel Kredensial (ikon kunci) lalu hubungkan ulang akun ini.", reconnect: true };
    }
    if (low.includes("kredensial") || low.includes("client_id") || low.includes("client_secret")) {
      return { summary: "Kredensial API belum lengkap atau berubah.", hint: "Periksa Client ID & Secret di panel Kredensial.", reconnect: true };
    }
    if (low.includes("melebihi batas") || low.includes("durasi")) {
      return { summary: raw, hint: "Gunakan video dengan durasi lebih pendek sesuai batas platform." };
    }
    if (low.includes("quota") || low.includes("rate_limit") || low.includes("-> 429:")) {
      return { summary: "Kuota harian API platform tercapai.", hint: "Tunggu reset kuota harian lalu klik Ulangi." };
    }
    if (low.includes("gagal menghubungi") || low.includes("timed out") || low.includes("connection")) {
      return { summary: "Gagal menghubungi server platform (masalah jaringan).", hint: "Worker mencoba ulang otomatis dengan jeda bertahap." };
    }
    return { summary: raw.length > 160 ? raw.slice(0, 160) + "…" : raw };
  }

  let toastTimer = null;
  function notify(kind, text) {
    const box = document.getElementById("toast-box");
    box.className = `alert toast-floating ${kind === "ok" ? "alert-ok" : "alert-bad"}`;
    box.innerHTML = `<span>${esc(text)}</span><button type="button" class="btn btn-ghost btn-sm" id="toast-close">✕</button>`;
    document.getElementById("toast-close").onclick = () => box.classList.add("hidden");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => box.classList.add("hidden"), 6000);
  }

  function navigate(path) {
    history.pushState(null, "", path);
    ui.route = resolveRoute(path);
    render();
  }

  function openCredentials(provider) {
    if (provider) {
      ui.credProvider = provider;
      ui.credTestResult = null;
    }
    ui.drawerOpen = true;
    renderDrawer();
  }

  function closeCredentials() {
    ui.drawerOpen = false;
    renderDrawer();
  }

  async function refreshState() {
    const r = await fetch("/api/state", { cache: "no-store" });
    if (r.status === 401) {
      location.href = "/login";
      return;
    }
    if (!r.ok) return;
    const prevVideoId = ui.selectedVideoId;
    ui.state = await r.json();
    if (!prevVideoId && ui.state.videos.length > 0) {
      ui.selectedVideoId = ui.state.videos[0].id;
    } else if (prevVideoId && !ui.state.videos.some((v) => v.id === prevVideoId)) {
      ui.selectedVideoId = ui.state.videos[0]?.id || "";
    }
    updateTopbar();
    // Hindari mereset fokus input teks bila pengguna sedang mengetik
    const activeTag = document.activeElement?.tagName;
    if (activeTag !== "INPUT" && activeTag !== "TEXTAREA") {
      render();
    }
    if (ui.drawerOpen && activeTag !== "INPUT") {
      renderDrawer();
    }
  }

  function updateTopbar() {
    document.getElementById("nav-upload").classList.toggle("active", ui.route === "upload");
    document.getElementById("nav-queue").classList.toggle("active", ui.route === "queue");

    if (!ui.state) return;
    const active = ui.state.jobs.filter((j) => j.status === "queued" || j.status === "processing").length;
    const failed = ui.state.jobs.filter((j) => j.status === "failed").length;
    const badge = document.getElementById("nav-queue-badge");
    if (failed > 0) {
      badge.textContent = failed + "!";
      badge.className = "nav-count alert";
    } else if (active > 0) {
      badge.textContent = String(active);
      badge.className = "nav-count";
    } else {
      badge.className = "nav-count hidden";
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
    if (!ui.state) {
      root.innerHTML = `<div style="padding:64px 0; text-align:center;" class="mut">Memuat studio…</div>`;
      return;
    }
    if (ui.route === "queue") {
      renderQueue(root);
    } else if (ui.route === "credentials") {
      root.innerHTML = `
        <div style="max-width:620px; margin:26px auto 56px;" class="stack">
          <div>
            <h1 style="margin:0; font-size:19px;">Kredensial &amp; Koneksi Akun</h1>
            <p class="card-sub">Halaman langsung pengelolaan API Client ID, Secret, dan akun OAuth.</p>
          </div>
          <div id="creds-page-slot"></div>
        </div>`;
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
      ${isPrivate && !ui.hidePrivacyBanner ? `
        <div class="alert alert-warn" style="margin-top:20px;">
          <div>
            <strong style="color:var(--fg);">Mode Sandbox / Sebelum Audit Aktif:</strong>
            <span class="mut"> YouTube diset <code>${esc(st.privacy.youtube)}</code> dan TikTok diset <code>${esc(st.privacy.tiktok)}</code> agar aman diuji sebelum App Review selesai.</span>
          </div>
          <button type="button" id="btn-hide-priv" class="btn btn-ghost btn-sm">✕</button>
        </div>
      ` : ""}

      <div class="grid-upload">
        <!-- Kolom Kiri -->
        <div class="stack">
          <div class="card">
            <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom:12px;">
              <h2 class="card-title">1. Sumber Video</h2>
              <span class="mut" style="font-size:11px;">MP4, MOV, WEBM, MKV</span>
            </div>

            <div id="dropzone" class="dropzone">
              <input id="file-input" type="file" accept="video/*" class="hidden" />
              <div class="drop-icon">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                  <polyline points="17 8 12 3 7 8"></polyline>
                  <line x1="12" y1="3" x2="12" y2="15"></line>
                </svg>
              </div>
              <div style="font-weight:600;">Tarik &amp; lepas video di sini, atau <span style="color:var(--primary); text-decoration:underline;">pilih file</span></div>
              <div class="mut" style="font-size:12px; margin-top:4px;">Otomatis dinormalisasi ke 1080×1920 (9:16 H.264/AAC) saat dikirim</div>

              ${ui.uploadProgress !== null ? `
                <div style="margin-top:14px; text-align:left;">
                  <div style="display:flex; justify-content:space-between; font-size:12px;">
                    <span>Mengunggah &amp; ekstrak thumbnail…</span>
                    <strong style="color:var(--primary);">${ui.uploadProgress}%</strong>
                  </div>
                  <div class="progress-track"><div class="progress-fill" style="width:${ui.uploadProgress}%"></div></div>
                </div>
              ` : ""}
            </div>

            ${selVid ? `
              <div class="video-preview">
                <video controls preload="metadata" poster="/api/videos/${esc(selVid.id)}/thumb" src="/api/videos/${esc(selVid.id)}/stream"></video>
                <div class="video-meta-bar">
                  <div style="min-width:0;">
                    <div class="truncate" style="font-weight:600;">${esc(selVid.name)}</div>
                    <div class="mut" style="font-size:11px;">
                      ${fmtDur(selVid.durationSec)} · ${selVid.width && selVid.height ? `${selVid.width}×${selVid.height} · ` : ""}${fmtBytes(selVid.sizeBytes)}
                    </div>
                  </div>
                  <span class="badge">Dipilih</span>
                </div>
              </div>
            ` : ""}
          </div>

          <div class="card">
            <div style="font-size:11.5px; font-weight:700; text-transform:uppercase; letter-spacing:.05em; margin-bottom:10px;" class="mut">
              Pustaka Video (${st.videos.length})
            </div>
            ${st.videos.length === 0 ? `
              <div class="mut" style="text-align:center; padding:20px 0; font-size:12.5px;">Belum ada video yang diunggah.</div>
            ` : `
              <div class="lib-list">
                ${st.videos.map((v) => `
                  <div class="lib-item ${v.id === ui.selectedVideoId ? "active" : ""}" data-select-video="${esc(v.id)}">
                    <div style="display:flex; align-items:center; gap:10px; min-width:0;">
                      <img class="thumb-sm" src="/api/videos/${esc(v.id)}/thumb" alt="" onerror="this.style.visibility='hidden'" />
                      <div style="min-width:0;">
                        <div class="truncate" style="font-size:12.5px; font-weight:600;">${esc(v.name)}</div>
                        <div class="mut" style="font-size:11px;">${fmtDur(v.durationSec)} · ${fmtBytes(v.sizeBytes)}</div>
                      </div>
                    </div>
                    <button type="button" class="btn btn-ghost btn-sm" data-del-video="${esc(v.id)}" data-del-name="${esc(v.name)}" title="Hapus video">🗑</button>
                  </div>
                `).join("")}
              </div>
            `}
          </div>
        </div>

        <!-- Kolom Kanan -->
        <form id="post-form" class="stack">
          <div class="card">
            <div style="display:flex; justify-content:space-between; align-items:flex-start; gap:10px; margin-bottom:14px;">
              <div>
                <h2 class="card-title">2. Pilih Tujuan Platform</h2>
                <p class="card-sub">Klik akun tujuan cross-upload. Kredensial diatur lewat ikon kunci di header.</p>
              </div>
              ${st.accounts.length > 0 ? `
                <button type="button" id="btn-toggle-all" class="btn btn-ghost btn-sm">
                  ${ui.pickedAccounts.size === st.accounts.length ? "Batal Semua" : "Pilih Semua"}
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
                        <span class="plat-badge ${sp.cls}">${platformSvg(pl)}</span>
                        <div>
                          <div style="font-size:12.5px; font-weight:600;" class="mut">${sp.label}</div>
                          <div class="mut" style="font-size:11px;">Belum terhubung</div>
                        </div>
                      </div>
                      <button type="button" class="btn btn-outline btn-sm" data-open-cred="${sp.provider}">🔑 Hubungkan</button>
                    </div>`;
                }
                return accs.map((a) => {
                  const sel = ui.pickedAccounts.has(a.id);
                  return `
                    <button type="button" class="plat-card ${sel ? "selected" : ""}" data-toggle-acc="${esc(a.id)}">
                      <div style="display:flex; align-items:center; gap:10px; min-width:0;">
                        <span class="plat-badge ${sp.cls}">${platformSvg(pl)}</span>
                        <div style="min-width:0;">
                          <div class="truncate" style="font-size:12.5px; font-weight:600;">${esc(a.displayName || a.externalId)}</div>
                          <div class="mut" style="font-size:11px;">${sp.label} · maks ${sp.maxDur} dtk</div>
                        </div>
                      </div>
                      <span class="check-box">${sel ? "✓" : ""}</span>
                    </button>`;
                }).join("");
              }).join("")}
            </div>

            ${durWarnings.length > 0 ? `
              <div class="alert alert-warn" style="margin-top:12px; flex-direction:column; gap:4px;">
                <strong>⚠ Peringatan Durasi Video</strong>
                ${durWarnings.map((w) => `<div>• ${esc(w)}</div>`).join("")}
              </div>
            ` : ""}
          </div>

          <div class="card stack" style="gap:14px;">
            <div>
              <h2 class="card-title">3. Detail Konten &amp; Override</h2>
              <p class="card-sub">Isi sekali di tab Default, atau kustomisasi khusus per platform bila diperlukan.</p>
            </div>

            ${selPlats.length > 0 ? `
              <div class="tab-row">
                <button type="button" class="tab-chip ${ui.contentTab === "default" ? "active" : ""}" data-ctab="default">
                  ✦ Semua Platform (Default)
                </button>
                ${selPlats.map((pl) => {
                  const hasOv = !!((ui.overrides[pl]?.title || "").trim() || (ui.overrides[pl]?.caption || "").trim());
                  return `
                    <button type="button" class="tab-chip ${ui.contentTab === pl ? "active" : ""}" data-ctab="${pl}">
                      ${platformSvg(pl)} ${SPECS[pl].short} ${hasOv ? "•" : ""}
                    </button>`;
                }).join("")}
              </div>
            ` : ""}

            ${ui.contentTab === "default" ? `
              <div>
                <label class="field-label">
                  <span>Judul Video</span>
                  <span class="mut" id="title-counter">${ui.title.length}/100 (batas YouTube)</span>
                </label>
                <input id="inp-title" class="input" maxlength="150" placeholder="Judul singkat & menarik…" value="${esc(ui.title)}" />
              </div>
              <div>
                <label class="field-label">
                  <span>Caption / Deskripsi</span>
                  <span class="mut" id="cap-counter">${ui.caption.length}/2000 karakter</span>
                </label>
                <textarea id="inp-caption" class="textarea" placeholder="Tulis caption utama untuk semua platform…">${esc(ui.caption)}</textarea>
              </div>
            ` : `
              <div class="uri-box stack" style="gap:12px;">
                <div style="display:flex; justify-content:space-between; align-items:center; font-size:12px;">
                  <strong>Kustomisasi khusus ${SPECS[ui.contentTab].label}</strong>
                  <button type="button" id="btn-reset-ov" class="btn btn-ghost btn-sm">Ikuti Default</button>
                </div>
                ${SPECS[ui.contentTab].titleMax > 0 ? `
                  <div>
                    <label class="field-label">
                      <span class="mut">Judul Khusus</span>
                      <span class="mut" id="ov-title-counter">${(ui.overrides[ui.contentTab]?.title || "").length}/${SPECS[ui.contentTab].titleMax}</span>
                    </label>
                    <input id="inp-ov-title" class="input" maxlength="${SPECS[ui.contentTab].titleMax}" placeholder="${esc(ui.title || "Kosongkan untuk memakai judul default…")}" value="${esc(ui.overrides[ui.contentTab]?.title || "")}" />
                  </div>
                ` : ""}
                <div>
                  <label class="field-label">
                    <span class="mut">Caption Khusus</span>
                    <span class="mut" id="ov-cap-counter">${(ui.overrides[ui.contentTab]?.caption || "").length}/${SPECS[ui.contentTab].capMax}</span>
                  </label>
                  <textarea id="inp-ov-caption" class="textarea" maxlength="${SPECS[ui.contentTab].capMax}" placeholder="${esc(ui.caption || "Kosongkan untuk memakai caption default…")}">${esc(ui.overrides[ui.contentTab]?.caption || "")}</textarea>
                </div>
              </div>
            `}

            <div style="border-top:1px solid var(--border); padding-top:12px;">
              <div class="field-label">
                <span># Hashtag (<span id="tag-count">${tags.length}</span>)</span>
                <button type="button" id="btn-toggle-preset" class="btn btn-ghost btn-sm" style="color:var(--primary);">+ Simpan Preset</button>
              </div>
              <input id="inp-hashtags" class="input" placeholder="#fyp #reels #shorts #edukasi" value="${esc(ui.hashtagInput)}" />

              ${ui.showPresetForm ? `
                <div style="display:flex; gap:8px; margin-top:8px;">
                  <input id="inp-preset-name" class="input" placeholder="Nama preset (mis. Edukasi)…" value="${esc(ui.presetName)}" />
                  <button type="button" id="btn-save-preset" class="btn btn-sm" style="height:36px;">Simpan</button>
                </div>
              ` : ""}

              ${st.presets.length > 0 ? `
                <div style="display:flex; flex-wrap:wrap; align-items:center; gap:6px; margin-top:10px;">
                  <span class="mut" style="font-size:11px;">Preset:</span>
                  ${st.presets.map((pr) => `
                    <span class="preset-pill">
                      <button type="button" data-apply-preset="${esc(pr.id)}" title="${esc(pr.hashtags.map((t) => "#" + t).join(" "))}">+ ${esc(pr.name)}</button>
                      <button type="button" data-del-preset="${esc(pr.id)}" class="mut" title="Hapus">×</button>
                    </span>
                  `).join("")}
                </div>
              ` : ""}
            </div>
          </div>

          <div class="card stack" style="gap:14px;">
            <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:10px;">
              <div>
                <h2 class="card-title">4. Waktu Publikasi</h2>
                <p class="card-sub">Zona waktu lokal: ${esc(tz)}</p>
              </div>
              <div class="nav-pills">
                <button type="button" class="nav-btn ${ui.scheduleMode === "now" ? "active" : ""}" data-sched-mode="now">Kirim Sekarang</button>
                <button type="button" class="nav-btn ${ui.scheduleMode === "later" ? "active" : ""}" data-sched-mode="later">Jadwalkan</button>
              </div>
            </div>

            ${ui.scheduleMode === "later" ? `
              <div class="uri-box stack" style="gap:10px;">
                <div style="display:flex; flex-wrap:wrap; gap:6px;">
                  <button type="button" class="btn btn-sec btn-sm" data-quick-sched="plus1h">+1 Jam</button>
                  <button type="button" class="btn btn-sec btn-sm" data-quick-sched="tonight">Malam Ini 19:00</button>
                  <button type="button" class="btn btn-sec btn-sm" data-quick-sched="tmrMorning">Besok Pagi 08:00</button>
                  <button type="button" class="btn btn-sec btn-sm" data-quick-sched="tmrNight">Besok Malam 19:00</button>
                </div>
                <input id="inp-sched-at" type="datetime-local" class="input" value="${esc(ui.scheduleAt)}" required />
              </div>
            ` : ""}

            <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:12px;">
              <button type="button" id="link-to-queue" class="btn btn-ghost btn-sm">Lihat status &amp; riwayat di Antrean →</button>
              <button type="submit" id="btn-submit-post" class="btn btn-lg"
                ${!ui.selectedVideoId || ui.pickedAccounts.size === 0 || durWarnings.length > 0 ? "disabled" : ""}>
                ${ui.scheduleMode === "later"
                  ? `Jadwalkan ke ${ui.pickedAccounts.size} Akun`
                  : `Publikasikan ke ${ui.pickedAccounts.size} Akun`}
              </button>
            </div>
          </div>
        </form>
      </div>
    `;

    bindUploadEvents();
  }

  function bindUploadEvents() {
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

    document.querySelectorAll("[data-select-video]").forEach((el) => {
      el.onclick = () => {
        ui.selectedVideoId = el.getAttribute("data-select-video");
        render();
      };
    });

    document.querySelectorAll("[data-del-video]").forEach((btn) => {
      btn.onclick = async (e) => {
        e.stopPropagation();
        const id = btn.getAttribute("data-del-video");
        const name = btn.getAttribute("data-del-name");
        if (!confirm(`Hapus video "${name}" dari pustaka?`)) return;
        const r = await fetch(`/api/videos/${id}`, { method: "DELETE" });
        if (r.ok) {
          notify("ok", "Video dihapus");
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
      btn.onclick = () => openCredentials(btn.getAttribute("data-open-cred"));
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

    const inpOvTitle = document.getElementById("inp-ov-title");
    if (inpOvTitle) {
      inpOvTitle.oninput = () => {
        const cur = ui.overrides[ui.contentTab] || { title: "", caption: "" };
        cur.title = inpOvTitle.value;
        ui.overrides[ui.contentTab] = cur;
        document.getElementById("ov-title-counter").textContent = `${cur.title.length}/${SPECS[ui.contentTab].titleMax}`;
      };
    }

    const inpOvCap = document.getElementById("inp-ov-caption");
    if (inpOvCap) {
      inpOvCap.oninput = () => {
        const cur = ui.overrides[ui.contentTab] || { title: "", caption: "" };
        cur.caption = inpOvCap.value;
        ui.overrides[ui.contentTab] = cur;
        document.getElementById("ov-cap-counter").textContent = `${cur.caption.length}/${SPECS[ui.contentTab].capMax}`;
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

    document.getElementById("link-to-queue").onclick = () => navigate("/queue");

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
          if ((v.title && v.title.trim()) || (v.caption && v.caption.trim())) {
            cleanOverrides[k] = {
              title: v.title?.trim() || undefined,
              caption: v.caption?.trim() || undefined,
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
            hashtags: parsedHashtags(),
            scheduleAt: ui.scheduleMode === "later" ? new Date(ui.scheduleAt).toISOString() : null,
            overrides: cleanOverrides,
          }),
        });
        const j = await r.json().catch(() => ({}));
        if (r.ok) {
          notify("ok", `${j.jobIds.length} job berhasil dibuat!`);
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
        notify("bad", "Gagal menghubungi server");
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
      if (j.status === "published") return `<span class="badge badge-ok">✓ Terbit</span>`;
      if (j.status === "processing") return `<span class="badge badge-info">⟳ Diproses</span>`;
      if (j.status === "failed") return `<span class="badge badge-bad">✕ Gagal</span>`;
      if (j.status === "cancelled") return `<span class="badge">Dibatalkan</span>`;
      return `<span class="badge badge-warn">${future ? "🕒 Terjadwal" : "Menunggu"}</span>`;
    };

    const filters = [
      ["all", "Semua", counts.all],
      ["scheduled", "Terjadwal / Antre", counts.scheduled],
      ["processing", "Diproses", counts.processing],
      ["published", "Selesai", counts.published],
      ["failed", "Gagal", counts.failed],
    ];

    root.innerHTML = `
      <div class="stack" style="padding:24px 0 56px;">
        <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:12px;">
          <div>
            <h1 style="margin:0; font-size:19px;">Antrean &amp; Riwayat Publikasi</h1>
            <p class="card-sub">Diperbarui otomatis setiap 5 detik. Worker Rust memproses antrean di latar belakang.</p>
          </div>
          <button type="button" id="btn-go-upload" class="btn">+ Buat Upload Baru</button>
        </div>

        <div class="stats-grid">
          <div class="card"><div class="mut" style="font-size:12px;">Menunggu / Terjadwal</div><div class="stat-val" style="color:var(--warn);">${counts.scheduled}</div></div>
          <div class="card"><div class="mut" style="font-size:12px;">Sedang Diproses</div><div class="stat-val" style="color:var(--info);">${counts.processing}</div></div>
          <div class="card"><div class="mut" style="font-size:12px;">Berhasil Terbit</div><div class="stat-val" style="color:var(--ok);">${counts.published}</div></div>
          <div class="card"><div class="mut" style="font-size:12px;">Perlu Tindakan (Gagal)</div><div class="stat-val" style="color:var(--bad);">${counts.failed}</div></div>
        </div>

        <div style="display:flex; flex-wrap:wrap; gap:6px;">
          ${filters.map(([id, label, cnt]) => `
            <button type="button" class="tab-chip ${ui.queueFilter === id ? "active" : ""}" data-qfilter="${id}">
              ${label} <span style="opacity:.8;">(${cnt})</span>
            </button>
          `).join("")}
        </div>

        ${list.length === 0 ? `
          <div class="card" style="text-align:center; padding:52px 20px;">
            <div style="font-weight:600;">Tidak ada item pada filter ini</div>
            <div class="mut" style="font-size:12.5px; margin-top:4px;">Unggah video dan pilih platform tujuan di halaman Upload untuk memulai.</div>
          </div>
        ` : `
          <div class="stack" style="gap:12px;">
            ${list.map((j) => {
              const sp = SPECS[j.platform] || SPECS.youtube;
              const fr = describeError(j.error);
              const showRaw = ui.expandedErrors.has(j.id);
              return `
                <div class="card" style="padding:16px;">
                  <div style="display:flex; justify-content:space-between; align-items:flex-start; flex-wrap:wrap; gap:14px;">
                    <div style="display:flex; align-items:flex-start; gap:12px; min-width:0;">
                      <img class="thumb-sm" src="/api/videos/${esc(j.videoId)}/thumb" alt="" onerror="this.style.visibility='hidden'" />
                      <div style="min-width:0;">
                        <div style="display:flex; flex-wrap:wrap; align-items:center; gap:7px;">
                          <span class="badge ${sp.cls}" style="border:0;">${platformSvg(j.platform)} ${sp.short}</span>
                          <span class="mut" style="font-size:12px;">${esc(j.account || "-")}</span>
                          ${statusBadge(j)}
                          ${j.attempts > 0 && j.status !== "published" ? `<span class="mut" style="font-size:11px;">Percobaan ${j.attempts}/3</span>` : ""}
                        </div>
                        <div class="truncate" style="font-weight:700; margin-top:4px;">${esc(j.title || j.video)}</div>
                        <div class="mut" style="font-size:11.5px; margin-top:2px;">
                          File: ${esc(j.video)}
                          ${j.scheduleAt ? ` · Jadwal: ${esc(new Date(j.scheduleAt).toLocaleString())}` : ""}
                          ${j.publishedAt ? ` · Terbit: ${esc(new Date(j.publishedAt).toLocaleString())}` : ""}
                        </div>
                      </div>
                    </div>

                    <div style="display:flex; align-items:center; gap:8px;">
                      ${j.publishedUrl ? `<a class="btn btn-ok btn-sm" href="${esc(j.publishedUrl)}" target="_blank" rel="noreferrer">Lihat Post ↗</a>` : ""}
                      ${j.status === "failed" || j.status === "cancelled" ? `<button type="button" class="btn btn-sm" data-job-act="retry" data-job-id="${esc(j.id)}">↻ Ulangi</button>` : ""}
                      ${j.status === "queued" ? `<button type="button" class="btn btn-outline btn-sm" data-job-act="cancel" data-job-id="${esc(j.id)}">Batalkan</button>` : ""}
                    </div>
                  </div>

                  ${fr ? `
                    <div class="alert alert-bad" style="margin-top:12px; flex-direction:column; align-items:stretch;">
                      <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:8px;">
                        <div>
                          <div style="font-weight:600;">${esc(fr.summary)}</div>
                          ${fr.hint ? `<div class="mut" style="font-size:11.5px; margin-top:2px;">${esc(fr.hint)}</div>` : ""}
                        </div>
                        <div style="display:flex; align-items:center; gap:8px;">
                          ${fr.reconnect ? `<button type="button" class="btn btn-outline btn-sm" data-open-cred="${sp.provider}">🔑 Buka Kredensial</button>` : ""}
                          ${j.error ? `<button type="button" class="btn btn-ghost btn-sm" data-toggle-raw="${esc(j.id)}">${showRaw ? "Tutup detail" : "Detail teknis"}</button>` : ""}
                        </div>
                      </div>
                      ${showRaw && j.error ? `<pre style="margin:8px 0 0; padding:8px; background:rgba(0,0,0,.35); border-radius:7px; overflow-x:auto; font-size:11px;">${esc(j.error)}</pre>` : ""}
                    </div>
                  ` : ""}
                </div>`;
            }).join("")}
          </div>
        `}
      </div>
    `;

    document.getElementById("btn-go-upload").onclick = () => navigate("/");
    document.querySelectorAll("[data-qfilter]").forEach((btn) => {
      btn.onclick = () => {
        ui.queueFilter = btn.getAttribute("data-qfilter");
        render();
      };
    });
    document.querySelectorAll("[data-open-cred]").forEach((btn) => {
      btn.onclick = () => openCredentials(btn.getAttribute("data-open-cred"));
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
            <strong style="color:var(--fg);">Tersimpan terenkripsi di SQLite (AES-256-GCM)</strong>
            <div class="mut" style="font-size:11.5px; margin-top:2px;">
              Semua Client ID, Secret, dan token OAuth dikelola dari panel ini tanpa perlu mengedit file <code>.env</code>.
            </div>
          </div>
        </div>

        <div class="prov-tabs">
          ${PROVIDERS.map((p) => {
            const c = st.credentials[p.id];
            const cnt = st.accounts.filter((a) => p.platforms.includes(a.platform)).length;
            const dotCls = cnt > 0 ? "dot-ok" : c?.configured ? "dot-warn" : "dot-mut";
            return `
              <button type="button" class="prov-tab ${p.id === meta.id ? "active" : ""}" data-prov-tab="${p.id}">
                <div style="display:flex; width:100%; justify-content:space-between; align-items:center;">
                  <div style="display:flex; gap:4px;">
                    ${p.platforms.map((pl) => `<span class="plat-badge ${SPECS[pl].cls}" style="width:24px; height:24px; border-radius:6px;">${platformSvg(pl)}</span>`).join("")}
                  </div>
                  <span class="dot ${dotCls}"></span>
                </div>
                <div style="font-size:12px; font-weight:700;">${p.short}</div>
                <div class="mut" style="font-size:11px;">${cnt > 0 ? `${cnt} akun aktif` : c?.configured ? "Siap OAuth" : "Belum diisi"}</div>
              </button>`;
          }).join("")}
        </div>

        <div class="card stack" style="gap:16px;">
          <div style="display:flex; justify-content:space-between; align-items:flex-start; gap:10px;">
            <div>
              <div style="display:flex; align-items:center; gap:8px; flex-wrap:wrap;">
                <h3 class="card-title">${meta.title}</h3>
                ${linked.length > 0
                  ? `<span class="badge badge-ok">Terhubung (${linked.length})</span>`
                  : cred.configured
                    ? `<span class="badge badge-info">Siap dihubungkan</span>`
                    : `<span class="badge">Belum diisi</span>`}
                ${cred.source === "env" ? `<span class="badge badge-warn">Dari .env</span>` : ""}
              </div>
              <p class="card-sub">${meta.sub}</p>
            </div>
            ${cred.source === "db" ? `<button type="button" id="btn-reset-cred" class="btn btn-ghost btn-sm">🗑 Reset</button>` : ""}
          </div>

          <div class="uri-box">
            <div style="display:flex; justify-content:space-between; align-items:center; gap:8px;">
              <span class="mut" style="font-size:10.5px; font-weight:700; text-transform:uppercase; letter-spacing:.05em;">Redirect URI (salin ke portal developer)</span>
              <button type="button" id="btn-copy-uri" class="btn btn-sec btn-sm">Salin</button>
            </div>
            <code style="display:block; margin-top:6px; word-break:break-all; font-size:12px;">${esc(redirectUri)}</code>
            <div class="mut" style="font-size:11px; margin-top:6px;">${esc(meta.hint)}</div>
          </div>

          <form id="cred-form" class="stack" style="gap:12px;">
            <div>
              <label class="field-label">${meta.idLabel}</label>
              <input id="inp-cid" class="input" placeholder="${esc(meta.idPh)}" value="${esc(draft.clientId)}" autocomplete="off" required />
            </div>
            <div>
              <label class="field-label">
                <span>${meta.secLabel}</span>
                ${cred.secretHint ? `<span class="mut" style="font-weight:400; font-size:11px;">Tersimpan: ${esc(cred.secretHint)}</span>` : ""}
              </label>
              <div class="secret-wrap">
                <input id="inp-csec" class="input" type="${ui.showSecret ? "text" : "password"}" style="padding-right:40px;"
                  placeholder="${cred.configured ? "Kosongkan jika tidak ingin mengganti secret" : "Tempel Client Secret di sini…"}"
                  value="${esc(draft.clientSecret)}" autocomplete="new-password" ${cred.configured ? "" : "required"} />
                <button type="button" id="btn-eye" class="secret-toggle" title="Tampilkan/sembunyikan">${ui.showSecret ? "🙈" : "👁"}</button>
              </div>
            </div>

            <div style="display:flex; flex-wrap:wrap; align-items:center; gap:8px; padding-top:2px;">
              <button type="submit" id="btn-save-cred" class="btn">🔑 Simpan Kredensial</button>
              <button type="button" id="btn-test-cred" class="btn btn-outline" ${cred.configured ? "" : "disabled"}>⚡ Tes Koneksi</button>
              ${cred.configured
                ? `<a href="/api/connect/${meta.id}" class="btn btn-ok">🔗 ${linked.length > 0 ? "Hubungkan Akun Lain" : "Hubungkan via OAuth"}</a>`
                : `<span class="mut" style="font-size:11.5px;">Simpan kredensial dulu untuk mengaktifkan OAuth.</span>`}
            </div>
          </form>

          ${ui.credTestResult ? `
            <div class="alert ${ui.credTestResult.ok ? "alert-ok" : "alert-bad"}">
              <span>${ui.credTestResult.ok ? "✓ " : "✕ "}${esc(ui.credTestResult.message)}</span>
            </div>
          ` : ""}

          <div style="border-top:1px solid var(--border); padding-top:14px;">
            <div class="mut" style="font-size:12px; font-weight:600; margin-bottom:8px;">Akun ${meta.short} yang Terhubung (${linked.length})</div>
            ${linked.length === 0
              ? `<div class="mut" style="font-size:12px;">Belum ada akun yang terhubung pada provider ini.</div>`
              : `<div class="stack" style="gap:8px;">
                  ${linked.map((a) => `
                    <div class="lib-item" style="cursor:default;">
                      <div style="display:flex; align-items:center; gap:10px; min-width:0;">
                        <span class="plat-badge ${SPECS[a.platform].cls}">${platformSvg(a.platform)}</span>
                        <div style="min-width:0;">
                          <div class="truncate" style="font-weight:600; font-size:13px;">${esc(a.displayName || a.externalId)}</div>
                          <div class="mut" style="font-size:11px;">${SPECS[a.platform].label}</div>
                        </div>
                      </div>
                      <button type="button" class="btn btn-bad btn-sm" data-disc-acc="${esc(a.id)}" data-disc-name="${esc(a.displayName || a.platform)}">Putuskan</button>
                    </div>
                  `).join("")}
                </div>`}
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
      copyBtn.textContent = "Tersalin ✓";
      setTimeout(() => { copyBtn.textContent = "Salin"; }, 1600);
    };

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
        if (!confirm(`Hapus Client ID & Secret ${meta.title} dari database?`)) return;
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
      tbtn.textContent = "Menguji…";
      const r = await fetch(`/api/credentials/${meta.id}/test`, { method: "POST" });
      const j = await r.json().catch(() => ({ ok: false, message: "Gagal menghubungi server" }));
      ui.credTestResult = { ok: !!j.ok, message: j.message || "Selesai" };
      renderCredentialsInto(container);
    };

    container.querySelectorAll("[data-disc-acc]").forEach((btn) => {
      btn.onclick = async () => {
        const id = btn.getAttribute("data-disc-acc");
        const nm = btn.getAttribute("data-disc-name");
        if (!confirm(`Putuskan akun "${nm}"? Riwayat job pada akun ini juga akan dibersihkan.`)) return;
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

  // Init global listeners & theme
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

  document.getElementById("brand-link").onclick = (e) => { e.preventDefault(); navigate("/"); };
  document.getElementById("nav-upload").onclick = () => navigate("/");
  document.getElementById("nav-queue").onclick = () => navigate("/queue");
  document.getElementById("btn-open-creds").onclick = () => openCredentials();
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

  // Tangani query string hasil callback OAuth (?creds=...&connected=... / ?error=...)
  const q = new URLSearchParams(location.search);
  const credsParam = q.get("creds");
  if (credsParam) {
    if (["youtube", "tiktok", "meta"].includes(credsParam)) ui.credProvider = credsParam;
    ui.drawerOpen = true;
  }
  if (q.get("error")) notify("bad", q.get("error"));
  else if (q.get("connected")) notify("ok", `${q.get("connected")} akun berhasil dihubungkan!`);
  if (q.toString()) history.replaceState(null, "", location.pathname);

  refreshState().then(() => {
    render();
    if (ui.drawerOpen) renderDrawer();
  });
  setInterval(refreshState, 5000);
})();
