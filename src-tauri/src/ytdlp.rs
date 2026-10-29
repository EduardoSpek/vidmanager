use crate::binaries;
use crate::models::{Download, ProbeResult, ResolutionOption};
use crate::settings::Settings;
use crate::state::{now_ms, AppState};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub fn kill_tree(pid: u32) {
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    binaries::hide(&mut cmd);
    let _ = cmd.status();
}

pub async fn probe(app: &AppHandle, url: &str) -> Result<ProbeResult, String> {
    let ytdlp = binaries::resolve_ytdlp(app)
        .ok_or_else(|| "yt-dlp não encontrado. Reinstale em Configurações → Binários.".to_string())?;
    let mut cmd = Command::new(&ytdlp);
    let mut args: Vec<String> = vec![
        "--dump-single-json".into(),
        "--no-warnings".into(),
        "--skip-download".into(),
        "--no-playlist".into(),
        "--ignore-config".into(),
    ];
    if let Some((name, path)) = binaries::resolve_js_runtime(app) {
        args.push("--js-runtimes".into());
        args.push(format!("{name}:{}", path.to_string_lossy()));
    }
    args.push(url.to_string());
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    binaries::hide_tokio(&mut cmd);

    let out = tokio::time::timeout(Duration::from_secs(75), cmd.output())
        .await
        .map_err(|_| "Tempo esgotado ao analisar o vídeo (75s).".to_string())?
        .map_err(|e| format!("Falha ao executar o yt-dlp: {e}"))?;

    if !out.status.success() {
        return Err(clean_error(&String::from_utf8_lossy(&out.stderr)));
    }
    let value: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("Resposta inválida do yt-dlp: {e}"))?;
    Ok(parse_probe(url, &value))
}

fn parse_probe(url: &str, v: &Value) -> ProbeResult {
    let title = v["title"]
        .as_str()
        .or_else(|| v["fulltitle"].as_str())
        .unwrap_or("Vídeo")
        .to_string();
    let uploader = v["uploader"]
        .as_str()
        .or_else(|| v["channel"].as_str())
        .or_else(|| v["creator"].as_str())
        .map(|s| s.to_string());
    let thumbnail = v["thumbnail"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| {
            v["thumbnails"]
                .as_array()
                .and_then(|arr| arr.last())
                .and_then(|t| t["url"].as_str())
                .map(|s| s.to_string())
        });
    let duration = v["duration"].as_f64();

    let mut map: BTreeMap<u32, ResolutionOption> = BTreeMap::new();
    if let Some(formats) = v["formats"].as_array() {
        for f in formats {
            let height = f["height"].as_u64().unwrap_or(0) as u32;
            let vcodec = f["vcodec"].as_str().unwrap_or("none");
            if height == 0 || vcodec == "none" {
                continue;
            }
            let entry = map.entry(height).or_insert(ResolutionOption {
                height,
                fps: None,
                ext: None,
                vcodec: None,
                filesize: None,
            });
            let size = f["filesize"].as_i64().or_else(|| f["filesize_approx"].as_i64());
            if entry.filesize.unwrap_or(0) < size.unwrap_or(0) {
                entry.filesize = size;
            }
            if entry.fps.is_none() {
                entry.fps = f["fps"].as_f64();
            }
            if entry.ext.is_none() {
                entry.ext = f["ext"].as_str().map(|s| s.to_string());
            }
            if entry.vcodec.is_none() {
                entry.vcodec = f["vcodec"].as_str().map(|s| s.to_string());
            }
        }
    }

    let mut resolutions: Vec<ResolutionOption> = map.into_values().collect();
    resolutions.reverse();

    let is_playlist = v["_type"].as_str() == Some("playlist") || url.contains("list=");
    let playlist_count = v["playlist_count"]
        .as_u64()
        .or_else(|| v["entries"].as_array().map(|a| a.len() as u64));

    ProbeResult {
        url: v["webpage_url"].as_str().unwrap_or(url).to_string(),
        title,
        uploader,
        thumbnail,
        duration,
        is_playlist,
        playlist_count,
        resolutions,
    }
}

pub fn spawn_download(app: AppHandle, id: String) {
    tauri::async_runtime::spawn(async move {
        run(app, id).await;
    });
}

