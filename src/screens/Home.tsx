import { Gauge, Inbox, Link2, Sparkles, TriangleAlert } from "lucide-react";
import { UrlBar } from "../components/UrlBar";
import { DownloadCard } from "../components/DownloadCard";
import { DownloadRow } from "../components/DownloadRow";
import { EmptyState } from "../components/EmptyState";
import { useActiveDownloads, useDownloadsList } from "../store/useApp";
import { formatSpeed } from "../lib/format";

export function Home() {
  const active = useActiveDownloads();
  const downloads = useDownloadsList();
  const failed = downloads.filter((d) => d.status === "error");
  const recent = downloads.filter((d) => d.status === "completed").slice(0, 4);
  const totalSpeed = active.reduce((sum, d) => sum + (d.speed || 0), 0);

  return (
    <div className="screen">
      <div className="screen-head">
        <div>
          <span className="eyebrow">VidManager</span>
          <h1 className="screen-title">Baixar vídeo</h1>
          <p className="screen-sub">
            Cole um link e o download começa na melhor qualidade disponível.
          </p>
        </div>
        {active.length > 0 ? (
          <div className="stat-card">
            <div className="stat-icon">
              <Gauge size={17} />
            </div>
            <div className="stat-body">
              <span className="stat-value">{formatSpeed(totalSpeed)}</span>
              <span className="stat-label">
                {active.length} {active.length === 1 ? "download ativo" : "downloads ativos"}
              </span>
            </div>
          </div>
        ) : null}
      </div>

      <UrlBar />

      {active.length > 0 ? (
        <section className="section">
          <div className="section-head">
            <h2 className="section-title">Em andamento</h2>
            <span className="section-count">{active.length}</span>
          </div>
          <div className="dl-list">
            {active.map((download) => (
              <DownloadCard key={download.id} download={download} />
            ))}
          </div>
        </section>
      ) : null}

      {failed.length > 0 ? (
        <section className="section">
          <div className="section-head">
            <h2 className="section-title">
              <TriangleAlert size={15} /> Precisa de atenção
            </h2>
            <span className="section-count">{failed.length}</span>
          </div>
          <div className="dl-list">
            {failed.slice(0, 3).map((download) => (
              <DownloadCard key={download.id} download={download} />
            ))}
          </div>
        </section>
      ) : null}

      {recent.length > 0 ? (
        <section className="section">
          <div className="section-head">
            <h2 className="section-title">Concluídos recentemente</h2>
          </div>
          <div className="lib-list">
            {recent.map((download) => (
              <DownloadRow key={download.id} download={download} />
            ))}
          </div>
        </section>
      ) : null}

      {downloads.length === 0 ? (
        <EmptyState
          icon={<Inbox size={26} />}
          title="Nenhum download ainda"
          text="Cole um link acima para começar. O VidManager usa o yt-dlp para suportar milhares de sites."
        >
          <div className="steps">
            <div className="step">
              <span className="step-icon">
                <Link2 size={14} />
              </span>
              <div>
                <strong>Cole o link</strong>
                <span>YouTube, Vimeo, Twitch, redes sociais e mais.</span>
              </div>
            </div>
            <div className="step">
              <span className="step-icon">
                <Sparkles size={14} />
              </span>
              <div>
                <strong>Escolha ou deixe no automático</strong>
                <span>Melhor qualidade compatível, áudio ou resolução específica.</span>
              </div>
            </div>
            <div className="step">
              <span className="step-icon">
                <Gauge size={14} />
              </span>
              <div>
                <strong>Acompanhe em tempo real</strong>
                <span>Pause, retome e limite a velocidade quando quiser.</span>
              </div>
            </div>
          </div>
        </EmptyState>
      ) : null}
    </div>
  );
}
