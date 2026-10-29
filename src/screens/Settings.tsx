import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Bell,
  Check,
  Clipboard,
  Cookie,
  Copy,
  Download,
  FolderOpen,
  Gauge,
  HardDrive,
  KeyRound,
  Loader2,
  Puzzle,
  RefreshCw,
  Save,
  SlidersHorizontal,
  Terminal,
} from "lucide-react";
import { api, errorMessage } from "../lib/api";
import { refreshBinaries } from "../lib/actions";
import { useApp } from "../store/useApp";
import type { PairingInfo, Settings as SettingsType } from "../lib/types";

export function SettingsScreen() {
  const saved = useApp((s) => s.settings);
  const setSettings = useApp((s) => s.setSettings);
  const binaries = useApp((s) => s.binaries);
  const toast = useApp((s) => s.toast);

  const [draft, setDraft] = useState<SettingsType | null>(saved);
  const [saving, setSaving] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [pairing, setPairing] = useState<PairingInfo | null>(null);

  useEffect(() => {
    if (saved) setDraft(saved);
  }, [saved]);

  useEffect(() => {
    api
      .pairingInfo()
      .then(setPairing)
      .catch(() => undefined);
  }, [saved?.apiPort]);

  const dirty = useMemo(
    () => Boolean(draft && saved && JSON.stringify(draft) !== JSON.stringify(saved)),
    [draft, saved]
  );

  const set = <K extends keyof SettingsType>(key: K, value: SettingsType[K]) =>
    setDraft((current) => (current ? { ...current, [key]: value } : current));

  const save = async () => {
    if (!draft) return;
    setSaving(true);
    try {
      const result = await api.saveSettings(draft);
      setSettings(result);
      setDraft(result);
      toast({ kind: "success", message: "Configurações salvas." });
    } catch (error) {
      toast({ kind: "error", message: errorMessage(error) });
    } finally {
      setSaving(false);
    }
  };

  const pickFolder = async () => {
    try {
      const path = await api.pickFolder();
      if (path) set("outputDir", path);
    } catch (error) {
      toast({ kind: "error", message: errorMessage(error) });
    }
  };

  const install = async (which: "ytdlp" | "ffmpeg" | "deno") => {
    setBusy(which);
    try {
      const version =
        which === "ytdlp"
          ? await api.installYtdlp()
          : which === "ffmpeg"
            ? await api.installFfmpeg()
            : await api.installDeno();
      await refreshBinaries();
      const label = which === "ytdlp" ? "yt-dlp" : which === "ffmpeg" ? "ffmpeg" : "Deno";
      toast({ kind: "success", message: `${label} instalado: ${version}` });
    } catch (error) {
      toast({ kind: "error", message: errorMessage(error) });
    } finally {
      setBusy(null);
    }
  };

  const copy = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast({ kind: "success", message: `${label} copiado.` });
    } catch {
      toast({ kind: "error", message: "Não foi possível copiar para a área de transferência." });
    }
  };

  const rotate = async () => {
    try {
      const info = await api.rotatePairing();
      setPairing(info);
      toast({ kind: "success", message: "Novo código de pareamento gerado." });
    } catch (error) {
      toast({ kind: "error", message: errorMessage(error) });
    }
  };

  if (!draft) {
    return (
      <div className="screen">
        <div className="fmt-loading">
          <Loader2 size={24} className="spin" />
          <p>Carregando configurações…</p>
        </div>
      </div>
    );
  }

  return (
    <div className="screen">
      <div className="screen-head">
        <div>
          <span className="eyebrow">Preferências</span>
          <h1 className="screen-title">Configurações</h1>
          <p className="screen-sub">Destino, qualidade padrão, desempenho e integrações.</p>
        </div>
        <button className="btn btn-primary" onClick={save} disabled={!dirty || saving}>
          {saving ? <Loader2 size={15} className="spin" /> : <Save size={15} />}
          {saving ? "Salvando…" : "Salvar alterações"}
        </button>
      </div>

      <div className="settings-grid">
        <Card icon={<HardDrive size={16} />} title="Destino" sub="Onde os arquivos são salvos" wide>
          <Row
            label="Pasta de destino"
            hint="Cada canal recebe uma subpasta, conforme o modelo abaixo."
          >
            <div className="input-group">
              <input
                className="input mono"
                value={draft.outputDir}
                onChange={(event) => set("outputDir", event.target.value)}
                spellCheck={false}
              />
              <button className="btn btn-soft" onClick={pickFolder} type="button">
                <FolderOpen size={14} />
                Escolher
              </button>
            </div>
          </Row>
          <Row
            label="Modelo de nome do arquivo"
            hint="Variáveis do yt-dlp: %(title)s, %(uploader)s, %(id)s, %(ext)s, %(resolution)s. Use %(uploader,channel|Outros)s para cair num valor alternativo quando o campo não existir."
          >
            <input
              className="input mono"
              value={draft.filenameTemplate}
              onChange={(event) => set("filenameTemplate", event.target.value)}
              spellCheck={false}
            />
          </Row>
        </Card>

        <Card icon={<SlidersHorizontal size={16} />} title="Qualidade padrão" sub="Usada quando você baixa sem escolher formato">
          <Row label="Predefinição">
            <select
              className="select"
              value={draft.preset}
              onChange={(event) => set("preset", event.target.value)}
            >
              <option value="best">Melhor qualidade compatível</option>
              <option value="max">Máxima qualidade bruta</option>
              <option value="audio">Só áudio</option>
            </select>
          </Row>
          <Row label="Formato do áudio">
            <select
              className="select"
              value={draft.audioFormat}
              onChange={(event) => set("audioFormat", event.target.value)}
            >
              <option value="mp3">MP3</option>
              <option value="m4a">M4A (AAC)</option>
              <option value="opus">Opus</option>
              <option value="flac">FLAC</option>
              <option value="wav">WAV</option>
            </select>
          </Row>
          <Row label="Sempre perguntar o formato" hint="Abre o painel de formatos antes de baixar.">
            <Switch checked={draft.askFormat} onChange={(v) => set("askFormat", v)} label="Sempre perguntar" />
          </Row>
          <Row label="Embutir metadados" hint="Título, canal e capa ficam dentro do arquivo.">
            <Switch
              checked={draft.embedMetadata}
              onChange={(v) => set("embedMetadata", v)}
              label="Embutir metadados"
            />
          </Row>
          <Row label="Embutir capa">
            <Switch
              checked={draft.embedThumbnail}
              onChange={(v) => set("embedThumbnail", v)}
              label="Embutir capa"
            />
          </Row>
          <Row label="Baixar legendas" hint="Inclui legendas em português e inglês quando existirem.">
            <div className="stack-end">
              <Switch checked={draft.embedSubs} onChange={(v) => set("embedSubs", v)} label="Baixar legendas" />
              {draft.embedSubs ? (
                <input
                  className="input input-sm mono"
                  value={draft.subLangs}
                  onChange={(event) => set("subLangs", event.target.value)}
                  placeholder="pt.*,en.*"
                />
              ) : null}
            </div>
          </Row>
        </Card>

        <Card icon={<Gauge size={16} />} title="Desempenho" sub="Quanto o app pode consumir">
          <Row label="Downloads simultâneos" hint="Recomendado 2 a 4.">
            <select
              className="select"
              value={draft.maxConcurrent}
              onChange={(event) => set("maxConcurrent", Number(event.target.value))}
            >
              {[1, 2, 3, 4, 5, 6].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
          </Row>
          <Row label="Limite de velocidade" hint="Em KB/s por download. Use 0 para ilimitado.">
            <div className="input-group">
              <input
                className="input input-sm"
                type="number"
                min={0}
                step={128}
                value={draft.speedLimitKb}
                onChange={(event) => set("speedLimitKb", Math.max(0, Number(event.target.value) || 0))}
              />
              <span className="unit">KB/s</span>
            </div>
          </Row>
        </Card>

        <Card icon={<Bell size={16} />} title="Sistema" sub="Comportamento do aplicativo">
          <Row label="Monitorar área de transferência" hint="Sugere baixar quando você copia um link de vídeo.">
            <Switch
              checked={draft.clipboardMonitor}
              onChange={(v) => set("clipboardMonitor", v)}
              label="Monitorar área de transferência"
            />
          </Row>
          <Row label="Notificar ao concluir">
            <Switch
              checked={draft.notifyOnComplete}
              onChange={(v) => set("notifyOnComplete", v)}
              label="Notificar ao concluir"
            />
          </Row>
          <Row
            label="Cookies do navegador"
            hint="Necessário para vídeos com login, idade ou membros."
          >
            <select
              className="select"
              value={draft.cookiesBrowser}
              onChange={(event) => set("cookiesBrowser", event.target.value)}
            >
              <option value="">Não usar</option>
              <option value="chrome">Chrome</option>
              <option value="edge">Edge</option>
              <option value="firefox">Firefox</option>
              <option value="brave">Brave</option>
              <option value="opera">Opera</option>
            </select>
          </Row>
        </Card>

        <Card
          icon={<Puzzle size={16} />}
          title="Extensão e API local"
          sub="Receba vídeos direto do Chrome ou Edge"
          wide
        >
          <Row label="API local" hint="Reinicie o aplicativo após mudar a porta ou desativar.">
            <div className="stack-end">
              <Switch
                checked={draft.apiEnabled}
                onChange={(v) => set("apiEnabled", v)}
                label="API local"
              />
              <div className="input-group">
                <span className="unit">porta</span>
                <input
                  className="input input-sm"
                  type="number"
                  min={1024}
                  max={65535}
                  value={draft.apiPort}
                  onChange={(event) => set("apiPort", Number(event.target.value) || 41300)}
                />
              </div>
            </div>
          </Row>

          <div className="pair-box">
            <div className="pair-left">
              <span className="side-label">Código de pareamento</span>
              <div className="pair-code-row">
                <span className="pair-code mono">{pairing?.code ?? "••••••"}</span>
                <button className="icon-btn" onClick={rotate} title="Gerar novo código">
                  <RefreshCw size={14} />
                </button>
              </div>
              <span className="pair-hint">
                Digite este código na extensão. Válido por 24 horas.
              </span>
            </div>

            <div className="pair-right">
              <span className="side-label">Token da API</span>
              <div className="input-group">
                <span className="token mono" title={pairing?.token}>
                  {pairing ? `${pairing.token.slice(0, 6)}••••••${pairing.token.slice(-4)}` : "—"}
                </span>
                <button
                  className="btn btn-soft"
                  onClick={() => pairing && copy(pairing.token, "Token")}
                  disabled={!pairing}
                >
                  <Copy size={14} />
                  Copiar
                </button>
              </div>
              <span className="pair-hint">
                A extensão guarda o token depois do pareamento.
              </span>
            </div>
          </div>

          <div className="install-steps">
            <div className="install-step">
              <span className="step-num">1</span>
              <span>
                Abra <code>chrome://extensions</code> (ou <code>edge://extensions</code>) e ative o modo
                desenvolvedor.
              </span>
            </div>
            <div className="install-step">
              <span className="step-num">2</span>
              <span>
                Clique em <strong>Carregar sem compactação</strong> e escolha a pasta{" "}
                <code>extension</code> do projeto.
              </span>
            </div>
            <div className="install-step">
              <span className="step-num">3</span>
              <span>
                Abra a extensão, cole o código <strong>{pairing?.code ?? "••••••"}</strong> e pronto.
              </span>
            </div>
          </div>
        </Card>

        <Card
          icon={<Terminal size={16} />}
          title="Binários"
          sub="Motores usados para extrair e converter vídeos"
          wide
        >
          <div className="bin-row">
            <div className="bin-info">
              <span className="bin-name">yt-dlp</span>
              <span className={`bin-version ${binaries?.ytdlp ? "ok" : "bad"}`}>
                {binaries?.ytdlp ? `v${binaries.ytdlp.version}` : "não instalado"}
              </span>
              <span className="bin-path mono" title={binaries?.ytdlp?.path}>
                {binaries?.ytdlp?.path ?? "—"}
              </span>
            </div>
            <button
              className="btn btn-soft"
              onClick={() => install("ytdlp")}
              disabled={busy !== null}
            >
              {busy === "ytdlp" ? <Loader2 size={14} className="spin" /> : <Download size={14} />}
              {binaries?.ytdlp ? "Atualizar" : "Instalar"}
            </button>
          </div>

          <div className="bin-row">
            <div className="bin-info">
              <span className="bin-name">ffmpeg</span>
              <span className={`bin-version ${binaries?.ffmpeg ? "ok" : "bad"}`}>
                {binaries?.ffmpeg?.version ?? "não encontrado"}
              </span>
              <span className="bin-path mono" title={binaries?.ffmpeg?.path}>
                {binaries?.ffmpeg?.path ?? "—"}
              </span>
            </div>
            <button
              className="btn btn-soft"
              onClick={() => install("ffmpeg")}
              disabled={busy !== null}
            >
              {busy === "ffmpeg" ? <Loader2 size={14} className="spin" /> : <Download size={14} />}
              {binaries?.ffmpeg ? "Reinstalar" : "Instalar"}
            </button>
          </div>

          <div className="bin-row">
            <div className="bin-info">
              <span className="bin-name">JavaScript</span>
              <span className={`bin-version ${binaries?.jsRuntime ? "ok" : "bad"}`}>
                {binaries?.jsRuntime?.version ?? "não encontrado"}
              </span>
              <span className="bin-path mono" title={binaries?.jsRuntime?.path}>
                {binaries?.jsRuntime?.path ?? "—"}
              </span>
            </div>
            <button className="btn btn-soft" onClick={() => install("deno")} disabled={busy !== null}>
              {busy === "deno" ? <Loader2 size={14} className="spin" /> : <Download size={14} />}
              {binaries?.jsRuntime ? "Reinstalar Deno" : "Instalar Deno"}
            </button>
          </div>

          <div className="bin-note">
            <KeyRound size={13} />
            <span>
              O Deno é o runtime JavaScript usado pelo YouTube. Sem ele, alguns vídeos podem não
              oferecer todas as resoluções. Se um download falhar com erro de extração, atualize o
              yt-dlp.
            </span>
          </div>

          <button
            className="btn btn-ghost"
            onClick={() => binaries && api.openPath(binaries.binDir).catch(() => undefined)}
          >
            <FolderOpen size={14} />
            Abrir pasta de binários
          </button>
        </Card>
      </div>

      {dirty ? (
        <div className="save-bar">
          <span>
            <span className="status-dot warn" /> Alterações não salvas
          </span>
          <div className="save-bar-actions">
            <button className="btn btn-ghost" onClick={() => setDraft(saved)}>
              Descartar
            </button>
            <button className="btn btn-primary" onClick={save} disabled={saving}>
              {saving ? <Loader2 size={15} className="spin" /> : <Check size={15} />}
              Salvar
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function Card({
  icon,
  title,
  sub,
  children,
  wide,
}: {
  icon: ReactNode;
  title: string;
  sub?: string;
  children: ReactNode;
  wide?: boolean;
}) {
  return (
    <section className={`card ${wide ? "span-2" : ""}`}>
      <header className="card-head">
        <span className="card-icon">{icon}</span>
        <div>
          <h2>{title}</h2>
          {sub ? <p>{sub}</p> : null}
        </div>
      </header>
      <div className="card-body">{children}</div>
    </section>
  );
}

function Row({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="setting-row">
      <div className="setting-label">
        <span>{label}</span>
        {hint ? <small>{hint}</small> : null}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

function Switch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`switch ${checked ? "on" : ""}`}
      onClick={() => onChange(!checked)}
    >
      <span className="switch-knob" />
    </button>
  );
}
