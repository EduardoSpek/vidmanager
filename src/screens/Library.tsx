import { useState } from "react";
import { Inbox, Search, Trash2 } from "lucide-react";
import { DownloadRow } from "../components/DownloadRow";
import { EmptyState } from "../components/EmptyState";
import { api, errorMessage } from "../lib/api";
import { useApp, useDownloadsList } from "../store/useApp";

type Filter = "all" | "active" | "completed" | "error";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "Todos" },
  { id: "active", label: "Em andamento" },
  { id: "completed", label: "Concluídos" },
  { id: "error", label: "Com erro" },
];

export function Library() {
  const downloads = useDownloadsList();
  const search = useApp((s) => s.search);
  const setSearch = useApp((s) => s.setSearch);
  const toast = useApp((s) => s.toast);
  const [filter, setFilter] = useState<Filter>("all");

  const counts = {
    all: downloads.length,
    active: downloads.filter((d) =>
      ["queued", "downloading", "paused"].includes(d.status)
    ).length,
    completed: downloads.filter((d) => d.status === "completed").length,
    error: downloads.filter((d) => d.status === "error").length,
  };

  const filtered = downloads.filter((download) => {
    if (filter === "active" && !["queued", "downloading", "paused"].includes(download.status))
      return false;
    if (filter === "completed" && download.status !== "completed") return false;
    if (filter === "error" && download.status !== "error") return false;
    if (!search.trim()) return true;
    const needle = search.trim().toLowerCase();
    return (
      download.title.toLowerCase().includes(needle) ||
      (download.uploader ?? "").toLowerCase().includes(needle) ||
      download.url.toLowerCase().includes(needle)
    );
  });

  const clearCompleted = async () => {
    try {
      const removed = await api.clearCompleted();
      toast({ kind: "info", message: `${removed} item(ns) removidos da lista.` });
    } catch (error) {
      toast({ kind: "error", message: errorMessage(error) });
    }
  };

  return (
    <div className="screen">
      <div className="screen-head">
        <div>
          <span className="eyebrow">Histórico</span>
          <h1 className="screen-title">Biblioteca</h1>
          <p className="screen-sub">Todos os downloads deste computador.</p>
        </div>
        <button
          className="btn btn-ghost"
          onClick={clearCompleted}
          disabled={counts.completed === 0}
        >
          <Trash2 size={15} />
          Limpar concluídos
        </button>
      </div>

      <div className="lib-toolbar">
        <div className="search">
          <Search size={15} />
          <input
            id="lib-search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Buscar por título, canal ou link"
            aria-label="Buscar downloads"
          />
        </div>
        <div className="chips">
          {FILTERS.map((item) => (
            <button
              key={item.id}
              className={`chip ${filter === item.id ? "active" : ""}`}
              onClick={() => setFilter(item.id)}
            >
              {item.label}
              <span className="chip-count">{counts[item.id]}</span>
            </button>
          ))}
        </div>
      </div>

      {filtered.length === 0 ? (
        <EmptyState
          icon={<Inbox size={26} />}
          title={downloads.length === 0 ? "Sua biblioteca está vazia" : "Nada encontrado"}
          text={
            downloads.length === 0
              ? "Os downloads que você fizer aparecem aqui, com histórico e caminho do arquivo."
              : "Ajuste a busca ou troque o filtro para ver outros downloads."
          }
        />
      ) : (
        <div className="lib-list">
          {filtered.map((download) => (
            <DownloadRow key={download.id} download={download} />
          ))}
        </div>
      )}
    </div>
  );
}
