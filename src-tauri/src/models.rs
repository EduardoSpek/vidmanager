use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub id: String,
    pub url: String,
    pub title: String,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub status: String,
    pub progress: f64,
    pub downloaded_bytes: i64,
    pub total_bytes: i64,
    pub speed: f64,
    pub eta: Option<i64>,
    pub filepath: Option<String>,
    pub error: Option<String>,
    pub preset: String,
    pub quality: Option<String>,
    pub playlist_items: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub url: String,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub playlist_items: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionOption {
    pub height: u32,
    pub fps: Option<f64>,
    pub ext: Option<String>,
    pub vcodec: Option<String>,
    pub filesize: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub url: String,
    pub title: String,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub is_playlist: bool,
    pub playlist_count: Option<u64>,
    pub resolutions: Vec<ResolutionOption>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryInfo {
    pub path: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinariesStatus {
    pub ytdlp: Option<BinaryInfo>,
    pub ffmpeg: Option<BinaryInfo>,
    pub js_runtime: Option<BinaryInfo>,
    pub bin_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    pub code: String,
    pub expires_at: i64,
    pub port: u16,
    pub token: String,
    pub api_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub output_dir: String,
    pub download_dir: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CookieInput {
    pub domain: String,
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub secure: Option<bool>,
    #[serde(default, rename = "expirationDate")]
    pub expiration_date: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDownload {
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub referer: Option<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub cookies: Option<Vec<CookieInput>>,
}
