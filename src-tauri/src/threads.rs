use crate::models::{Download, ProbeResult};
use crate::state::{now_ms, AppState};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::process::Stdio;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// O Meta serve o post inteiro (com as URLs do CDN) para crawlers, sem login.
const UA_CRAWLER: &str =
    "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";

#[derive(Debug, Clone)]
pub struct ThreadsPost {
    pub post_url: String,
    pub id: String,
    pub uploader: Option<String>,
    pub title: String,
    pub thumbnail: Option<String>,
    /// URLs diretas (mp4 progressivo, já com áudio), na ordem do post.
    pub videos: Vec<String>,
}

pub fn is_threads_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    let host = match lower.split("://").nth(1) {
        Some(rest) => rest.split(['/', '?', '#']).next().unwrap_or(""),
        None => return false,
    };
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    host == "threads.com"
        || host == "threads.net"
        || host.ends_with(".threads.com")
        || host.ends_with(".threads.net")
}

async fn fetch_html(url: &str) -> Result<(String, String), String> {
    let mut cmd = tokio::process::Command::new("curl.exe");
    cmd.args([
        "-s",
        "-L",
        "--compressed",
        "--max-time",
        "60",
        "--retry",
        "2",
        "--retry-delay",
        "1",
        "-A",
        UA_CRAWLER,
        "-H",
        "Accept-Language: pt-BR,pt;q=0.9,en;q=0.8",
        "-H",
        "Accept: text/html,application/xhtml+xml",
        "-w",
        "\n@@FINAL@@%{url_effective}",
    ]);
    cmd.arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::binaries::hide_tokio(&mut cmd);

    let out = tokio::time::timeout(Duration::from_secs(80), cmd.output())
        .await
        .map_err(|_| "Tempo esgotado ao abrir o post do Threads.".to_string())?
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "curl não encontrado no sistema.".to_string()
            } else {
                format!("Falha ao acessar o Threads: {e}")
            }
        })?;

    let raw = String::from_utf8_lossy(&out.stdout).to_string();
    let (body, final_url) = match raw.rfind("@@FINAL@@") {
        Some(i) => (raw[..i].to_string(), raw[i + 9..].trim().to_string()),
        None => (raw, url.to_string()),
    };
    if !out.status.success() && body.len() < 4000 {
        return Err("Não foi possível abrir o post do Threads.".to_string());
    }
    if final_url.contains("error=invalid_post") {
        return Err("Post do Threads não encontrado (pode ter sido removido).".to_string());
    }
    Ok((body, final_url))
}

/// Percorre o HTML e coleta as URLs de `video_versions` (arrays JSON embutidos).
fn extract_video_urls(html: &str) -> Vec<String> {
    let key = "\"video_versions\":";
    let bytes = html.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut from = 0usize;

    while let Some(rel) = html[from..].find(key) {
        let i = from + rel;
        let mut j = i + key.len();
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'[' {
            let mut depth = 0i32;
            let mut k = j;
            let mut end = None;
            while k < bytes.len() {
                match bytes[k] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(k);
                            break;
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            if let Some(e) = end {
                if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&html[j..=e]) {
                    for item in items {
                        if let Some(u) = item["url"].as_str() {
                            let u = u.trim().to_string();
                            // As variantes (101/102/103) repetem a mesma URL: deduplica.
                            if u.starts_with("http") && seen.insert(u.clone()) {
                                out.push(u);
                            }
                        }
                    }
                }
            }
        }
        from = i + key.len();
    }

    // Fallback: alguns posts só trazem a tag og:video.
    if out.is_empty() {
        if let Some(u) = og_content(html, "og:video") {
            if u.starts_with("http") {
                out.push(u);
            }
        }
    }
    out
}

/// Primeiro valor de string de um campo JSON simples (ex.: `"username":"x"`).
fn first_string(html: &str, key: &str) -> Option<String> {
    let i = html.find(key)?;
    let rest = &html[i + key.len()..];
    let mut raw = String::new();
    let mut escaped = false;
    for ch in rest.chars() {
        if escaped {
            raw.push('\\');
            raw.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        } else {
            raw.push(ch);
        }
    }
    let unescaped = serde_json::from_str::<String>(&format!("\"{raw}\"")).unwrap_or(raw);
    let clean = unescaped.trim().to_string();
    if clean.is_empty() {
        None
    } else {
        Some(clean)
    }
}

