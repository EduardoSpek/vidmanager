import { useEffect, useState, type ReactNode } from "react";
import {
  Check,
  Download,
  Film,
  ListVideo,
  Loader2,
  Music,
  TriangleAlert,
  X,
} from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { startDownload } from "../lib/actions";
import { formatBytes, formatDuration, resolutionLabel } from "../lib/format";
import { useApp } from "../store/useApp";
import type { ProbeResult, StartRequest } from "../lib/types";

export function FormatModal() {
  const url = useApp((s) => s.formatFor);
  const close = useApp((s) => s.closeFormat);
  const toast = useApp((s) => s.toast);

  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [choice, setChoice] = useState<string>("best");
  const [playlist, setPlaylist] = useState(false);
  const [range, setRange] = useState("");
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    if (!url) return;
    let alive = true;
    setProbe(null);
    setError(null);
    setChoice("best");
    setPlaylist(false);
    setRange("");
    setLoading(true);
    api
      .probe(url)
      .then((result) => {
        if (alive) setProbe(result);
      })
      .catch((err) => {
        if (alive) setError(errorMessage(err));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [url]);

  useEffect(() => {
    if (!url) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [url, close]);

  if (!url) return null;

  const start = async () => {
    const request: StartRequest = { url };
    if (choice === "audio") {
      request.preset = "audio";
      request.quality = "mp3";
    } else if (choice === "max") {
      request.preset = "max";
    } else {
      request.preset = "best";
      if (choice !== "best") request.quality = choice;
    }
    if (playlist) {
      request.playlistItems = range.trim() || "1-";
    }
    setStarting(true);
    const ok = await startDownload(request);
    setStarting(false);
    if (ok) {
      close();
      toast({ kind: "success", message: "Download adicionado à fila." });
    }
  };

  return (
    <div
      className="modal-overlay"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) close();
      }}
    >
      <div className="modal modal-fmt" role="dialog" aria-modal="true" aria-label="Escolher formato">
        <div className="modal-head">
          <div>
            <h2 className="modal-title">Escolher formato</h2>
            <p className="modal-sub">Selecione a qualidade antes de baixar</p>
          </div>
          <button className="icon-btn" onClick={close} aria-label="Fechar">
            <X size={16} />
          </button>
        </div>

        {loading ? (
          <div className="fmt-loading">
            <Loader2 size={26} className="spin" />
            <p>Analisando o vídeo…</p>
            <span>Isso leva alguns segundos.</span>
          </div>
        ) : error ? (
          <div className="fmt-error">
            <TriangleAlert size={26} />
            <p>{error}</p>
            <span className="mono ellipsis" title={url}>
              {url}
            </span>
          </div>
        ) : probe ? (
          <div className="modal-body">
            <div className="fmt-layout">
              <div className="fmt-preview">
                {probe.thumbnail ? (
                  <img src={probe.thumbnail} alt="" />
                ) : (
                  <div className="fmt-preview-fallback">
                    <Film size={26} />
                  </div>
                )}
                {probe.duration ? (
                  <span className="dl-duration">{formatDuration(probe.duration)}</span>
                ) : null}
              </div>
              <div className="fmt-info">
                <h3 className="fmt-title">{probe.title}</h3>
                <p className="fmt-meta">
                  {probe.uploader ? <span>{probe.uploader}</span> : null}
                  {probe.isPlaylist ? (
                    <span className="fmt-tag">
                      <ListVideo size={12} />
                      Playlist{probe.playlistCount ? ` · ${probe.playlistCount} itens` : ""}
                    </span>
                  ) : null}
                </p>
                <p className="fmt-url" title={probe.url}>
                  {probe.url}
                </p>
              </div>
            </div>

            <div className="fmt-options" role="radiogroup" aria-label="Formato">
              <OptionRow
                active={choice === "best"}
                onClick={() => setChoice("best")}
                title="Melhor qualidade"
                subtitle="H.264/AAC quando disponível — abre em qualquer player"
                recommended
              />
              <OptionRow
                active={choice === "max"}
                onClick={() => setChoice("max")}
                title="Máxima qualidade"
                subtitle="Melhor vídeo + áudio, sem preferência de codec (arquivo maior)"
              />
              {probe.resolutions.map((resolution) => (
                <OptionRow
                  key={resolution.height}
                  active={choice === String(resolution.height)}
                  onClick={() => setChoice(String(resolution.height))}
                  title={`${resolutionLabel(resolution.height)} · ${resolution.height}p`}
                  subtitle={[
                    resolution.fps ? `${Math.round(resolution.fps)} fps` : null,
                    resolution.ext ? resolution.ext.toUpperCase() : null,
                    resolution.filesize ? `~${formatBytes(resolution.filesize)}` : null,
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                />
              ))}
              <OptionRow
                active={choice === "audio"}
                onClick={() => setChoice("audio")}
                title="Só áudio (MP3)"
                subtitle="Extrai a faixa de áudio em alta qualidade"
                icon={<Music size={14} />}
              />
            </div>

            {probe.isPlaylist ? (
              <div className="fmt-playlist">
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={playlist}
                    onChange={(event) => setPlaylist(event.target.checked)}
                  />
                  <span>Baixar a playlist inteira</span>
                </label>
                {playlist ? (
                  <input
                    className="input"
                    value={range}
                    onChange={(event) => setRange(event.target.value)}
                    placeholder="Intervalo, ex.: 1-10 (vazio = tudo)"
                  />
                ) : null}
              </div>
            ) : null}
          </div>
        ) : null}

        <div className="modal-foot">
          <button className="btn btn-ghost" onClick={close}>
            Cancelar
          </button>
          <button
            className="btn btn-primary"
            onClick={start}
            disabled={!probe || starting}
          >
            {starting ? <Loader2 size={15} className="spin" /> : <Download size={15} />}
            {starting ? "Adicionando…" : "Baixar agora"}
          </button>
        </div>
      </div>
    </div>
  );
}

function OptionRow({
  active,
  onClick,
  title,
  subtitle,
  recommended,
  icon,
}: {
  active: boolean;
  onClick: () => void;
  title: string;
  subtitle: string;
  recommended?: boolean;
  icon?: ReactNode;
}) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={active}
      className={`fmt-option ${active ? "selected" : ""}`}
      onClick={onClick}
    >
      <span className="fmt-radio">{active ? <Check size={11} /> : null}</span>
      <span className="fmt-opt-text">
        <span className="fmt-opt-title">
          {icon}
          {title}
          {recommended ? <span className="fmt-badge">Recomendado</span> : null}
        </span>
        <span className="fmt-opt-sub">{subtitle}</span>
      </span>
    </button>
  );
}
