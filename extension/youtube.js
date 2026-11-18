// Botão "Baixar" nas páginas de vídeo do YouTube.
// Envia a URL canônica do vídeo para o app (o background cuida de token/cookies).
(() => {
  "use strict";

  const HOST_ID = "vidmanager-download-host";
  const BTN_ID = "vidmanager-download-btn";
  const TOAST_HOST_ID = "vidmanager-toast-host";
  const TOAST_ID = "vidmanager-download-toast";

  // O host pode ficar dentro do shadow DOM do YouTube, então a referência
  // guardada aqui é a fonte da verdade (getElementById não atravessa shadow roots).
  let hostEl = null;
  let toastHost = null;
  let mountedFor = null;
  let resetTimer = null;
  let lastDeepSearch = 0;
  let deepTries = 0;

  // A barra de ações do watch vive no DOM normal; estes seletores bastam no
  // caminho comum e evitam varrer o documento inteiro a cada ciclo.
  const BAR_SELECTORS = [
    "ytd-watch-metadata #actions-inner #top-level-buttons-computed",
    "ytd-watch-metadata #actions #top-level-buttons-computed",
    "ytd-watch-metadata #top-level-buttons-computed",
  ];

  // ---------------------------------------------------------------- URL / título

  function currentVideoId() {
    try {
      const url = new URL(location.href);
      if (url.pathname === "/watch") return url.searchParams.get("v") || null;
      const m = url.pathname.match(/^\/(?:shorts|live|embed)\/([\w-]{6,})/);
      return m ? m[1] : null;
    } catch {
      return null;
    }
  }

  function canonicalUrl(id) {
    return `https://www.youtube.com/watch?v=${id}`;
  }

  function pageTitle() {
    const meta = document.querySelector('meta[name="title"]');
    const raw = (meta && meta.content) || document.title || "";
    return raw.replace(/\s*-\s*YouTube\s*$/, "").trim();
  }

  // ---------------------------------------------------------------- DOM helpers

  function isVisible(el) {
    if (!el || !el.isConnected) return false;
    const rect = el.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  }

  // Procura o seletor atravessando shadow roots (usado só quando o caminho
  // normal falha, porque varrer a árvore toda é caro).
  function deepQuery(selector) {
    const done = new Set();
    const walk = (root) => {
      if (!root || done.has(root)) return null;
      done.add(root);
      let hit = null;
      try {
        hit = root.querySelector(selector);
      } catch {
        hit = null;
      }
      if (hit) return hit;
      const nodes = root.querySelectorAll ? root.querySelectorAll("*") : [];
      for (const node of nodes) {
        if (node.shadowRoot) {
          const found = walk(node.shadowRoot);
          if (found) return found;
        }
      }
      return null;
    };
    return walk(document);
  }

  function findActionsBar() {
    for (const selector of BAR_SELECTORS) {
      const nodes = document.querySelectorAll(selector);
      for (const node of nodes) {
        if (isVisible(node)) return node;
      }
    }
    // Shorts e mobile não têm a barra do watch: varrer o DOM ali seria só custo.
    if (location.pathname.startsWith("/shorts/") || !document.querySelector("ytd-watch-metadata")) {
      return null;
    }
    // Reserva para uma eventual mudança de layout: poucas tentativas, espaçadas.
    if (deepTries >= 3) return null;
    const now = Date.now();
    if (now - lastDeepSearch < 10000) return null;
    lastDeepSearch = now;
    deepTries += 1;
    const deep = deepQuery("#top-level-buttons-computed") || deepQuery("#actions-inner");
    return deep && isVisible(deep) ? deep : null;
  }

  const BTN_STYLE = `
    :host { all: initial; }
    .btn {
      display: inline-flex;
      align-items: center;
      gap: 7px;
      height: 36px;
      padding: 0 14px;
      border: 1px solid rgba(255,255,255,.14);
      border-radius: 999px;
      background: #0e7490;
      color: #fff;
      font: 600 13px/1 "Segoe UI", system-ui, -apple-system, sans-serif;
      cursor: pointer;
      white-space: nowrap;
      transition: background .15s ease, transform .1s ease;
    }
    .btn:hover { background: #0891b2; }
    .btn:active { transform: scale(.98); }
    .btn[disabled] { cursor: default; opacity: .85; }
    .btn.ok { background: #0f766e; }
    .btn.err { background: #b91c1c; }
    .lbl { line-height: 1; }
    .spinner {
      width: 12px; height: 12px; border-radius: 50%;
      border: 2px solid rgba(255,255,255,.35); border-top-color: #fff;
      animation: spin .7s linear infinite;
    }
    @keyframes spin { to { transform: rotate(360deg); } }
  `;

  const TOAST_STYLE = `
    :host { all: initial; }
    .toast {
      position: fixed;
      right: 20px;
      bottom: 24px;
      z-index: 2147483647;
      max-width: 320px;
      padding: 10px 14px;
      border-radius: 10px;
      border: 1px solid rgba(255,255,255,.12);
      background: #0f172a;
      color: #e2e8f0;
      font: 500 12.5px/1.35 "Segoe UI", system-ui, sans-serif;
      box-shadow: 0 10px 30px rgba(0,0,0,.45);
      opacity: 0;
      transform: translateY(6px);
      transition: opacity .18s ease, transform .18s ease;
      pointer-events: none;
    }
    .toast.show { opacity: 1; transform: translateY(0); }
    .toast.err { border-color: rgba(248,113,113,.45); }
    .toast.ok { border-color: rgba(45,212,191,.45); }
  `;

  const ICON = `<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
      <path fill="currentColor" d="M12 3a1 1 0 0 1 1 1v9.59l3.3-3.3a1 1 0 1 1 1.4 1.42l-5 5a1 1 0 0 1-1.4 0l-5-5a1 1 0 1 1 1.4-1.42l3.3 3.3V4a1 1 0 0 1 1-1Z"/>
      <path fill="currentColor" d="M5 18a1 1 0 0 1 1 1v1h12v-1a1 1 0 1 1 2 0v1.5A1.5 1.5 0 0 1 18.5 22h-13A1.5 1.5 0 0 1 4 20.5V19a1 1 0 0 1 1-1Z"/>
    </svg>`;

  function mountButton() {
    deepTries = 0;
    const host = document.createElement("div");
    host.id = HOST_ID;

    const root = host.attachShadow({ mode: "open" });
    root.innerHTML = `
      <style>${BTN_STYLE}</style>
      <button id="${BTN_ID}" class="btn" type="button" title="Baixar com o VidManager">
        ${ICON}<span class="lbl">Baixar</span>
      </button>
    `;

    root.getElementById(BTN_ID).addEventListener("click", () => {
      const id = currentVideoId();
      if (!id) return;
      send(id, root.getElementById(BTN_ID));
    });

    place(host);
    hostEl = host;
    return host;
  }

  function place(host) {
    const bar = findActionsBar();
    if (bar) {
      host.style.cssText = "display:inline-flex;margin-left:8px;vertical-align:middle;";
      bar.appendChild(host);
      return;
    }
    // Reserva: botão flutuante caso o YouTube mude a barra de ações.
    // No Shorts não existe barra de ações; o canto superior direito fica livre
    // e não cobre a coluna de like/compartilhar.
    host.style.cssText = location.pathname.startsWith("/shorts/")
      ? "position:fixed;top:76px;right:16px;z-index:2147483646;"
      : "position:fixed;right:20px;bottom:84px;z-index:2147483646;";
    (document.body || document.documentElement).appendChild(host);
  }

  // Se a barra apareceu depois do primeiro mount, move o botão para lá.
  function ensurePlacement(host) {
    if (!host.isConnected) return;
    const parent = host.parentElement;
    if (parent && parent.id === "top-level-buttons-computed") return;
    const bar = findActionsBar();
    if (!bar) return;
    bar.appendChild(host);
    host.style.cssText = "display:inline-flex;margin-left:8px;vertical-align:middle;";
  }

  function toastElement() {
    if (toastHost && toastHost.isConnected) {
      return toastHost.shadowRoot.getElementById(TOAST_ID);
    }
    toastHost = document.createElement("div");
    toastHost.id = TOAST_HOST_ID;
    const root = toastHost.attachShadow({ mode: "open" });
    root.innerHTML = `<style>${TOAST_STYLE}</style>
      <div id="${TOAST_ID}" class="toast" role="status" aria-live="polite"></div>`;
    (document.body || document.documentElement).appendChild(toastHost);
    return root.getElementById(TOAST_ID);
  }

  // ---------------------------------------------------------------- envio

  function setLabel(btn, text, kind) {
    btn.className = `btn${kind ? " " + kind : ""}`;
    if (text === "__loading__") {
      btn.innerHTML = `<span class="spinner"></span><span class="lbl">Enviando…</span>`;
      return;
    }
    btn.innerHTML = `${ICON}<span class="lbl"></span>`;
    btn.querySelector(".lbl").textContent = text;
  }

  function showToast(message, kind) {
    const toast = toastElement();
    toast.textContent = message;
    toast.className = `toast show${kind ? " " + kind : ""}`;
    window.clearTimeout(showToast.timer);
    showToast.timer = window.setTimeout(() => {
      toast.className = "toast";
    }, 6000);
  }

  function friendlyError(message) {
    const text = String(message || "");
    if (/paread|token/i.test(text)) return "Extensão não pareada. Abra o popup e digite o código.";
    if (/VidManager|respondeu|falar/i.test(text)) return "Abra o VidManager e confira se a API local está ativa.";
    return text || "Não foi possível enviar o download.";
  }

  async function send(id, btn) {
    btn.disabled = true;
    setLabel(btn, "__loading__");

    let response = null;
    try {
      response = await chrome.runtime.sendMessage({
        type: "DOWNLOAD",
        url: canonicalUrl(id),
        title: pageTitle(),
        referer: location.href,
      });
    } catch {
      btn.disabled = false;
      setLabel(btn, "Baixar");
      showToast("Extensão recarregada — atualize a página e tente de novo.", "err");
      return;
    }

    btn.disabled = false;
    if (response && response.ok) {
      setLabel(btn, "Adicionado ✓", "ok");
      showToast("Download enviado para o VidManager.", "ok");
    } else {
      setLabel(btn, "Erro", "err");
      showToast(friendlyError(response && response.error), "err");
    }

    window.clearTimeout(resetTimer);
    resetTimer = window.setTimeout(() => {
      if (btn.isConnected) setLabel(btn, "Baixar");
    }, 4000);
  }

  // ---------------------------------------------------------------- ciclo de vida

  function teardown() {
    if (hostEl) hostEl.remove();
    hostEl = null;
    mountedFor = null;
  }

  function sync() {
    const id = currentVideoId();
    if (!id) {
      teardown();
      return;
    }
    if (hostEl && !hostEl.isConnected) {
      hostEl = null;
      mountedFor = null;
    }
    if (hostEl && mountedFor !== id) {
      hostEl.remove();
      hostEl = null;
      mountedFor = null;
    }
    if (!hostEl) {
      mountButton();
      mountedFor = id;
      return;
    }
    ensurePlacement(hostEl);
  }

  function scheduleSync(delay = 250) {
    window.clearTimeout(scheduleSync.timer);
    scheduleSync.timer = window.setTimeout(sync, delay);
  }

  for (const event of ["yt-navigate-finish", "yt-page-data-updated", "yt-player-updated"]) {
    window.addEventListener(event, () => scheduleSync(300));
  }
  window.addEventListener("popstate", () => scheduleSync(300));

  // O YouTube re-renderiza a barra de ações; uma verificação periódica leve
  // evita depender de MutationObserver no documento inteiro (muito custoso).
  window.setInterval(sync, 1500);
  scheduleSync(0);
})();
