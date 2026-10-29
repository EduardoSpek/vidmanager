export type DownloadStatus =
  | "queued"
  | "downloading"
  | "paused"
  | "completed"
  | "error"
  | "canceled";

export interface Download {
  id: string;
  url: string;
  title: string;
  uploader: string | null;
  thumbnail: string | null;
  duration: number | null;
  status: DownloadStatus;
  progress: number;
  downloadedBytes: number;
  totalBytes: number;
  speed: number;
  eta: number | null;
  filepath: string | null;
  error: string | null;
  preset: string;
  quality: string | null;
  playlistItems: string | null;
  createdAt: number;
  finishedAt: number | null;
  source: string;
}

export interface Settings {
  outputDir: string;
  preset: string;
  audioFormat: string;
  maxConcurrent: number;
  speedLimitKb: number;
  filenameTemplate: string;
  askFormat: boolean;
  embedMetadata: boolean;
  embedThumbnail: boolean;
  embedSubs: boolean;
  subLangs: string;
  cookiesBrowser: string;
  clipboardMonitor: boolean;
  notifyOnComplete: boolean;
  apiEnabled: boolean;
  apiPort: number;
  apiToken: string;
}

export interface ResolutionOption {
  height: number;
  fps: number | null;
  ext: string | null;
  vcodec: string | null;
  filesize: number | null;
}

export interface ProbeResult {
  url: string;
  title: string;
  uploader: string | null;
  thumbnail: string | null;
  duration: number | null;
  isPlaylist: boolean;
  playlistCount: number | null;
  resolutions: ResolutionOption[];
}

export interface BinaryInfo {
  path: string;
  version: string;
}

export interface BinariesStatus {
  ytdlp: BinaryInfo | null;
  ffmpeg: BinaryInfo | null;
  jsRuntime: BinaryInfo | null;
  binDir: string;
}

export interface PairingInfo {
  code: string;
  expiresAt: number;
  port: number;
  token: string;
  apiEnabled: boolean;
}

export interface AppInfo {
  version: string;
  outputDir: string;
  downloadDir: string;
  dataDir: string;
}

export interface StartRequest {
  url: string;
  preset?: string;
  quality?: string;
  playlistItems?: string;
  source?: string;
}
