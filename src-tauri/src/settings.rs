use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output_dir: String,
    pub preset: String,
    pub audio_format: String,
    pub max_concurrent: u32,
    pub speed_limit_kb: u32,
    pub filename_template: String,
    pub ask_format: bool,
    pub embed_metadata: bool,
    pub embed_thumbnail: bool,
    pub embed_subs: bool,
    pub sub_langs: String,
    pub cookies_browser: String,
    pub clipboard_monitor: bool,
    pub notify_on_complete: bool,
    pub api_enabled: bool,
    pub api_port: u16,
    pub api_token: String,
}

impl Default for Settings {
    fn default() -> Self {
        let base = dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            output_dir: base.join("VidManager").to_string_lossy().to_string(),
            preset: "best".to_string(),
            audio_format: "mp3".to_string(),
            max_concurrent: 3,
            speed_limit_kb: 0,
            filename_template: "%(uploader,channel,creator|Outros)s/%(title).150B [%(id)s].%(ext)s".to_string(),
            ask_format: false,
            embed_metadata: true,
            embed_thumbnail: true,
            embed_subs: false,
            sub_langs: "pt.*,en.*".to_string(),
            cookies_browser: String::new(),
            clipboard_monitor: true,
            notify_on_complete: true,
            api_enabled: true,
            api_port: 41300,
            api_token: String::new(),
        }
    }
}

impl Settings {
    pub fn sanitized(mut self) -> Self {
        self.max_concurrent = self.max_concurrent.clamp(1, 8);
        if self.api_port < 1024 {
            self.api_port = 41300;
        }
        if !matches!(
            self.preset.as_str(),
            "best" | "max" | "audio"
        ) {
            self.preset = "best".to_string();
        }
        if self.filename_template.trim().is_empty() {
            self.filename_template = "%(uploader,channel,creator|Outros)s/%(title).150B [%(id)s].%(ext)s".to_string();
        }
        if self.output_dir.trim().is_empty() {
            self.output_dir = Settings::default().output_dir;
        }
        self
    }
}
