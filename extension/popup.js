const $ = (id) => document.getElementById(id);
let currentTab = null;

function cleanTitle(title) {
  if (!title) return "Vídeo";
  return title
    .replace(/^\(\d+\)\s*/, "")
    .replace(/\s*[-|–]\s*YouTube\s*$/i, "")
    .replace(/\s*[-|–]\s*Vimeo\s*$/i, "")
    .replace(/\s*\|?\s*Twitch\s*$/i, "")
    .replace(/\s*[-|–]\s*Dailymotion\s*$/i, "")
    .trim();
}

function setStatus(message, kind) {
  const box = $("status");
  box.textContent = message;
  box.className = `status-msg ${kind}`;
}

function clearStatus() {
  const box = $("status");
  box.className = "status-msg hidden";
  box.textContent = "";
}

function showPair() {
  $("pair-view").classList.remove("hidden");
  $("main-view").classList.add("hidden");
}

function showMain() {
  $("pair-view").classList.add("hidden");
  $("main-view").classList.remove("hidden");
}

async function checkConnection(port) {
  const dot = $("dot");
  const text = $("conn-text");
  const result = await chrome.runtime.sendMessage({ type: "PING", port });
  if (result?.ok) {
    dot.className = "dot online";
    text.textContent = "conectado ao aplicativo";
  } else {
    dot.className = "dot offline";
    text.textContent = "aplicativo fechado ou API desativada";
  }
}

async function detectOnPage(tab) {
  if (!tab?.id) return;
  try {
    chrome.tabs.sendMessage(tab.id, { type: "GET_INFO" }, (info) => {
      if (chrome.runtime.lastError || !info) {
        $("detect").textContent = "Extração via yt-dlp — funciona em milhares de sites";
        return;
      }
      if (info.title) $("page-title").textContent = cleanTitle(info.title);
      $("detect").textContent =
        info.mediaCount > 0
          ? `${info.mediaCount} mídia(s) detectada(s) nesta página`
          : "Sem elemento de vídeo visível — o yt-dlp cuida da extração";
    });
  } catch {
    $("detect").textContent = "Extração via yt-dlp";
  }
}

async function init() {
  const stored = await chrome.storage.local.get({ port: 41300, token: "", preset: "best" });
  $("port").value = stored.port;
  $("preset").value = stored.preset || "best";

  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  currentTab = tabs[0] || null;

  if (currentTab) {
    $("page-title").textContent = cleanTitle(currentTab.title);
    $("page-url").textContent = currentTab.url || "—";
    $("page-url").title = currentTab.url || "";
    await detectOnPage(currentTab);
  }

  if (stored.token) {
    showMain();
    await checkConnection(stored.port);
  } else {
    showPair();
    const dot = $("dot");
    dot.className = "dot offline";
    $("conn-text").textContent = "não pareado";
  }

  $("code").addEventListener("input", (event) => {
    event.target.value = event.target.value.replace(/\D/g, "").slice(0, 6);
  });

  $("pair-btn").addEventListener("click", async () => {
    clearStatus();
    const code = $("code").value.trim();
    const port = Number($("port").value) || 41300;
    if (code.length !== 6) {
      setStatus("Digite os 6 dígitos do código de pareamento.", "err");
      return;
    }
    $("pair-btn").disabled = true;
    const result = await chrome.runtime.sendMessage({ type: "PAIR", code, port });
    $("pair-btn").disabled = false;
    if (result?.ok) {
      setStatus("Pareado com sucesso! Já pode enviar vídeos.", "ok");
      showMain();
      await checkConnection(port);
    } else {
      setStatus(result?.error || "Não foi possível parear.", "err");
    }
  });

  $("preset").addEventListener("change", async (event) => {
    await chrome.storage.local.set({ preset: event.target.value });
  });

  $("send-btn").addEventListener("click", async () => {
    clearStatus();
    if (!currentTab?.url) {
      setStatus("Nenhuma página ativa.", "err");
      return;
    }
    $("send-btn").disabled = true;
    $("send-btn").textContent = "Enviando…";
    const result = await chrome.runtime.sendMessage({
      type: "DOWNLOAD",
      url: currentTab.url,
      title: cleanTitle(currentTab.title),
      referer: currentTab.url,
      preset: $("preset").value
    });
    $("send-btn").disabled = false;
    $("send-btn").textContent = "Baixar com VidManager";
    if (result?.ok) {
      setStatus(
        result.duplicate
          ? "Este vídeo já está na fila do VidManager."
          : "Enviado! O download começou no aplicativo.",
        "ok"
      );
    } else {
      setStatus(result?.error || "Falha ao enviar para o VidManager.", "err");
    }
  });

  $("unpair").addEventListener("click", async () => {
    await chrome.storage.local.set({ token: "" });
    showPair();
    $("dot").className = "dot offline";
    $("conn-text").textContent = "não pareado";
    setStatus("Extensão despareada.", "ok");
  });
}

init().catch((error) => {
  setStatus(`Erro ao iniciar a extensão: ${error?.message || error}`, "err");
});
