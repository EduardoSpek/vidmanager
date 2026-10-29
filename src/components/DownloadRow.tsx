import {
  AlertTriangle,
  Film,
  FolderOpen,
  Pause,
  Play,
  RotateCcw,
  Trash2,
} from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { formatBytes, formatDate } from "../lib/format";
import { useApp } from "../store/useApp";
import type { Download } from "../lib/types";
import { StatusBadge } from "./StatusBadge";

export function DownloadRow({ download }: { download: Download }) {
  const toast = useApp((s) => s.toast);
  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
    } catch (error) {
      toast({ kind: "error", message: errorMessage(error) });
    }
  };

  const size =
    download.totalBytes > 0
      ? formatBytes(download.totalBytes)
      : download.downloadedBytes > 0
        ? formatBytes(download.downloadedBytes)
        : "—";

  return (
    <div className="lib-row" data-status={download.status}>
      <div className="lib-thumb">
        {download.thumbnail ? (
          <img src={download.thumbnail} alt="" loading="lazy" />
        ) : (
          <div className="lib-thumb-fallback">
            <Film size={15} />
          </div>
        )}
      </div>

      <div className="lib-main">
        <h4 className="lib-title" title={download.title || download.url}>
          {download.title || download.url}
        </h4>
        <div className="lib-sub">
          {download.uploader ? <span>{download.uploader}</span> : null}
          {download.error ? (
            <span className="lib-err" title={download.error}>
              <AlertTriangle size={11} /> {download.error}
            </span>
          ) : (
            <span className="lib-url" title={download.url}>
              {download.url}
            </span>
          )}
        </div>
      </div>

      <div className="lib-cell">
        <StatusBadge status={download.status} />
      </div>
      <div className="lib-cell lib-size">{size}</div>
      <div className="lib-cell lib-date">{formatDate(download.finishedAt || download.createdAt)}</div>

      <div className="lib-actions">
        {download.status === "downloading" ? (
          <button className="icon-btn" title="Pausar" onClick={() => act(() => api.pause(download.id))}>
            <Pause size={14} />
          </button>
        ) : null}
        {["paused", "canceled"].includes(download.status) ? (
          <button className="icon-btn" title="Retomar" onClick={() => act(() => api.resume(download.id))}>
            <Play size={14} />
          </button>
        ) : null}
        {download.status === "error" ? (
          <button className="icon-btn" title="Tentar novamente" onClick={() => act(() => api.retry(download.id))}>
            <RotateCcw size={14} />
          </button>
        ) : null}
        {download.filepath ? (
          <button
            className="icon-btn"
            title="Abrir pasta"
            onClick={() => act(() => api.revealPath(download.filepath as string))}
          >
            <FolderOpen size={14} />
          </button>
        ) : null}
        <button
          className="icon-btn danger"
          title="Remover da lista"
          onClick={() => act(() => api.remove(download.id, false))}
        >
          <Trash2 size={14} />
        </button>
      </div>
    </div>
  );
}
