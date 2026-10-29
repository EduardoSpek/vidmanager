use crate::binaries;
use crate::db;
use crate::models::{
    AppInfo, BinariesStatus, BinaryInfo, Download, PairingInfo, ProbeResult, StartRequest,
};
use crate::settings::Settings;
use crate::state::{new_pair_code, now_ms, AppState};
use crate::ytdlp;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn list_downloads(state: State<'_, AppState>) -> Vec<Download> {
    state.all()
}

#[tauri::command]
pub async fn probe_url(app: AppHandle, url: String) -> Result<ProbeResult, String> {
    let u = url.trim();
    if !crate::util::is_probable_url(u) {
        return Err("Cole uma URL http(s) válida.".to_string());
    }
    if crate::threads::is_threads_url(u) {
        return crate::threads::probe(u).await;
    }
    ytdlp::probe(&app, u).await
}

#[tauri::command]
pub fn start_download(
    app: AppHandle,
    state: State<'_, AppState>,
    req: StartRequest,
) -> Result<Download, String> {
    let url = req.url.trim().to_string();
    if !crate::util::is_probable_url(&url) {
        return Err("Cole uma URL http(s) válida.".to_string());
    }
    if let Some(existing) = state.exist_by_url(&url) {
        return Ok(existing);
    }
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let preset = req
        .preset
        .filter(|p| matches!(p.as_str(), "best" | "max" | "audio"))
        .unwrap_or(settings.preset.clone());
    let quality = if preset == "audio" {
        req.quality.or_else(|| Some("mp3".to_string()))
    } else {
        req.quality
    };

    let d = Download {
        id: uuid::Uuid::new_v4().to_string(),
        url,
        title: String::new(),
        uploader: None,
        thumbnail: None,
        duration: None,
        status: "queued".to_string(),
        progress: 0.0,
        downloaded_bytes: 0,
        total_bytes: 0,
        speed: 0.0,
        eta: None,
        filepath: None,
        error: None,
        preset,
        quality,
        playlist_items: req.playlist_items.filter(|p| !p.trim().is_empty()),
        created_at: now_ms(),
        finished_at: None,
        source: req.source.unwrap_or_else(|| "ui".to_string()),
    };

    state.put(d.clone());
    let _ = app.emit("download:update", &d);
    Ok(d)
}

#[tauri::command]
pub fn pause_download(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let current = state.get(&id).ok_or("Download não encontrado.")?;
    if !matches!(current.status.as_str(), "downloading" | "queued") {
        return Err("Este download não está em andamento.".to_string());
    }
    let pid = state.pids.lock().ok().and_then(|m| m.get(&id).copied());
    let updated = state.patch(&id, |d| {
        d.status = "paused".to_string();
        d.speed = 0.0;
        d.eta = None;
    });
    if let Some(pid) = pid {
        ytdlp::kill_tree(pid);
    }
    if let Some(d) = updated {
        let _ = app.emit("download:update", &d);
    }
    Ok(())
}

