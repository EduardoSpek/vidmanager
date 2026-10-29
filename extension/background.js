const DEFAULTS = { port: 41300, token: "", preset: "best" };

async function settings() {
  return await chrome.storage.local.get(DEFAULTS);
}

function baseUrl(port) {
  return `http://127.0.0.1:${port}`;
}

async function request(port, path, options = {}, timeoutMs = 12000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(`${baseUrl(port)}${path}`, {
      ...options,
      signal: controller.signal
    });
    const text = await response.text();
    let data = {};
    try {
      data = text ? JSON.parse(text) : {};
    } catch {
      data = { ok: false, error: text };
    }
    if (!response.ok) {
      return { ok: false, error: data.error || `HTTP ${response.status}` };
    }
    return { ok: true, ...data };
  } catch (error) {
    const message =
      error?.name === "AbortError"
        ? "O VidManager não respondeu a tempo."
        : "Não foi possível falar com o VidManager. Verifique se o app está aberto e a API local ativa.";
    return { ok: false, error: message };
  } finally {
    clearTimeout(timer);
  }
}

async function ping(port) {
  return request(port, "/ping", { method: "GET" }, 4000);
}

async function pair(code, port) {
  const clean = String(code || "").replace(/\D/g, "").slice(0, 6);
  if (clean.length !== 6) {
    return { ok: false, error: "Digite os 6 dígitos do código de pareamento." };
  }
  const result = await request(
    port,
    "/pair",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code: clean })
    },
    8000
  );
  if (result.ok && result.token) {
    await chrome.storage.local.set({ token: result.token, port });
  }
  return result;
}

async function collectCookies(url) {
  try {
    const parsed = new URL(url);
    const host = parsed.hostname.replace(/^www\./, "");
    const domains = new Set([host, ".youtube.com", ".google.com"]);
    const map = new Map();
    for (const domain of domains) {
      const list = await chrome.cookies.getAll({ domain });
      for (const cookie of list) {
        map.set(`${cookie.domain}|${cookie.name}|${cookie.path}`, {
          domain: cookie.domain,
          name: cookie.name,
          value: cookie.value,
          path: cookie.path,
          secure: cookie.secure,
          expirationDate: cookie.expirationDate
        });
      }
    }
    return [...map.values()];
  } catch {
    return [];
  }
}

async function download({ url, title, preset, referer }) {
  const config = await settings();
  if (!config.token) {
    return { ok: false, error: "Extensão não pareada. Abra o popup e digite o código." };
  }
  if (!url) {
    return { ok: false, error: "Nenhum vídeo detectado nesta página." };
  }
  const cookies = await collectCookies(url);
  const result = await request(config.port, "/download", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-VidManager-Token": config.token
    },
    body: JSON.stringify({
      url,
      title: title || undefined,
      referer: referer || url,
      preset: preset || config.preset,
      cookies
    })
  });
  if (!result.ok) {
    const badge = result.error?.includes("Token") ? "!" : "";
    await chrome.action.setBadgeText({ text: badge });
    await chrome.action.setBadgeBackgroundColor({ color: "#dc2626" });
  }
  return result;
}

chrome.runtime.onInstalled.addListener(() => {
  // removeAll evita erro de id duplicado ao recarregar a extensão em desenvolvimento
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "vidmanager-page",
      title: "Baixar com VidManager",
      contexts: ["page", "link", "video", "audio"]
    });
  });
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== "vidmanager-page") return;
  const url = info.linkUrl || info.srcUrl || info.pageUrl || tab?.url;
  const result = await download({
    url,
    title: tab?.title,
    preset: "best",
    referer: tab?.url
  });
  await chrome.action.setBadgeText({ text: result.ok ? "✓" : "!" });
  await chrome.action.setBadgeBackgroundColor({
    color: result.ok ? "#0e9f6e" : "#dc2626"
  });
  setTimeout(() => chrome.action.setBadgeText({ text: "" }), 4000);
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message?.type === "MEDIA_COUNT") {
    const tabId = sender.tab?.id;
    if (tabId) {
      chrome.action.setBadgeText({
        tabId,
        text: message.count > 0 ? String(Math.min(message.count, 9)) : ""
      });
      chrome.action.setBadgeBackgroundColor({ tabId, color: "#0e7490" });
    }
    return;
  }

  if (message?.type === "DOWNLOAD") {
    download(message).then(sendResponse);
    return true;
  }

  if (message?.type === "PAIR") {
    pair(message.code, message.port).then(sendResponse);
    return true;
  }

  if (message?.type === "PING") {
    ping(message.port).then(sendResponse);
    return true;
  }
});
