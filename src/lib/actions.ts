import { api, errorMessage } from "./api";
import type { StartRequest } from "./types";
import { useApp } from "../store/useApp";

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function startDownload(req: StartRequest): Promise<boolean> {
  try {
    await api.start(req);
    return true;
  } catch (error) {
    useApp.getState().toast({ kind: "error", message: errorMessage(error) });
    return false;
  }
}

export async function startFromUrl(url: string): Promise<boolean> {
  const settings = useApp.getState().settings;
  const preset = settings?.preset ?? "best";
  return startDownload({ url, preset, quality: preset === "audio" ? "mp3" : undefined });
}

export async function refreshBinaries(): Promise<void> {
  try {
    const binaries = await api.binariesStatus();
    useApp.getState().setBinaries(binaries);
  } catch {
    /* silencioso */
  }
}

export async function initApp(): Promise<void> {
  if (!isTauri()) return;
  const store = useApp.getState();
  try {
    const [list, settings, binaries, info] = await Promise.all([
      api.listDownloads(),
      api.getSettings(),
      api.binariesStatus(),
      api.appInfo(),
    ]);
    store.mergeList(list);
    store.setSettings(settings);
    store.setBinaries(binaries);
    store.setInfo(info);
  } catch (error) {
    store.toast({ kind: "error", message: errorMessage(error) });
  }
}

export function presetLabel(preset: string | null | undefined): string {
  switch (preset) {
    case "max":
      return "Máxima qualidade";
    case "audio":
      return "Só áudio";
    default:
      return "Melhor qualidade";
  }
}