#[tauri::command]
pub fn resume_download(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let current = state.get(&id).ok_or("Download não encontrado.")?;
    if current.status == "completed" {
        return Err("Este download já foi concluído.".to_string());
    }
    if current.status == "downloading" {
        return Ok(());
    }
    let updated = state.patch(&id, |d| {
        d.status = "queued".to_string();
        d.error = None;
        d.speed = 0.0;
    });
    if let Some(d) = updated {
        let _ = app.emit("download:update", &d);
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_download(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let current = state.get(&id).ok_or("Download não encontrado.")?;
    let pid = state.pids.lock().ok().and_then(|m| m.get(&id).copied());
    let alvo = current.filepath.clone();

    // Marca como cancelado ANTES de matar o processo: se o worker terminar primeiro,
    // ele encontraria o status "downloading" e gravaria uma falha falsa.
    let updated = state.patch(&id, |d| {
        d.status = "canceled".to_string();
        d.speed = 0.0;
        d.eta = None;
        d.progress = 0.0;
        d.downloaded_bytes = 0;
        d.total_bytes = 0;
        d.error = None;
        d.filepath = None;
    });
    if let Some(d) = updated {
        let _ = app.emit("download:update", &d);
    }

    if let Some(pid) = pid {
        ytdlp::kill_tree(pid);
    }

    // Remove o parcial em segundo plano, com retentativas: o yt-dlp pode manter o
    // arquivo aberto por alguns instantes depois do kill.
    if let Some(path) = alvo {
        std::thread::spawn(move || {
            for _ in 0..15 {
                ytdlp::cleanup_partial(&path);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
    }
    Ok(())
}

#[tauri::command]
pub fn retry_download(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let current = state.get(&id).ok_or("Download não encontrado.")?;
    if current.status == "downloading" {
        return Ok(());
    }
    let pid = state.pids.lock().ok().and_then(|m| m.get(&id).copied());
    if let Some(pid) = pid {
        ytdlp::kill_tree(pid);
    }
    // Threads: descarta a URL direta guardada — as do CDN expiram, então
    // resolvemos o post de novo antes de tentar.
    if crate::threads::is_threads_url(&current.url) {
        if let Ok(mut extras) = state.extras.lock() {
            extras.remove(&id);
        }
    }
    let updated = state.patch(&id, |d| {
        d.status = "queued".to_string();
        d.error = None;
        d.speed = 0.0;
        d.progress = 0.0;
        d.downloaded_bytes = 0;
        d.total_bytes = 0;
        d.finished_at = None;
    });
    if let Some(d) = updated {
        let _ = app.emit("download:update", &d);
    }
    Ok(())
}

#[tauri::command]
pub fn remove_download(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    delete_file: bool,
) -> Result<(), String> {
    let current = state.get(&id);
    let pid = state.pids.lock().ok().and_then(|m| m.get(&id).copied());
    if let Some(pid) = pid {
        ytdlp::kill_tree(pid);
    }
    if delete_file {
        if let Some(path) = current.as_ref().and_then(|d| d.filepath.clone()) {
            ytdlp::cleanup_partial(&path);
        }
    }
    state.remove(&id);
    if let Ok(mut extras) = state.extras.lock() {
        extras.remove(&id);
    }
    let _ = app.emit("download:removed", id);
    Ok(())
}

#[tauri::command]
pub fn clear_completed(app: AppHandle, state: State<'_, AppState>) -> Result<u32, String> {
    let ids: Vec<String> = state
        .all()
        .into_iter()
        .filter(|d| matches!(d.status.as_str(), "completed" | "canceled"))
        .map(|d| d.id)
        .collect();
    let count = ids.len() as u32;
    for id in ids {
        state.remove(&id);
        let _ = app.emit("download:removed", id);
    }
    Ok(count)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().map(|s| s.clone()).unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    let mut s = settings.sanitized();
    if s.api_token.trim().is_empty() {
        s.api_token = uuid::Uuid::new_v4().simple().to_string();
    }
    std::fs::create_dir_all(&s.output_dir).map_err(|e| format!("Pasta de destino inválida: {e}"))?;
    {
        let conn = state.db.lock().map_err(|_| "Banco indisponível.".to_string())?;
        let json = serde_json::to_string(&s).map_err(|e| e.to_string())?;
        db::set_setting(&conn, "settings", &json).map_err(|e| e.to_string())?;
    }
    *state.settings.lock().map_err(|_| "Configurações indisponíveis.".to_string())? = s.clone();
    let _ = app.emit("settings:changed", &s);
    Ok(s)
}

#[tauri::command]
pub fn binaries_status(app: AppHandle, state: State<'_, AppState>) -> BinariesStatus {
    let ytdlp = binaries::resolve_ytdlp(&app).map(|p| {
        let version = binaries::ytdlp_version(&p).unwrap_or_default();
        if let Ok(mut cache) = state.ytdlp_version.lock() {
            *cache = Some(version.clone());
        }
        BinaryInfo {
            path: p.to_string_lossy().to_string(),
            version,
        }
    });
    let ffmpeg = binaries::resolve_ffmpeg(&app).map(|p| {
        let version = binaries::ffmpeg_version(&p).unwrap_or_default();
        if let Ok(mut cache) = state.ffmpeg_version.lock() {
            *cache = Some(version.clone());
        }
        BinaryInfo {
            path: p.to_string_lossy().to_string(),
            version,
        }
    });
    let js_runtime = binaries::resolve_js_runtime(&app).map(|(_name, path)| {
        let version = binaries::version_of(&path, "--version")
            .unwrap_or_else(|| "runtime JavaScript".to_string());
        BinaryInfo {
            path: path.to_string_lossy().to_string(),
            version,
        }
    });
    BinariesStatus {
        ytdlp,
        ffmpeg,
        js_runtime,
        bin_dir: state.bin_dir.to_string_lossy().to_string(),
    }
}

#[tauri::command]
pub async fn install_ytdlp(app: AppHandle) -> Result<String, String> {
    let version = binaries::install_ytdlp(&app).await?;
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut cache) = state.ytdlp_version.lock() {
            *cache = Some(version.clone());
        }
    }
    let _ = app.emit("binaries:changed", ());
    Ok(version)
}

#[tauri::command]
pub async fn install_ffmpeg(app: AppHandle) -> Result<String, String> {
    let version = binaries::install_ffmpeg(&app).await?;
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut cache) = state.ffmpeg_version.lock() {
            *cache = Some(version.clone());
        }
    }
    let _ = app.emit("binaries:changed", ());
    Ok(version)
}

#[tauri::command]
pub async fn install_deno(app: AppHandle) -> Result<String, String> {
    let version = binaries::install_deno(&app).await?;
    let _ = app.emit("binaries:changed", ());
    Ok(version)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("Arquivo ou pasta não encontrado.".to_string());
    }
    abrir_explorer(&path, false).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("Arquivo não encontrado.".to_string());
    }
    abrir_explorer(&path, true).map_err(|e| e.to_string())?;
    Ok(())
}