async fn run(app: AppHandle, id: String) {
    let state = app.state::<AppState>();
    let mut dl = match state.get(&id) {
        Some(d) => d,
        None => return,
    };

    // Threads: o yt-dlp não suporta; resolvemos o post e baixamos a URL direta.
    if crate::threads::is_threads_url(&dl.url) {
        if let Err(e) = crate::threads::prepare(&app, &state, &id).await {
            fail(&app, &state, &id, &e);
            return;
        }
        // Pode ter sido cancelado durante a análise do post.
        match state.get(&id) {
            Some(d) if d.status == "downloading" => dl = d,
            _ => return,
        }
    }

    if dl.title.starts_with("http") || dl.title.trim().is_empty() {
        if let Ok(Ok(info)) = tokio::time::timeout(Duration::from_secs(50), probe(&app, &dl.url)).await {
            dl.title = info.title;
            dl.uploader = info.uploader;
            dl.thumbnail = info.thumbnail;
            dl.duration = info.duration;
            state.patch(&id, |d| {
                d.title = dl.title.clone();
                d.uploader = dl.uploader.clone();
                d.thumbnail = dl.thumbnail.clone();
                d.duration = dl.duration;
            });
            if let Some(d) = state.get(&id) {
                let _ = app.emit("download:update", &d);
            }
        }
    }

    let settings = match state.settings.lock() {
        Ok(s) => s.clone(),
        Err(_) => Settings::default(),
    };
    let _ = std::fs::create_dir_all(&settings.output_dir);
    let ffmpeg = binaries::resolve_ffmpeg(&app);
    let js = binaries::resolve_js_runtime(&app);
    let extras = state
        .extras
        .lock()
        .ok()
        .and_then(|m| m.get(&id).cloned())
        .unwrap_or_default();

    let current = match state.get(&id) {
        Some(d) => d,
        None => return,
    };
    let args = match build_args(
        &current,
        &settings,
        ffmpeg.as_deref(),
        js.as_ref().map(|(name, path)| (name.as_str(), path.as_path())),
        &extras,
    ) {
        Ok(a) => a,
        Err(e) => {
            fail(&app, &state, &id, &e);
            return;
        }
    };
    let ytdlp = match binaries::resolve_ytdlp(&app) {
        Some(p) => p,
        None => {
            fail(&app, &state, &id, "yt-dlp não encontrado. Reinstale em Configurações → Binários.");
            return;
        }
    };

    let mut cmd = Command::new(&ytdlp);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    binaries::hide_tokio(&mut cmd);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            fail(&app, &state, &id, &format!("Não foi possível iniciar o yt-dlp: {e}"));
            return;
        }
    };

    if let Some(pid) = child.id() {
        if let Ok(mut pids) = state.pids.lock() {
            pids.insert(id.clone(), pid);
        }
    }

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return,
    };
    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => return,
    };

    let errbuf: Arc<StdMutex<VecDeque<String>>> = Arc::new(StdMutex::new(VecDeque::new()));
    let errbuf_task = errbuf.clone();
    let stderr_task = tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(mut buf) = errbuf_task.lock() {
                buf.push_back(line);
                if buf.len() > 60 {
                    buf.pop_front();
                }
            }
        }
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut filepath: Option<String> = None;
    let mut last_emit = Instant::now();
    let mut last_persist = Instant::now();
    let mut canceled = false;

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Some(rest) = line.strip_prefix("@@P@@") {
                    handle_progress(&app, &state, &id, rest, &mut last_emit, &mut last_persist);
                } else if let Some(rest) = line.strip_prefix("@@D@@") {
                    let p = rest.trim().trim_matches('"').to_string();
                    if !p.is_empty() && p != "NA" {
                        filepath = Some(p.clone());
                        if let Some(d) = state.patch(&id, |d| d.filepath = Some(p.clone())) {
                            let _ = app.emit("download:update", &d);
                        }
                    }
                } else if let Some(rest) = line.strip_prefix("@@F@@") {
                    let p = rest.trim().trim_matches('"').to_string();
                    if !p.is_empty() {
                        filepath = Some(p);
                    }
                } else if let Some(rest) = line.strip_prefix("[download] Destination: ") {
                    filepath = Some(rest.trim().to_string());
                } else if let Some(rest) = line.strip_prefix("[Merger] Merging formats into ") {
                    filepath = Some(rest.trim().trim_matches('"').to_string());
                }

                if let Some(cur) = state.get(&id) {
                    if cur.status != "downloading" {
                        canceled = true;
                        break;
                    }
                } else {
                    canceled = true;
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    if canceled {
        let pid = state.pids.lock().ok().and_then(|m| m.get(&id).copied());
        if let Some(pid) = pid {
            kill_tree(pid);
        }
        let _ = child.start_kill();
        let _ = child.wait().await;
        let _ = stderr_task.await;
        if let Ok(mut pids) = state.pids.lock() {
            pids.remove(&id);
        }
        // Só limpa o parcial em cancelamento: em pausa o .part é mantido para retomar.
        // Feito depois do wait() porque o yt-dlp ainda mantinha o arquivo aberto.
        if state.get(&id).map(|d| d.status == "canceled").unwrap_or(false) {
            if let Some(p) = filepath.as_deref() {
                cleanup_partial(p);
            }
        }
        drop_extras(&state, &id);
        return;
    }

    let status = child.wait().await;
    let _ = stderr_task.await;
    if let Ok(mut pids) = state.pids.lock() {
        pids.remove(&id);
    }
    drop_extras(&state, &id);

    let current = match state.get(&id) {
        Some(d) => d,
        None => return,
    };

    // Ao cancelar, o processo é morto e o leitor recebe EOF (não uma troca de status),
    // então a limpeza do .part tem que acontecer aqui, depois que o arquivo é liberado.
    if current.status == "canceled" {
        if let Some(p) = filepath.as_deref().or(current.filepath.as_deref()) {
            cleanup_partial(p);
        }
    }

    if current.status != "downloading" {
        return;
    }

    let success = matches!(&status, Ok(s) if s.success());
    if success {
        let path = filepath.or_else(|| current.filepath.clone());
        let title_from_path = path
            .as_ref()
            .and_then(|p| Path::new(p).file_stem().map(|s| s.to_string_lossy().to_string()))
            .filter(|_| current.title.starts_with("http"));
        let updated = state.patch(&id, |d| {
            d.status = "completed".to_string();
            d.progress = 100.0;
            d.speed = 0.0;
            d.eta = Some(0);
            d.finished_at = Some(now_ms());
            d.error = None;
            d.filepath = path.clone();
            if d.total_bytes > 0 {
                d.downloaded_bytes = d.total_bytes;
            }
            if let Some(t) = title_from_path.clone() {
                d.title = t;
            }
        });
        if let Some(d) = updated {
            let _ = app.emit("download:update", &d);
            if settings.notify_on_complete {
                notify(&app, "Download concluído", &d.title);
            }
        }
    } else {
        let details = errbuf
            .lock()
            .map(|b| b.iter().cloned().collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        let msg = clean_error(&details);
        fail(&app, &state, &id, &msg);
    }
}

fn handle_progress(
    app: &AppHandle,
    state: &AppState,
    id: &str,
    payload: &str,
    last_emit: &mut Instant,
    last_persist: &mut Instant,
) {
    let parts: Vec<&str> = payload.split('|').collect();
    let num = |i: usize| -> f64 {
        parts
            .get(i)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(0.0)
    };
    let downloaded = num(0);
    let total = {
        let t = num(1);
        if t > 0.0 {
            t
        } else {
            num(2)
        }
    };
    let speed = num(3);
    let eta = num(4);
    let frag_idx = num(5);
    let frag_count = num(6);

    let progress = if total > 0.0 {
        (downloaded / total * 100.0).clamp(0.0, 100.0)
    } else if frag_count > 0.0 {
        (frag_idx / frag_count * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let updated = state.patch_mem(id, |d| {
        if d.status != "downloading" {
            return;
        }
        if downloaded > 0.0 {
            d.downloaded_bytes = downloaded as i64;
        }
        if total > 0.0 {
            d.total_bytes = total as i64;
        }
        d.speed = speed;
        d.eta = Some(eta as i64);
        if progress > 0.0 {
            d.progress = progress;
        }
    });

    if last_emit.elapsed() >= Duration::from_millis(200) {
        if let Some(d) = &updated {
            let _ = app.emit("download:update", d);
        }
        *last_emit = Instant::now();
    }
    if last_persist.elapsed() >= Duration::from_millis(3000) {
        if let Some(d) = &updated {
            state.commit(d);
        }
        *last_persist = Instant::now();
    }
}

fn fail(app: &AppHandle, state: &AppState, id: &str, message: &str) {
    let updated = state.patch(id, |d| {
        d.status = "error".to_string();
        d.speed = 0.0;
        d.eta = None;
        d.error = Some(message.to_string());
        d.finished_at = Some(now_ms());
    });
    if let Some(d) = updated {
        let _ = app.emit("download:update", &d);
    }
    let _ = app.emit(
        "toast",
        serde_json::json!({ "kind": "error", "message": message }),
    );
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let short: String = body.chars().take(90).collect();
    let _ = app
        .notification()
        .builder()
        .title(title.to_string())
        .body(short)
        .show();
}

pub fn build_args(
    d: &Download,
    s: &Settings,
    ffmpeg: Option<&Path>,
    js_runtime: Option<(&str, &Path)>,
    extras: &crate::state::JobExtras,
) -> Result<Vec<String>, String> {
    let mut a: Vec<String> = Vec::new();
    a.push("--newline".into());
    a.push("--no-colors".into());
    a.push("--ignore-config".into());
    if let Some((name, path)) = js_runtime {
        a.push("--js-runtimes".into());
        a.push(format!("{name}:{}", path.to_string_lossy()));
    }
    a.push("--no-simulate".into());
    a.push("--progress".into());
    a.push("--windows-filenames".into());
    a.push("--no-mtime".into());
    a.push("--retries".into());
    a.push("10".into());
    a.push("--fragment-retries".into());
    a.push("10".into());
    a.push("--socket-timeout".into());
    a.push("20".into());
    a.push("--concurrent-fragments".into());
    a.push("4".into());
    a.push("--progress-template".into());
    a.push(
        "download:@@P@@%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(progress.fragment_index)s|%(progress.fragment_count)s"
            .into(),
    );
    a.push("--print".into());
    a.push("after_move:@@F@@%(filepath)s".into());
    // Informa o destino antes de baixar: permite cancelar/limpar o .part corretamente.
    // `filepath` só existe depois do download; no before_dl o campo válido é `filename`.
    a.push("--print".into());
    a.push("before_dl:@@D@@%(filename)s".into());

    let base = Path::new(&s.output_dir);
    let tpl = s.filename_template.trim_start_matches(|c| c == '/' || c == '\\');
    let full = base.join(tpl);
    a.push("-o".into());
    a.push(match extras.output_template.as_deref() {
        Some(custom) if !custom.trim().is_empty() => custom.trim().to_string(),
        _ => full.to_string_lossy().to_string(),
    });

    if let Some(ff) = ffmpeg {
        a.push("--ffmpeg-location".into());
        a.push(ff.to_string_lossy().to_string());
    }

    let quality = d.quality.clone().unwrap_or_default();
    let audio_only = d.preset == "audio"
        || quality == "mp3"
        || quality == "m4a"
        || quality.starts_with("audio:");

    if audio_only {
        a.push("-f".into());
        a.push("ba/b".into());
        a.push("-x".into());
        let fmt = if quality.starts_with("audio:") {
            quality.split(':').nth(1).unwrap_or(&s.audio_format).to_string()
        } else if quality == "m4a" {
            "m4a".to_string()
        } else {
            s.audio_format.clone()
        };
        a.push("--audio-format".into());
        a.push(fmt);
        a.push("--audio-quality".into());
        a.push("0".into());
    } else {
        let filter = if d.preset == "max" {
            "bv*+ba/b".to_string()
        } else if let Ok(h) = quality.parse::<u32>() {
            format!("bv*[height<={h}]+ba/b[height<={h}]/b")
        } else {
            "bv*[vcodec^=avc1]+ba[ext=m4a]/bv*[vcodec^=avc1]+ba/bv*+ba/b".to_string()
        };
        a.push("-f".into());
        a.push(filter);
        a.push("--merge-output-format".into());
        a.push("mp4/mkv".into());
    }

    if s.embed_metadata && !audio_only {
        a.push("--embed-metadata".into());
    }
    if s.embed_thumbnail {
        a.push("--embed-thumbnail".into());
    }
    if s.embed_subs && !audio_only {
        a.push("--write-subs".into());
        a.push("--write-auto-subs".into());
        a.push("--sub-langs".into());
        a.push(s.sub_langs.clone());
        a.push("--embed-subs".into());
        a.push("--convert-subs".into());
        a.push("srt".into());
    }

    if s.speed_limit_kb > 0 {
        a.push("--limit-rate".into());
        a.push(format!("{}K", s.speed_limit_kb));
    }

    if let Some(c) = extras.cookies.as_deref() {
        a.push("--cookies".into());
        a.push(c.to_string_lossy().to_string());
    } else if !s.cookies_browser.trim().is_empty() {
        a.push("--cookies-from-browser".into());
        a.push(s.cookies_browser.trim().to_string());
    }

    if let Some(ua) = extras.user_agent.as_deref() {
        if !ua.trim().is_empty() {
            a.push("--user-agent".into());
            a.push(ua.trim().to_string());
        }
    }

    if let Some(r) = extras.referer.as_deref() {
        if !r.trim().is_empty() {
            a.push("--referer".into());
            a.push(r.trim().to_string());
        }
    }

    match d.playlist_items.as_deref() {
        Some(items) if !items.trim().is_empty() => {
            a.push("--yes-playlist".into());
            a.push("--playlist-items".into());
            a.push(items.trim().to_string());
        }
        _ => {
            a.push("--no-playlist".into());
        }
    }

    a.push(
        extras
            .source_url
            .clone()
            .unwrap_or_else(|| d.url.clone()),
    );
    Ok(a)
}

pub fn clean_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("confirm your age") || lower.contains("age-restricted") {
        return "O site exige login para confirmar idade. Configure os cookies do navegador em Configurações → Avançado.".into();
    }
    if lower.contains("private video") {
        return "Este vídeo é privado.".into();
    }
    if lower.contains("members-only") || lower.contains("join this channel") {
        return "Conteúdo exclusivo para membros do canal.".into();
    }
    if lower.contains("not available in your country") || lower.contains("geo restricted") {
        return "Vídeo bloqueado na sua região.".into();
    }
    if lower.contains("video unavailable") || lower.contains("this video is unavailable") {
        return "Vídeo indisponível (removido, bloqueado ou restrito).".into();
    }
    if lower.contains("http error 403") {
        return "Acesso negado (403). Atualize o yt-dlp ou configure cookies do navegador.".into();
    }
    if lower.contains("unsupported url") {
        return "URL não suportada.".into();
    }
    if lower.contains("ffmpeg") && (lower.contains("not found") || lower.contains("not installed")) {
        return "ffmpeg não encontrado. Instale em Configurações → Binários.".into();
    }
    let last = raw
        .lines()
        .rev()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("[download]"))
        .unwrap_or("Falha desconhecida no download.");
    let clean = last
        .trim_start_matches("ERROR:")
        .trim_start_matches("WARNING:")
        .trim()
        .to_string();
    if clean.chars().count() > 300 {
        format!("{}…", clean.chars().take(300).collect::<String>())
    } else if clean.is_empty() {
        "Falha desconhecida no download.".to_string()
    } else {
        clean
    }
}

pub fn cleanup_partial(path: &str) {
    let p = PathBuf::from(path);
    let _ = std::fs::remove_file(&p);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    if stem.is_empty() {
        return;
    }
    // Em downloads com merge o destino é "titulo.f137.mp4"; o parcial é "titulo.f137.mp4.part",
    // mas o outro stream usa outro sufixo. Removemos o sufixo de formato para pegar os dois.
    let base = match stem.rfind(".f") {
        Some(i) if stem[i + 2..].chars().all(|c| c.is_ascii_digit()) && i > 0 => stem[..i].to_string(),
        _ => stem.clone(),
    };
    if let Some(parent) = p.parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // só considera arquivos do mesmo download (separador '.' após o prefixo)
                let mesmo = name.starts_with(&base)
                    && (name.len() == base.len() || name.as_bytes()[base.len()] == b'.');
                let partial = name.ends_with(".part")
                    || name.ends_with(".ytdl")
                    || name.contains(".part-Frag");
                if mesmo && partial {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

fn drop_extras(state: &AppState, id: &str) {
    if let Ok(mut map) = state.extras.lock() {
        if let Some(extra) = map.remove(id) {
            if let Some(path) = extra.cookies {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}
