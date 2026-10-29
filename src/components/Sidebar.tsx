import { Home, Library, Settings as SettingsIcon, Zap } from "lucide-react";
import { useDownloadsList, useApp, type Screen } from "../store/useApp";

const ITEMS: { id: Screen; label: string; icon: typeof Home }[] = [
  { id: "home", label: "Início", icon: Home },
  { id: "library", label: "Biblioteca", icon: Library },
  { id: "settings", label: "Configurações", icon: SettingsIcon },
];

export function Sidebar() {
  const screen = useApp((s) => s.screen);
  const setScreen = useApp((s) => s.setScreen);
  const binaries = useApp((s) => s.binaries);
  const downloads = useDownloadsList();

  const active = downloads.filter((d) =>
    ["queued", "downloading", "paused"].includes(d.status)
  ).length;

  return (
    <aside className="sidebar">
      <nav className="nav" aria-label="Navegação principal">
        <span className="nav-label">Menu</span>
        {ITEMS.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              className={`nav-item ${screen === item.id ? "active" : ""}`}
              onClick={() => setScreen(item.id)}
              aria-current={screen === item.id ? "page" : undefined}
            >
              <Icon size={17} className="nav-icon" />
              <span>{item.label}</span>
              {item.id === "library" && active > 0 ? (
                <span className="nav-badge">{active}</span>
              ) : null}
            </button>
          );
        })}
      </nav>

      <div className="side-foot">
        <div className="side-tip">
          <Zap size={13} />
          <span>Downloads em paralelo ativos</span>
        </div>
        <button className="side-status" onClick={() => setScreen("settings")}>
          <span className={`status-dot ${binaries?.ytdlp ? "ok" : "warn"}`} />
          <span className="side-status-text">
            <span className="side-label">yt-dlp</span>
            <span className="side-value">
              {binaries?.ytdlp ? `v${binaries.ytdlp.version}` : "não instalado"}
            </span>
          </span>
        </button>
      </div>
    </aside>
  );
}
