use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn hide(cmd: &mut Command) {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

pub fn hide_tokio(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

pub fn app_data(_app: &tauri::AppHandle) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VidManager")
}

pub fn bin_dir(app: &tauri::AppHandle) -> PathBuf {
    app_data(app).join("bin")
}

pub fn cookies_dir(app: &tauri::AppHandle) -> PathBuf {
    app_data(app).join("cookies")
}

pub fn find_in_path(exe: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for part in std::env::split_paths(&paths) {
        let candidate = part.join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn resolve_ytdlp(app: &tauri::AppHandle) -> Option<PathBuf> {
    let local = bin_dir(app).join("yt-dlp.exe");
    if local.is_file() {
        return Some(local);
    }
    find_in_path("yt-dlp.exe")
}

pub fn resolve_ffmpeg(app: &tauri::AppHandle) -> Option<PathBuf> {
    let local = bin_dir(app).join("ffmpeg.exe");
    if local.is_file() {
        return Some(local);
    }
    find_in_path("ffmpeg.exe")
}

/// Runtime JavaScript usado pelo yt-dlp para decifrar assinaturas do YouTube.
pub fn resolve_js_runtime(app: &tauri::AppHandle) -> Option<(String, PathBuf)> {
    let local = bin_dir(app).join("deno.exe");
    if local.is_file() {
        return Some(("deno".to_string(), local));
    }
    if let Some(path) = find_in_path("deno.exe") {
        return Some(("deno".to_string(), path));
    }
    if let Some(path) = find_in_path("node.exe") {
        return Some(("node".to_string(), path));
    }
    None
}

pub fn version_of(path: &Path, arg: &str) -> Option<String> {
    let mut cmd = Command::new(path);
    cmd.arg(arg)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hide(&mut cmd);
    let out = cmd.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text.lines().find(|l| !l.trim().is_empty())?;
    let mut v = first.trim().to_string();
    if v.len() > 120 {
        v = v.chars().take(120).collect();
    }
    Some(v)
}

pub fn ytdlp_version(path: &Path) -> Option<String> {
    version_of(path, "--version")
}

pub fn ffmpeg_version(path: &Path) -> Option<String> {
    let raw = version_of(path, "-version")?;
    let part = raw
        .split("Copyright")
        .next()
        .unwrap_or(&raw)
        .trim()
        .to_string();
    Some(part)
}

pub async fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Não foi possível criar a pasta: {e}"))?;
    }
    let dest_str = dest.to_string_lossy().to_string();
    let mut cmd = tokio::process::Command::new("curl.exe");
    cmd.args([
        "-L",
        "--fail",
        "--silent",
        "--show-error",
        "--retry",
        "3",
        "--connect-timeout",
        "20",
        "-o",
        dest_str.as_str(),
        url,
    ])
    .stdin(Stdio::null());
    hide_tokio(&mut cmd);
    let out = cmd
        .output()
        .await
        .map_err(|e| format!("curl não encontrado no sistema: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "Falha ao baixar o arquivo.".to_string()
        } else {
            format!("Falha ao baixar: {err}")
        });
    }
    if !dest.is_file() {
        return Err("O download não gerou arquivo.".to_string());
    }
    Ok(())
}

pub async fn install_ytdlp(app: &tauri::AppHandle) -> Result<String, String> {
    let dir = bin_dir(app);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar a pasta de binários: {e}"))?;
    let tmp = dir.join("yt-dlp.new.exe");
    let _ = std::fs::remove_file(&tmp);
    download_file(
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
        &tmp,
    )
    .await?;
    let final_path = dir.join("yt-dlp.exe");
    if final_path.is_file() {
        std::fs::remove_file(&final_path).map_err(|_| {
            "O yt-dlp está em uso por um download ativo. Pause os downloads e tente novamente."
                .to_string()
        })?;
    }
    std::fs::rename(&tmp, &final_path).map_err(|e| format!("Falha ao instalar o yt-dlp: {e}"))?;
    Ok(ytdlp_version(&final_path).unwrap_or_else(|| "desconhecida".to_string()))
}

pub async fn install_ffmpeg(app: &tauri::AppHandle) -> Result<String, String> {
    let dir = bin_dir(app);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar a pasta de binários: {e}"))?;
    let zip_path = dir.join("ffmpeg-download.zip");
    download_file(
        "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
        &zip_path,
    )
    .await?;
    let bytes = std::fs::read(&zip_path).map_err(|e| format!("Falha ao ler o arquivo baixado: {e}"))?;
    let target = dir.join("ffmpeg.exe");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Pacote do ffmpeg inválido: {e}"))?;
    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().replace('\\', "/");
        if name.ends_with("/bin/ffmpeg.exe") || name == "ffmpeg.exe" {
            let mut out = std::fs::File::create(&target)
                .map_err(|e| format!("Falha ao gravar ffmpeg.exe: {e}"))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Falha ao extrair ffmpeg.exe: {e}"))?;
            found = true;
            break;
        }
    }
    let _ = std::fs::remove_file(&zip_path);
    if !found {
        return Err("O pacote baixado não contém o ffmpeg.exe.".to_string());
    }
    Ok(ffmpeg_version(&target).unwrap_or_else(|| "desconhecida".to_string()))
}

pub async fn install_deno(app: &tauri::AppHandle) -> Result<String, String> {
    let dir = bin_dir(app);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar a pasta de binários: {e}"))?;
    let zip_path = dir.join("deno-download.zip");
    download_file(
        "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip",
        &zip_path,
    )
    .await?;
    let bytes = std::fs::read(&zip_path).map_err(|e| format!("Falha ao ler o arquivo baixado: {e}"))?;
    let target = dir.join("deno.exe");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Pacote do Deno inválido: {e}"))?;
    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().replace('\\', "/");
        if name == "deno.exe" || name.ends_with("/deno.exe") {
            let mut out = std::fs::File::create(&target)
                .map_err(|e| format!("Falha ao gravar deno.exe: {e}"))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Falha ao extrair deno.exe: {e}"))?;
            found = true;
            break;
        }
    }
    let _ = std::fs::remove_file(&zip_path);
    if !found {
        return Err("O pacote baixado não contém o deno.exe.".to_string());
    }
    Ok(version_of(&target, "--version").unwrap_or_else(|| "deno".to_string()))
}

