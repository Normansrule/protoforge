//! # forge-server
//!
//! A small local representational state transfer (REST) server around the
//! `forge-core` engine. It exposes exactly the same JSON method API the browser
//! reaches through WebAssembly, so scripts, notebooks and other tools can
//! drive Protoforge over HTTP:
//!
//! | Route | What it does |
//! |---|---|
//! | `GET  /api/health` | liveness + version + engine module list |
//! | `GET  /api/methods` | engine modules (`api.methods`) |
//! | `POST /api/{method}` | JSON body → `forge_core::api::call_value` → `{"ok": …}` or `{"error": …}` |
//! | `GET  /api/{method}` | same, with a `null` input (handy for `project.iris`, `mfg.vendors`, …) |
//! | `POST /api/package` | project JSON (or empty body = Iris) → `.zip` fabrication package |
//! | `POST /api/ngspice` | SPICE netlist → runs a local `ngspice -b` and returns its output |
//! | anything else | static files from `--static DIR` with single-page-app fallback |
//!
//! Engine calls are CPU-bound, so they run on Tokio's blocking pool
//! (`spawn_blocking`) and never stall the async reactor.

// Handlers return early with ready-made `Response`s as their error type.
#![allow(clippy::result_large_err)]

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path as UrlPath, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use forge_core::project::Project;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

/// Server configuration (see `forge-server --help`).
#[derive(Clone, Debug)]
pub struct Config {
    /// Directory with the built web app (`web/dist`), served at `/`.
    pub static_dir: Option<PathBuf>,
    /// Maximum accepted request body in bytes.
    pub body_limit: usize,
    /// Wall-clock limit for one ngspice run.
    pub ngspice_timeout: Duration,
    /// Name or path of the ngspice executable.
    pub ngspice_bin: String,
    /// Allow cross-origin requests from any origin (local development).
    pub cors: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            static_dir: None,
            body_limit: 32 * 1024 * 1024,
            ngspice_timeout: Duration::from_secs(30),
            ngspice_bin: "ngspice".into(),
            cors: true,
        }
    }
}

/// Build the router. Used by `main` and by the integration tests.
pub fn app(cfg: Config) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/methods", get(methods))
        .route("/package", post(package).get(package_iris))
        .route("/ngspice", post(ngspice))
        .route("/{method}", post(call_method).get(call_method_get))
        .fallback(api_not_found);

    let mut router = Router::new().nest("/api", api);
    router = match &cfg.static_dir {
        Some(dir) => {
            let index = dir.join("index.html");
            router.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)))
        }
        None => router.route("/", get(root_info)),
    };
    router = router.layer(DefaultBodyLimit::max(cfg.body_limit));
    if cfg.cors {
        router = router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .expose_headers([header::CONTENT_DISPOSITION]),
        );
    }
    router.with_state(cfg)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// JSON error envelope with a status code.
fn error(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

/// Map an engine error string to an HTTP status.
fn engine_status(msg: &str) -> StatusCode {
    if msg.starts_with("unknown method") || msg.starts_with("unknown module") {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    }
}

/// Parse a request body as JSON; an empty body is `null`.
fn parse_body(body: &Bytes) -> Result<Value, Response> {
    if body.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(Value::Null);
    }
    serde_json::from_slice(body).map_err(|e| error(StatusCode::BAD_REQUEST, format!("invalid JSON body: {e}")))
}

/// Run a CPU-heavy closure on the blocking pool.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, Response> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| error(StatusCode::INTERNAL_SERVER_ERROR, format!("engine task failed: {e}")))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn root_info() -> Json<Value> {
    Json(json!({
        "name": "protoforge",
        "message": "Protoforge REST server. Try GET /api/health, GET /api/methods or POST /api/<method>. \
                    Start with --static web/dist to serve the web app here.",
    }))
}

async fn health() -> Json<Value> {
    let modules = forge_core::api::call_value("api.methods", &Value::Null).unwrap_or(Value::Null);
    Json(json!({
        "status": "ok",
        "name": "protoforge",
        "version": env!("CARGO_PKG_VERSION"),
        "modules": modules,
        "ngspice": find_on_path("ngspice").is_some(),
    }))
}

async fn methods() -> Response {
    match forge_core::api::call_value("api.methods", &Value::Null) {
        Ok(v) => Json(json!({ "ok": v })).into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, e),
    }
}

async fn api_not_found() -> Response {
    error(StatusCode::NOT_FOUND, "no such route — see GET /api/health")
}

async fn call_method(UrlPath(method): UrlPath<String>, body: Bytes) -> Response {
    let input = match parse_body(&body) {
        Ok(v) => v,
        Err(r) => return r,
    };
    run_method(method, input).await
}

async fn call_method_get(UrlPath(method): UrlPath<String>) -> Response {
    run_method(method, Value::Null).await
}

async fn run_method(method: String, input: Value) -> Response {
    let out = match blocking(move || forge_core::api::call_value(&method, &input)).await {
        Ok(r) => r,
        Err(r) => return r,
    };
    match out {
        Ok(v) => Json(json!({ "ok": v })).into_response(),
        Err(e) => error(engine_status(&e), e),
    }
}

async fn package_iris() -> Response {
    package(Bytes::new()).await
}

