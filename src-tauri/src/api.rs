use crate::binaries;
use crate::models::{CookieInput, Download, ExtensionDownload};
use crate::state::{now_ms, AppState, JobExtras};
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

type ApiResult = Result<Json<Value>, (StatusCode, Json<Value>)>;

pub fn spawn(app: AppHandle, port: u16) {
    tauri::async_runtime::spawn(async move {
        let addr = format!("127.0.0.1:{port}");
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                let _ = app.emit(
                    "toast",
                    json!({ "kind": "error", "message": format!("API local não pôde iniciar na porta {port}: {e}") }),
                );
                return;
            }
        };
        let router = Router::new()
            .route("/ping", get(ping))
            .route("/pair", post(pair))
            .route("/download", post(download))
            .layer(middleware::from_fn(cors))
            .with_state(app);
        let _ = axum::serve(listener, router).await;
    });
}

async fn ping(State(app): State<AppHandle>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "app": "VidManager",
        "version": app.package_info().version.to_string()
    }))
}

#[derive(Deserialize)]
struct PairBody {
    code: String,
}

async fn pair(State(app): State<AppHandle>, Json(body): Json<PairBody>) -> ApiResult {
    let state = app.state::<AppState>();
    let valid = {
        let pair = state
            .pair
            .lock()
            .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Estado indisponível"))?;
        pair.code == body.code.trim() && pair.expires_at > now_ms()
    };
    if !valid {
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "Código de pareamento inválido ou expirado. Gere um novo código no app.",
        ));
    }
    let token = state
        .settings
        .lock()
        .map(|s| s.api_token.clone())
        .unwrap_or_default();
    Ok(Json(json!({ "ok": true, "token": token })))
}

async fn download(
    State(app): State<AppHandle>,
    headers: HeaderMap,
    Json(body): Json<ExtensionDownload>,
) -> ApiResult {
    if !token_ok(&app, &headers) {
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "Token inválido. Pareie a extensão novamente.",
        ));
    }
    let url = body.url.trim().to_string();
    if !crate::util::is_probable_url(&url) {
        return Err(err(StatusCode::BAD_REQUEST, "URL inválida."));
    }

    let state = app.state::<AppState>();
    if let Some(existing) = state.exist_by_url(&url) {
        return Ok(Json(json!({ "ok": true, "id": existing.id, "duplicate": true })));
    }

    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let id = uuid::Uuid::new_v4().to_string();

    let mut extras = JobExtras::default();
    if let Some(cookies) = body.cookies.as_ref() {
        if !cookies.is_empty() {
            extras.cookies = write_cookies(&app, &id, cookies);
        }
    }
    extras.referer = body.referer.clone();
    let has_extras = extras.cookies.is_some() || extras.referer.is_some();
    if has_extras {
        if let Ok(mut map) = state.extras.lock() {
            map.insert(id.clone(), extras);
        }
    }

    let preset = body
        .preset
        .clone()
        .filter(|p| matches!(p.as_str(), "best" | "max" | "audio"))
        .unwrap_or(settings.preset.clone());

    let d = Download {
        id: id.clone(),
        url: url.clone(),
        title: body
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| url.clone()),
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
        quality: None,
        playlist_items: None,
        created_at: now_ms(),
        finished_at: None,
        source: "extension".to_string(),
    };

    state.put(d.clone());
    let _ = app.emit("download:update", &d);
    let _ = app.emit(
        "extension:download",
        json!({ "id": d.id, "title": d.title }),
    );
    Ok(Json(json!({ "ok": true, "id": d.id, "title": d.title })))
}

fn token_ok(app: &AppHandle, headers: &HeaderMap) -> bool {
    let token = app
        .state::<AppState>()
        .settings
        .lock()
        .map(|s| s.api_token.clone())
        .unwrap_or_default();
    if token.is_empty() {
        return false;
    }
    headers
        .get("x-vidmanager-token")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == token)
        .unwrap_or(false)
}

fn write_cookies(app: &AppHandle, id: &str, cookies: &[CookieInput]) -> Option<PathBuf> {
    let dir = binaries::cookies_dir(app);
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{id}.txt"));
    let mut out = String::from("# Netscape HTTP Cookie File\n\n");
    for c in cookies {
        let domain = c.domain.trim().to_string();
        if domain.is_empty() || c.name.trim().is_empty() {
            continue;
        }
        let flag = if domain.starts_with('.') { "TRUE" } else { "FALSE" };
        let pathv = c.path.clone().unwrap_or_else(|| "/".to_string());
        let secure = if c.secure.unwrap_or(false) { "TRUE" } else { "FALSE" };
        let expiry = c.expiration_date.unwrap_or(0.0) as i64;
        let value = c.value.replace(|ch| ch == '\n' || ch == '\r', "");
        let name = c.name.trim().replace(|ch| ch == '\n' || ch == '\r', "");
        out.push_str(&format!(
            "{domain}\t{flag}\t{pathv}\t{secure}\t{expiry}\t{name}\t{value}\n"
        ));
    }
    std::fs::write(&path, out).ok()?;
    Some(path)
}

async fn cors(req: Request, next: Next) -> Response {
    let preflight = req.method() == Method::OPTIONS;
    let mut res = if preflight {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(req).await
    };
    let h = res.headers_mut();
    h.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type, x-vidmanager-token"),
    );
    h.insert(header::ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("86400"));
    res
}

fn err(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "ok": false, "error": message })))
}
