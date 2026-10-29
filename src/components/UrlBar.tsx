import { useEffect, useState, type FormEvent } from "react";
import { Download, Link2, SlidersHorizontal } from "lucide-react";
import { startDownload } from "../lib/actions";
import { useApp } from "../store/useApp";

const PRESETS = [
  { id: "best", label: "Melhor" },
  { id: "max", label: "Máxima" },
  { id: "audio", label: "Só áudio" },
];

export function UrlBar() {
  const settings = useApp((s) => s.settings);
  const openFormat = useApp((s) => s.openFormat);
  const toast = useApp((s) => s.toast);
  const [url, setUrl] = useState("");
  const [preset, setPreset] = useState(settings?.preset ?? "best");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (settings?.preset) setPreset(settings.preset);
  }, [settings?.preset]);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const value = url.trim();
    if (!value) {
      toast({ kind: "info", message: "Cole o link de um vídeo para começar." });
      return;
    }
    if (!/^https?:\/\//i.test(value)) {
      toast({ kind: "error", message: "O link precisa começar com http:// ou https://" });
      return;
    }
    if (settings?.askFormat) {
      openFormat(value);
      return;
    }
    setBusy(true);
    const ok = await startDownload({
      url: value,
      preset,
      quality: preset === "audio" ? "mp3" : undefined,
    });
    setBusy(false);
    if (ok) {
      setUrl("");
      toast({ kind: "success", message: "Download adicionado à fila." });
    }
  };

  return (
    <form className="url-panel" onSubmit={submit}>
      <div className="url-row">
        <div className="url-input-wrap">
          <Link2 size={18} className="url-icon" />
          <input
            className="url-input"
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder="Cole o link do vídeo — YouTube, Vimeo, Twitch, redes sociais…"
            spellCheck={false}
            autoComplete="off"
            aria-label="Link do vídeo"
          />
          <button
            type="button"
            className="btn btn-soft btn-analyze"
            onClick={() => openFormat(url.trim())}
            disabled={!url.trim()}
          >
            <SlidersHorizontal size={14} />
            Escolher formato
          </button>
        </div>
        <button className="btn btn-primary" type="submit" disabled={busy}>
          <Download size={16} />
          {busy ? "Enviando…" : "Baixar"}
        </button>
      </div>

      <div className="url-foot">
        <div className="seg" role="group" aria-label="Qualidade do download">
          {PRESETS.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`seg-item ${preset === item.id ? "active" : ""}`}
              onClick={() => setPreset(item.id)}
              aria-pressed={preset === item.id}
            >
              {item.label}
            </button>
          ))}
        </div>
        <span className="url-hint">
          Enter baixa na hora · links copiados aparecem como sugestão
        </span>
      </div>
    </form>
  );
}
