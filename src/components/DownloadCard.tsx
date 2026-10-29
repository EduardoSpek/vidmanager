import {
  AlertTriangle,
  Film,
  FolderOpen,
  Pause,
  Play,
  RotateCcw,
  Trash2,
  X,
} from "lucide-react";
import type { ReactNode } from "react";
import { api, errorMessage } from "../lib/api";
import { presetLabel } from "../lib/actions";
import {
  fileName,
  formatBytes,
  formatDate,
  formatDuration,
  formatEta,
  formatSpeed,
} from "../lib/format";
import { useApp } from "../store/useApp";
import type { Download } from "../lib/types";
import { StatusBadge } from "./StatusBadge";

function IconButton({
  label,
  onClick,
  children,
  danger,
}: {
  label: string;
  onClick: () => void;
  children: ReactNode;
  danger?: boolean;
}) {
  return (
    <button
      className={`icon-btn ${danger ? "danger" : ""}`}
      onClick={onClick}
      title={label}
      aria-label={label}
    >
      {children}
    </button>
  );
}

export function DownloadCard({ download }: { download: Download }) {
  const toast = useApp((s) => s.toast);
  const percent = Math.min(100, Math.max(0, download.progress));

  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (error) {
      toast({ kind: "error", message: errorMessage(error) });
    }
  };

  const isRunning = download.status === "downloading";
  const canResume = download.status === "paused" || download.status === "canceled";
  const canRetry = download.status === "error";

  let meta: ReactNode;
  if (isRunning) {
    meta = (
      <>
        <span className="dl-percent">{percent.toFixed(0)}%</span>
        <span>
          {formatBytes(download.downloadedBytes)} de {formatBytes(download.totalBytes)}
        </span>
        <span className="dl-speed">{formatSpeed(download.speed)}</span>
        <span>restam {formatEta(download.eta)}</span>
      </>
    );
  } else if (download.status === "queued") {
    meta = <span>Aguardando na fila…</span>;
  } else if (download.status === "paused") {
    meta = (
      <>
        <span className="dl-percent">{percent.toFixed(0)}%</span>
        <span>Pausado — retoma do ponto onde parou</span>
      </>
    );
  } else if (download.status === "completed") {
    meta = (
      <>
        <span>{formatBytes(download.totalBytes || download.downloadedBytes)}</span>
        <span>{formatDate(download.finishedAt)}</span>
        {download.filepath ? (
          <span className="dl-file" title={download.filepath}>
            {fileName(download.filepath)}
          </span>
        ) : null}
      </>
    );
  } else if (download.status === "canceled") {
    meta = <span>Cancelado</span>;
  } else {
    meta = <span>Interrompido — tente novamente</span>;
  }

  return (
    <article className="dl-card" data-status={download.status}>
      <div className="dl-thumb">
        {download.thumbnail ? (
          <img src={download.thumbnail} alt="" loading="lazy" />
        ) : (
          <div className="dl-thumb-fallback">
            <Film size={20} />
          </div>
        )}
        {download.duration ? (
          <span className="dl-duration">{formatDuration(download.duration)}</span>
        ) : null}
      </div>

      <div className="dl-main">
        <div className="dl-top">
          <div className="dl-titles">
            <h3 className="dl-title" title={download.title || download.url}>
              {download.title || download.url}
            </h3>
            <div className="dl-sub">
              <StatusBadge status={download.status} />
              {download.uploader ? <span>{download.uploader}</span> : null}
              <span className="dl-dot" />
              <span>{presetLabel(download.preset)}</span>
              {download.quality && /^\d+$/.test(download.quality) ? (
                <span>{download.quality}p</span>
              ) : null}
            </div>
          </div>

          <div className="dl-actions">
            {isRunning ? (
              <IconButton label="Pausar" onClick={() => act(() => api.pause(download.id))}>
                <Pause size={15} />
              </IconButton>
            ) : null}
            {canResume ? (
              <IconButton label="Retomar" onClick={() => act(() => api.resume(download.id))}>
                <Play size={15} />
              </IconButton>
            ) : null}
            {canRetry ? (
              <IconButton label="Tentar novamente" onClick={() => act(() => api.retry(download.id))}>
                <RotateCcw size={15} />
              </IconButton>
            ) : null}
            {download.status === "queued" ? (
              <IconButton label="Cancelar" onClick={() => act(() => api.cancel(download.id))}>
                <X size={15} />
              </IconButton>
            ) : null}
            {download.status === "completed" && download.filepath ? (
              <IconButton
                label="Abrir arquivo"
                onClick={() => act(() => api.openPath(download.filepath as string))}
              >
                <Play size={15} />
              </IconButton>
            ) : null}
            {download.filepath ? (
              <IconButton
                label="Abrir pasta"
                onClick={() => act(() => api.revealPath(download.filepath as string))}
              >
                <FolderOpen size={15} />
              </IconButton>
            ) : null}
            <IconButton
              label="Remover da lista (o arquivo permanece no disco)"
              danger
              onClick={() =>
                act(async () => {
                  await api.remove(download.id, false);
                  if (download.status === "completed") {
                    toast({ kind: "info", message: "Removido da lista. O arquivo continua no disco." });
                  }
                })
              }
            >
              <Trash2 size={15} />
            </IconButton>
          </div>
        </div>

        <div className="dl-bar" aria-hidden="true">
          <div
            className="dl-bar-fill"
            style={{ width: download.status === "completed" ? "100%" : `${percent}%` }}
          />
        </div>

        <div className="dl-meta">{meta}</div>

        {download.status === "error" && download.error ? (
          <p className="dl-error">
            <AlertTriangle size={13} />
            <span>{download.error}</span>
          </p>
        ) : null}
      </div>
    </article>
  );
}
