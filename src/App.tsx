import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Titlebar } from "./components/Titlebar";
import { Sidebar } from "./components/Sidebar";
import { Toasts } from "./components/Toasts";
import { FormatModal } from "./components/FormatModal";
import { Home } from "./screens/Home";
import { Library } from "./screens/Library";
import { SettingsScreen } from "./screens/Settings";
import { useApp } from "./store/useApp";
import { initApp, isTauri, refreshBinaries, startFromUrl } from "./lib/actions";
import type { Download, Settings } from "./lib/types";

function shortUrl(url: string): string {
  try {
    const parsed = new URL(url);
    const host = parsed.hostname.replace(/^www\./, "");
    const path = parsed.pathname.length > 1 ? parsed.pathname : "";
    const full = `${host}${path}`;
    return full.length > 46 ? `${full.slice(0, 46)}…` : full;
  } catch {
    return url.slice(0, 46);
  }
}

export default function App() {
  const screen = useApp((s) => s.screen);

  useEffect(() => {
    void initApp();
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let unsubs: UnlistenFn[] = [];
    let disposed = false;
    let lastClipboard = "";

    const setup = async () => {
      const register = async <T,>(event: string, handler: (payload: T) => void) => {
        const unlisten = await listen<T>(event, (e) => handler(e.payload));
        if (disposed) unlisten();
        else unsubs.push(unlisten);
      };

      await register<Download>("download:update", (payload) =>
        useApp.getState().upsert(payload)
      );
      await register<string>("download:removed", (payload) =>
        useApp.getState().removeDownload(payload)
      );
      await register<{ kind: "info" | "success" | "error"; message: string }>(
        "toast",
        (payload) => useApp.getState().toast(payload)
      );
      await register<Settings>("settings:changed", (payload) =>
        useApp.getState().setSettings(payload)
      );
      await register<null>("binaries:changed", () => void refreshBinaries());
      await register<{ id: string; title: string }>("extension:download", (payload) => {
        const state = useApp.getState();
        state.setScreen("home");
        state.toast({
          kind: "success",
          message: `Recebido da extensão: ${payload.title || "novo download"}`,
        });
      });
      await register<{ url: string }>("clipboard:url", (payload) => {
        const url = payload.url;
        if (!url || url === lastClipboard) return;
        lastClipboard = url;
        const state = useApp.getState();
        const exists = Object.values(state.downloads).some(
          (d) => d.url === url && !["error", "canceled"].includes(d.status)
        );
        if (exists) return;
        state.toast({
          kind: "info",
          message: `Link copiado · ${shortUrl(url)}`,
          actionLabel: "Baixar",
          onAction: () => void startFromUrl(url),
        });
      });
    };

    void setup();
    return () => {
      disposed = true;
      unsubs.forEach((unlisten) => unlisten());
      unsubs = [];
    };
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
        event.preventDefault();
        useApp.getState().setScreen("library");
        window.setTimeout(() => document.getElementById("lib-search")?.focus(), 60);
      }
      if ((event.ctrlKey || event.metaKey) && event.key === ",") {
        event.preventDefault();
        useApp.getState().setScreen("settings");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="app-shell">
      <Titlebar />
      <div className="app-body">
        <Sidebar />
        <main className="main">
          <div className="main-inner">
            {screen === "home" ? <Home /> : null}
            {screen === "library" ? <Library /> : null}
            {screen === "settings" ? <SettingsScreen /> : null}
          </div>
        </main>
      </div>
      <FormatModal />
      <Toasts />
    </div>
  );
}
