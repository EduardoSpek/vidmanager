import { getCurrentWindow } from "@tauri-apps/api/window";
import { Download, Minus, Square, X } from "lucide-react";
import { useApp } from "../store/useApp";

export function Titlebar() {
  const version = useApp((s) => s.info?.version);
  const win = getCurrentWindow();

  return (
    <header className="titlebar">
      <div className="brand" data-tauri-drag-region>
        <div className="brand-mark">
          <Download size={14} strokeWidth={2.6} />
        </div>
        <span className="brand-name" data-tauri-drag-region>
          VidManager
        </span>
        {version ? <span className="brand-version">v{version}</span> : null}
      </div>
      <div
        className="titlebar-spacer"
        data-tauri-drag-region
        onDoubleClick={() => win.toggleMaximize()}
      />
      <div className="win-controls">
        <button className="win-btn" onClick={() => win.minimize()} aria-label="Minimizar">
          <Minus size={15} />
        </button>
        <button className="win-btn" onClick={() => win.toggleMaximize()} aria-label="Maximizar">
          <Square size={11.5} />
        </button>
        <button className="win-btn close" onClick={() => win.close()} aria-label="Fechar">
          <X size={15} />
        </button>
      </div>
    </header>
  );
}
