use crate::db;
use crate::models::Download;
use crate::settings::Settings;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct JobExtras {
    pub cookies: Option<PathBuf>,
    pub referer: Option<String>,
    /// URL direta do arquivo (usada no lugar da URL da página, ex.: Threads).
    pub source_url: Option<String>,
    /// Template de saída específico do job (sobrepõe o template global).
    pub output_template: Option<String>,
    pub user_agent: Option<String>,
}

impl Default for JobExtras {
    fn default() -> Self {
        Self {
            cookies: None,
            referer: None,
            source_url: None,
            output_template: None,
            user_agent: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PairState {
    pub code: String,
    pub expires_at: i64,
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub settings: Mutex<Settings>,
    pub downloads: Mutex<HashMap<String, Download>>,
    pub pids: Mutex<HashMap<String, u32>>,
    pub extras: Mutex<HashMap<String, JobExtras>>,
    pub pair: Mutex<PairState>,
    pub bin_dir: PathBuf,
    pub ytdlp_version: Mutex<Option<String>>,
    pub ffmpeg_version: Mutex<Option<String>>,
    pub app_data_dir: PathBuf,
}

impl AppState {
    pub fn new(conn: Connection, settings: Settings, bin_dir: PathBuf, app_data_dir: PathBuf) -> Self {
        let downloads = db::load_all(&conn)
            .unwrap_or_default()
            .into_iter()
            .map(|d| (d.id.clone(), d))
            .collect::<HashMap<String, Download>>();
        Self {
            db: Mutex::new(conn),
            settings: Mutex::new(settings),
            downloads: Mutex::new(downloads),
            pids: Mutex::new(HashMap::new()),
            extras: Mutex::new(HashMap::new()),
            pair: Mutex::new(PairState {
                code: new_pair_code(),
                expires_at: 0,
            }),
            bin_dir,
            ytdlp_version: Mutex::new(None),
            ffmpeg_version: Mutex::new(None),
            app_data_dir,
        }
    }

    pub fn get(&self, id: &str) -> Option<Download> {
        self.downloads.lock().ok()?.get(id).cloned()
    }

    pub fn all(&self) -> Vec<Download> {
        let mut list: Vec<Download> = self
            .downloads
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    pub fn put(&self, d: Download) {
        self.commit(&d);
        if let Ok(mut map) = self.downloads.lock() {
            map.insert(d.id.clone(), d);
        }
    }

    pub fn commit(&self, d: &Download) {
        if let Ok(conn) = self.db.lock() {
            let _ = db::upsert(&conn, d);
        }
    }

    pub fn patch<F>(&self, id: &str, f: F) -> Option<Download>
    where
        F: FnOnce(&mut Download),
    {
        let updated = self.patch_mem(id, f)?;
        self.commit(&updated);
        Some(updated)
    }

    pub fn patch_mem<F>(&self, id: &str, f: F) -> Option<Download>
    where
        F: FnOnce(&mut Download),
    {
        let mut map = self.downloads.lock().ok()?;
        let d = map.get_mut(id)?;
        f(d);
        Some(d.clone())
    }

    pub fn remove(&self, id: &str) {
        if let Ok(mut map) = self.downloads.lock() {
            map.remove(id);
        }
        if let Ok(conn) = self.db.lock() {
            let _ = db::delete(&conn, id);
        }
    }

    pub fn exist_by_url(&self, url: &str) -> Option<Download> {
        let map = self.downloads.lock().ok()?;
        map.values()
            .find(|d| d.url == url && matches!(d.status.as_str(), "queued" | "downloading"))
            .cloned()
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn new_pair_code() -> String {
    let raw = uuid::Uuid::new_v4().simple().to_string();
    let digits: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(6)
        .collect();
    let padded = format!("{:0>6}", digits);
    padded.chars().take(6).collect()
}
