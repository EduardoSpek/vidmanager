import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  BinariesStatus,
  Download,
  PairingInfo,
  ProbeResult,
  Settings,
  StartRequest,
} from "./types";

export const api = {
  listDownloads: () => invoke<Download[]>("list_downloads"),
  probe: (url: string) => invoke<ProbeResult>("probe_url", { url }),
  start: (req: StartRequest) => invoke<Download>("start_download", { req }),
  pause: (id: string) => invoke<void>("pause_download", { id }),
  resume: (id: string) => invoke<void>("resume_download", { id }),
  cancel: (id: string) => invoke<void>("cancel_download", { id }),
  retry: (id: string) => invoke<void>("retry_download", { id }),
  remove: (id: string, deleteFile: boolean) =>
    invoke<void>("remove_download", { id, deleteFile }),
  clearCompleted: () => invoke<number>("clear_completed"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  binariesStatus: () => invoke<BinariesStatus>("binaries_status"),
  installYtdlp: () => invoke<string>("install_ytdlp"),
  installFfmpeg: () => invoke<string>("install_ffmpeg"),
  installDeno: () => invoke<string>("install_deno"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  revealPath: (path: string) => invoke<void>("reveal_path", { path }),
  pickFolder: () => invoke<string | null>("pick_folder"),
  pairingInfo: () => invoke<PairingInfo>("pairing_info"),
  rotatePairing: () => invoke<PairingInfo>("rotate_pairing"),
  appInfo: () => invoke<AppInfo>("app_info"),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
