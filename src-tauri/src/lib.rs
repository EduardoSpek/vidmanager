mod api;
mod binaries;
mod commands;
mod db;
mod models;
mod settings;
mod state;
mod threads;
mod util;
mod ytdlp;

use settings::Settings;
use state::AppState;
use std::time::Duration;
use tauri::{Emitter, Manager};

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = binaries::app_data(&handle);
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("vidmanager.db");
            let conn = db::open(&db_path)?;

            let mut settings: Settings = db::get_setting(&conn, "settings")
                .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
                .unwrap_or_default();
            if settings.api_token.trim().is_empty() {
                settings.api_token = uuid::Uuid::new_v4().simple().to_string();
            }
            settings = settings.sanitized();
            let _ = std::fs::create_dir_all(&settings.output_dir);
            if let Ok(json) = serde_json::to_string(&settings) {
                let _ = db::set_setting(&conn, "settings", &json);
            }

            let bin_dir = binaries::bin_dir(&handle);
            let _ = std::fs::create_dir_all(&bin_dir);

            let state = AppState::new(conn, settings.clone(), bin_dir, data_dir.clone());
            app.manage(state);

            // Downloads interrompidos por fechamento do app ficam pausados.
            let st = app.state::<AppState>();
            let interrupted: Vec<String> = st
                .all()
                .into_iter()
                .filter(|d| d.status == "downloading")
                .map(|d| d.id)
                .collect();
            for id in interrupted {
                let _ = st.patch(&id, |d| {
                    d.status = "paused".to_string();
                    d.speed = 0.0;
                    d.eta = None;
                });
            }

            // Fila: inicia os itens enfileirados respeitando o limite de simultâneos.
            {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_millis(700)).await;
                        let state = h.state::<AppState>();
                        let max = state
                            .settings
                            .lock()
                            .map(|s| s.max_concurrent as usize)
                            .unwrap_or(3)
                            .max(1);
                        for _ in 0..max {
                            let running = state
                                .downloads
                                .lock()
                                .map(|m| {
                                    m.values()
                                        .filter(|d| d.status == "downloading")
                                        .count()
                                })
                                .unwrap_or(0);
                            if running >= max {
                                break;
                            }
                            let next = state.downloads.lock().ok().and_then(|m| {
                                m.values()
                                    .filter(|d| d.status == "queued")
                                    .min_by_key(|d| d.created_at)
                                    .map(|d| d.id.clone())
                            });
                            match next {
                                Some(id) => {
                                    let updated = state.patch(&id, |d| {
                                        d.status = "downloading".to_string();
                                        d.error = None;
                                        d.speed = 0.0;
                                    });
                                    if let Some(d) = updated {
                                        let _ = h.emit("download:update", &d);
                                    }
                                    ytdlp::spawn_download(h.clone(), id);
                                }
                                None => break,
                            }
                        }
                    }
                });
            }

            // API local para a extensão do navegador.
            if settings.api_enabled {
                api::spawn(handle.clone(), settings.api_port);
            }

            // Primeiro uso: baixa o yt-dlp automaticamente.
            if binaries::resolve_ytdlp(&handle).is_none() {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = h.emit(
                        "toast",
                        serde_json::json!({
                            "kind": "info",
                            "message": "Baixando yt-dlp (primeira execução)…"
                        }),
                    );
                    match binaries::install_ytdlp(&h).await {
                        Ok(v) => {
                            let _ = h.emit(
                                "toast",
                                serde_json::json!({
                                    "kind": "success",
                                    "message": format!("yt-dlp {v} instalado com sucesso")
                                }),
                            );
                            let _ = h.emit("binaries:changed", ());
                        }
                        Err(e) => {
                            let _ = h.emit(
                                "toast",
                                serde_json::json!({
                                    "kind": "error",
                                    "message": format!("Falha ao instalar o yt-dlp: {e}")
                                }),
                            );
                        }
                    }
                });
            }

            // Runtime JavaScript (Deno) para o yt-dlp decifrar assinaturas do YouTube.
            if binaries::resolve_js_runtime(&handle).is_none() {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = h.emit(
                        "toast",
                        serde_json::json!({
                            "kind": "info",
                            "message": "Baixando Deno (runtime para o YouTube)…"
                        }),
                    );
                    match binaries::install_deno(&h).await {
                        Ok(v) => {
                            let _ = h.emit(
                                "toast",
                                serde_json::json!({
                                    "kind": "success",
                                    "message": format!("{v} instalado. Mais formatos disponíveis.")
                                }),
                            );
                            let _ = h.emit("binaries:changed", ());
                        }
                        Err(e) => {
                            let _ = h.emit(
                                "toast",
                                serde_json::json!({
                                    "kind": "info",
                                    "message": format!("Não foi possível instalar o Deno: {e}")
                                }),
                            );
                        }
                    }
                });
            }

            // Monitor de clipboard.
            {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let mut last = String::new();
                    loop {
                        tokio::time::sleep(Duration::from_millis(1400)).await;
                        let enabled = h
                            .state::<AppState>()
                            .settings
                            .lock()
                            .map(|s| s.clipboard_monitor)
                            .unwrap_or(false);
                        if !enabled {
                            continue;
                        }
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            if let Ok(text) = cb.get_text() {
                                let t = text.trim().to_string();
                                if t != last {
                                    if util::is_probable_url(&t) {
                                        let _ = h.emit(
                                            "clipboard:url",
                                            serde_json::json!({ "url": t }),
                                        );
                                    }
                                    last = t;
                                }
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_downloads,
            commands::probe_url,
            commands::start_download,
            commands::pause_download,
            commands::resume_download,
            commands::cancel_download,
            commands::retry_download,
            commands::remove_download,
            commands::clear_completed,
            commands::get_settings,
            commands::save_settings,
            commands::binaries_status,
            commands::install_ytdlp,
            commands::install_ffmpeg,
            commands::install_deno,
            commands::open_path,
            commands::reveal_path,
            commands::pick_folder,
            commands::pairing_info,
            commands::rotate_pairing,
            commands::app_info
        ])
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o VidManager");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(state) = app_handle.try_state::<AppState>() {
                let pids: Vec<u32> = state
                    .pids
                    .lock()
                    .map(|m| m.values().copied().collect())
                    .unwrap_or_default();
                for pid in pids {
                    ytdlp::kill_tree(pid);
                }
            }
        }
    });
}