/// Project JSON in, `.zip` of `forge_core::mfg::package` out.
async fn package(body: Bytes) -> Response {
    let project: Project = match parse_body(&body) {
        Ok(Value::Null) => Project::iris(),
        Ok(v) => match serde_json::from_value(v) {
            Ok(p) => p,
            Err(e) => return error(StatusCode::UNPROCESSABLE_ENTITY, format!("body is not a valid project: {e}")),
        },
        Err(r) => return r,
    };
    let id = sanitize(&project.meta.id);
    let zipped = match blocking(move || build_zip(&project)).await {
        Ok(Ok(z)) => z,
        Ok(Err(e)) => return error(StatusCode::INTERNAL_SERVER_ERROR, e),
        Err(r) => return r,
    };
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    let disp = format!(
        "attachment; filename=\"{}-fab-package.zip\"",
        if id.is_empty() { "project" } else { &id }
    );
    if let Ok(v) = HeaderValue::from_str(&disp) {
        headers.insert(header::CONTENT_DISPOSITION, v);
    }
    (StatusCode::OK, headers, zipped).into_response()
}

fn sanitize(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect()
}

/// Build the fabrication-package zip in memory (base64 entries are decoded).
pub fn build_zip(project: &Project) -> Result<Vec<u8>, String> {
    let files = forge_core::mfg::package(project);
    let mut zw = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for f in &files {
        let data = if f.encoding == "base64" {
            let clean: String = f.content.chars().filter(|c| !c.is_ascii_whitespace()).collect();
            base64::engine::general_purpose::STANDARD
                .decode(clean)
                .map_err(|e| format!("{}: bad base64: {e}", f.path))?
        } else {
            f.content.as_bytes().to_vec()
        };
        zw.start_file(f.path.as_str(), opts).map_err(|e| format!("zip {}: {e}", f.path))?;
        zw.write_all(&data).map_err(|e| format!("zip {}: {e}", f.path))?;
    }
    let cur = zw.finish().map_err(|e| format!("zip: {e}"))?;
    Ok(cur.into_inner())
}

/// Look up an executable on `PATH` (no extra crates).
pub fn find_on_path(bin: &str) -> Option<PathBuf> {
    let p = Path::new(bin);
    if p.components().count() > 1 {
        return p.is_file().then(|| p.to_path_buf());
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(bin)).find(|c| c.is_file())
}

#[derive(Deserialize)]
struct NgspiceIn {
    netlist: String,
}

/// Run a netlist through a locally installed ngspice in batch mode.
///
/// Body: `{"netlist": "..."}` or the raw netlist text. The deck is written to
/// a temporary directory and run as `ngspice -b -r out.raw deck.cir`.
async fn ngspice(State(cfg): State<Config>, headers: HeaderMap, body: Bytes) -> Response {
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("json"));
    let netlist = if is_json {
        match serde_json::from_slice::<NgspiceIn>(&body) {
            Ok(i) => i.netlist,
            Err(e) => return error(StatusCode::BAD_REQUEST, format!("expected {{\"netlist\": \"...\"}}: {e}")),
        }
    } else {
        String::from_utf8_lossy(&body).into_owned()
    };
    if netlist.trim().is_empty() {
        return error(StatusCode::BAD_REQUEST, "empty netlist");
    }
    let Some(bin) = find_on_path(&cfg.ngspice_bin) else {
        return error(
            StatusCode::NOT_IMPLEMENTED,
            format!(
                "`{}` was not found on PATH. Install it to cross-check Circuit Lab results \
                 (Ubuntu / WSL2: `sudo apt install ngspice`, macOS: `brew install ngspice`) and restart the server.",
                cfg.ngspice_bin
            ),
        );
    };
    let dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => return error(StatusCode::INTERNAL_SERVER_ERROR, format!("temp dir: {e}")),
    };
    let deck = dir.path().join("deck.cir");
    let raw = dir.path().join("out.raw");
    // SPICE treats the first line as the title and needs a final `.end`.
    let mut text = netlist;
    if !text.to_ascii_lowercase().contains("\n.end") {
        text.push_str("\n.end\n");
    }
    if let Err(e) = tokio::fs::write(&deck, text).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, format!("write deck: {e}"));
    }
    let t0 = Instant::now();
    let child = tokio::process::Command::new(bin)
        .arg("-b")
        .arg("-r")
        .arg(&raw)
        .arg(&deck)
        .current_dir(dir.path())
        .kill_on_drop(true)
        .output();
    let out = match tokio::time::timeout(cfg.ngspice_timeout, child).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return error(StatusCode::INTERNAL_SERVER_ERROR, format!("could not start ngspice: {e}")),
        Err(_) => {
            return error(
                StatusCode::GATEWAY_TIMEOUT,
                format!("ngspice did not finish within {} s", cfg.ngspice_timeout.as_secs()),
            )
        }
    };
    let raw_bytes = tokio::fs::read(&raw).await.ok();
    let raw_b64 = raw_bytes
        .as_ref()
        .filter(|b| b.len() <= 16 * 1024 * 1024)
        .map(|b| base64::engine::general_purpose::STANDARD.encode(b));
    Json(json!({
        "ok": {
            "exit_code": out.status.code(),
            "success": out.status.success(),
            "stdout": String::from_utf8_lossy(&out.stdout),
            "stderr": String::from_utf8_lossy(&out.stderr),
            "elapsed_ms": t0.elapsed().as_secs_f64() * 1e3,
            "raw_bytes": raw_bytes.as_ref().map(|b| b.len()),
            "raw_base64": raw_b64,
        }
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_errors_map_to_statuses() {
        assert_eq!(engine_status("unknown method `dsp.nope`"), StatusCode::NOT_FOUND);
        assert_eq!(engine_status("unknown module `x`"), StatusCode::NOT_FOUND);
        assert_eq!(engine_status("bad input: missing field"), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn sanitizes_ids() {
        assert_eq!(sanitize("iris/../x y"), "irisxy");
    }

    #[test]
    fn missing_binary_is_none() {
        assert!(find_on_path("definitely-not-a-real-binary-4313").is_none());
    }
}