/// Abre o Explorer na pasta (ou com o arquivo selecionado).
///
/// No Windows a linha de comando é montada crua: `explorer /select,"C:\caminho com espaço\a.mp4"`.
/// Passar isso como argumento normal faz o Rust citar a string inteira (`"/select,C:\..."`), e o
/// Explorer então ignora o switch e abre a pasta padrão (Documentos).
fn abrir_explorer(path: &str, selecionar: bool) -> std::io::Result<()> {
    let limpo = path.replace('"', "");
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if selecionar {
            cmd.raw_arg(format!("/select,\"{limpo}\""));
        } else {
            cmd.raw_arg(format!("\"{limpo}\""));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = selecionar;
        cmd.arg(&limpo);
    }
    binaries::hide(&mut cmd);
    cmd.spawn().map(|_| ())
}

#[tauri::command]
pub async fn pick_folder() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Escolher pasta de downloads")
            .pick_folder()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
pub fn pairing_info(state: State<'_, AppState>) -> Result<PairingInfo, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let mut pair = state.pair.lock().map_err(|_| "Estado indisponível.".to_string())?;
    if pair.expires_at < now_ms() {
        pair.code = new_pair_code();
        pair.expires_at = now_ms() + 24 * 60 * 60 * 1000;
    }
    Ok(PairingInfo {
        code: pair.code.clone(),
        expires_at: pair.expires_at,
        port: settings.api_port,
        token: settings.api_token.clone(),
        api_enabled: settings.api_enabled,
    })
}

#[tauri::command]
pub fn rotate_pairing(state: State<'_, AppState>) -> Result<PairingInfo, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let mut pair = state.pair.lock().map_err(|_| "Estado indisponível.".to_string())?;
    pair.code = new_pair_code();
    pair.expires_at = now_ms() + 24 * 60 * 60 * 1000;
    Ok(PairingInfo {
        code: pair.code.clone(),
        expires_at: pair.expires_at,
        port: settings.api_port,
        token: settings.api_token.clone(),
        api_enabled: settings.api_enabled,
    })
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    AppInfo {
        version: app.package_info().version.to_string(),
        output_dir: settings.output_dir,
        download_dir: dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        data_dir: state.app_data_dir.to_string_lossy().to_string(),
    }
}