/// Legenda do post (primeira `"caption"` com texto).
fn caption_text(html: &str) -> Option<String> {
    let mut from = 0usize;
    while let Some(rel) = html[from..].find("\"caption\":") {
        let i = from + rel;
        let rest = html[i + "\"caption\":".len()..].trim_start();
        // Lê só o objeto da legenda (o HTML inteiro não é JSON válido).
        let mut de = serde_json::Deserializer::from_str(rest);
        if let Ok(value) = Value::deserialize(&mut de) {
            if let Some(text) = value.get("text").and_then(|t| t.as_str()) {
                let text = text.trim();
                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
        from = i + 10;
    }
    None
}

fn og_content(html: &str, property: &str) -> Option<String> {
    let needle = format!("property=\"{property}\"");
    let i = html.find(&needle)?;
    let tag_start = html[..i].rfind('<')?;
    let tag_end = html[i..].find('>')? + i;
    let tag = &html[tag_start..tag_end];
    let c = tag.find("content=\"")? + "content=\"".len();
    let rest = &tag[c..];
    let end = rest.find('"')?;
    let value = desescapar_html(&rest[..end]);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Desfaz as entidades usadas em atributos HTML (`&amp;`, `&#064;` etc.).
fn desescapar_html(text: &str) -> String {
    let mut out = text
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    while let Some(start) = out.find("&#") {
        let Some(len) = out[start..].find(';') else {
            break;
        };
        let digits = &out[start + 2..start + len];
        let codigo = if let Some(hex) = digits.strip_prefix('x').or_else(|| digits.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            digits.parse::<u32>().ok()
        };
        match codigo.and_then(char::from_u32) {
            Some(ch) => out.replace_range(start..start + len + 1, &ch.to_string()),
            None => break,
        }
    }
    out
}

fn post_id(url: &str) -> Option<String> {
    let i = url.find("/post/")? + "/post/".len();
    let tail = &url[i..];
    let id: String = tail
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

fn sem_espacos(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncar(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out = out.trim_end().to_string();
    format!("{out}…")
}

/// Nome de arquivo seguro para Windows (sem separadores, caracteres inválidos ou `%`).
fn sanitizar(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '%' => out.push(' '),
            c if (c as u32) < 32 => out.push(' '),
            c => out.push(c),
        }
    }
    let out = sem_espacos(&out);
    let out = out.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    truncar(&out, 90)
}

fn nome_relativo(post: &ThreadsPost, indice: usize, total: usize) -> String {
    let autor = post
        .uploader
        .as_deref()
        .map(sanitizar)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Threads".to_string());
    let mut base = sanitizar(&post.title);
    if base.is_empty() {
        base = "Vídeo do Threads".to_string();
    }
    let sufixo = if total > 1 {
        format!(" ({indice} de {total})")
    } else {
        String::new()
    };
    format!("{autor}\\{base} [{}]{sufixo}", post.id)
}

pub async fn resolve(url: &str) -> Result<ThreadsPost, String> {
    let (html, final_url) = fetch_html(url).await?;
    let videos = extract_video_urls(&html);
    if videos.is_empty() {
        return Err("Este post do Threads não tem vídeo (é texto ou imagem).".to_string());
    }
    let id = post_id(&final_url)
        .or_else(|| post_id(url))
        .unwrap_or_else(|| "threads".to_string());
    let uploader = first_string(&html, "\"username\":\"");
    let titulo = match caption_text(&html) {
        Some(c) => {
            let t = sem_espacos(&c);
            if t.is_empty() {
                "Vídeo do Threads".to_string()
            } else {
                truncar(&t, 110)
            }
        }
        None => "Vídeo do Threads".to_string(),
    };
    let thumbnail = og_content(&html, "og:image");

    Ok(ThreadsPost {
        post_url: final_url,
        id,
        uploader,
        title: titulo,
        thumbnail,
        videos,
    })
}

pub async fn probe(url: &str) -> Result<ProbeResult, String> {
    let post = resolve(url).await?;
    let total = post.videos.len();
    let title = if total > 1 {
        format!("{} · {total} vídeos", post.title)
    } else {
        post.title.clone()
    };
    Ok(ProbeResult {
        url: post.post_url,
        title,
        uploader: post.uploader,
        thumbnail: post.thumbnail,
        duration: None,
        is_playlist: false,
        playlist_count: None,
        resolutions: Vec::new(),
    })
}

/// Prepara um download do Threads: resolve o post, nomeia os arquivos e,
/// num carrossel, cria um registro por vídeo (o scheduler inicia cada um).
pub async fn prepare(app: &AppHandle, state: &AppState, id: &str) -> Result<(), String> {
    // Já resolvido nesta sessão (retomada/retry usam a mesma URL direta).
    // Atenção: a extensão também cria `extras` (cookies/referer) antes do download —
    // por isso o sinal de "pronto" é a URL direta, não a mera existência da entrada.
    let pronto = state
        .extras
        .lock()
        .ok()
        .and_then(|m| m.get(id).map(|e| e.source_url.is_some()))
        .unwrap_or(false);
    if pronto {
        return Ok(());
    }

    let dl = state.get(id).ok_or("Download não encontrado.")?;
    let post = resolve(&dl.url).await?;
    let total = post.videos.len();

    let indice = dl
        .source
        .strip_prefix("threads:")
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|i| *i >= 1 && *i <= total)
        .unwrap_or(1);

    let ja_expandido = state
        .all()
        .iter()
        .any(|d| d.url == dl.url && d.source.starts_with("threads:"));

    vincular(app, state, id, &post, indice, total);

    if indice == 1 && total > 1 && !ja_expandido {
        for i in 2..=total {
            let mut novo: Download = dl.clone();
            novo.id = uuid::Uuid::new_v4().to_string();
            novo.title = String::new();
            novo.uploader = None;
            novo.thumbnail = None;
            novo.status = "queued".to_string();
            novo.progress = 0.0;
            novo.downloaded_bytes = 0;
            novo.total_bytes = 0;
            novo.speed = 0.0;
            novo.eta = None;
            novo.filepath = None;
            novo.error = None;
            novo.finished_at = None;
            novo.created_at = now_ms() + i as i64;
            novo.source = format!("threads:{i}");
            state.put(novo.clone());
            let _ = app.emit("download:update", &novo);
            vincular(app, state, &novo.id, &post, i, total);
        }
    }

    Ok(())
}

/// Grava nome, autor, capa e a URL direta do vídeo `indice` no registro `id`.
fn vincular(
    app: &AppHandle,
    state: &AppState,
    id: &str,
    post: &ThreadsPost,
    indice: usize,
    total: usize,
) {
    let output_dir = state
        .settings
        .lock()
        .map(|s| s.output_dir.clone())
        .unwrap_or_default();
    let relativo = nome_relativo(post, indice, total);
    let template = format!(
        "{}\\{relativo}.%(ext)s",
        output_dir.trim_end_matches(['\\', '/'])
    );
    let titulo = if total > 1 {
        // Índice na frente: na lista o título é truncado no fim, então o
        // sufixo "(3 de 9)" sumiria e os itens ficariam iguais.
        format!("{indice}/{total} · {}", post.title)
    } else {
        post.title.clone()
    };

    let updated = state.patch(id, |d| {
        d.title = titulo;
        d.uploader = post.uploader.clone();
        d.thumbnail = post.thumbnail.clone();
        d.source = format!("threads:{indice}");
    });

    if let Ok(mut extras) = state.extras.lock() {
        // Preserva cookies/referer que a extensão tenha enviado.
        let mut job = extras.get(id).cloned().unwrap_or_default();
        job.referer = Some(post.post_url.clone());
        job.source_url = post.videos.get(indice - 1).cloned();
        job.output_template = Some(template);
        job.user_agent = Some(UA_CRAWLER.to_string());
        extras.insert(id.to_string(), job);
    }

    if let Some(d) = updated {
        let _ = app.emit("download:update", &d);
    }
}
