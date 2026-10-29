import { useMemo } from "react";
import { create } from "zustand";
import type { AppInfo, BinariesStatus, Download, Settings } from "../lib/types";

export type Screen = "home" | "library" | "settings";
export type ToastKind = "info" | "success" | "error";

export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  actionLabel?: string;
  onAction?: () => void;
}

interface AppStore {
  downloads: Record<string, Download>;
  settings: Settings | null;
  binaries: BinariesStatus | null;
  info: AppInfo | null;
  screen: Screen;
  toasts: Toast[];
  search: string;
  formatFor: string | null;
  setScreen: (screen: Screen) => void;
  setSettings: (settings: Settings) => void;
  setBinaries: (binaries: BinariesStatus | null) => void;
  setInfo: (info: AppInfo) => void;
  setSearch: (search: string) => void;
  openFormat: (url: string) => void;
  closeFormat: () => void;
  upsert: (download: Download) => void;
  removeDownload: (id: string) => void;
  mergeList: (list: Download[]) => void;
  toast: (toast: Omit<Toast, "id">) => void;
  dismissToast: (id: string) => void;
}

let toastSeq = 0;

export const useApp = create<AppStore>((set) => ({
  downloads: {},
  settings: null,
  binaries: null,
  info: null,
  screen: "home",
  toasts: [],
  search: "",
  formatFor: null,
  setScreen: (screen) => set({ screen }),
  setSettings: (settings) => set({ settings }),
  setBinaries: (binaries) => set({ binaries }),
  setInfo: (info) => set({ info }),
  setSearch: (search) => set({ search }),
  openFormat: (url) => set({ formatFor: url }),
  closeFormat: () => set({ formatFor: null }),
  upsert: (download) =>
    set((state) => ({
      downloads: { ...state.downloads, [download.id]: download },
    })),
  removeDownload: (id) =>
    set((state) => {
      const next = { ...state.downloads };
      delete next[id];
      return { downloads: next };
    }),
  mergeList: (list) =>
    set(() => {
      const map: Record<string, Download> = {};
      for (const item of list) map[item.id] = item;
      return { downloads: map };
    }),
  toast: (toast) =>
    set((state) => {
      toastSeq += 1;
      const id = `t${Date.now()}_${toastSeq}`;
      return { toasts: [...state.toasts.slice(-3), { ...toast, id }] };
    }),
  dismissToast: (id) =>
    set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}));

export function useDownloadsList(): Download[] {
  const downloads = useApp((state) => state.downloads);
  return useMemo(
    () => Object.values(downloads).sort((a, b) => b.createdAt - a.createdAt),
    [downloads]
  );
}

export function useActiveDownloads(): Download[] {
  const list = useDownloadsList();
  return useMemo(
    () => list.filter((d) => ["queued", "downloading", "paused"].includes(d.status)),
    [list]
  );
}
