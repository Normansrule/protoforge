//! End-to-end tests of the REST routes, driven in-process with
//! `tower::ServiceExt::oneshot` (no sockets, no HTTP client crate).

use std::io::{Cursor, Read};
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use forge_server::{app, Config};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn send(req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    send_with(Config::default(), req).await
}

async fn send_with(cfg: Config, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = app(cfg).oneshot(req).await.expect("infallible");
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, headers, bytes)
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

fn post_json(uri: &str, v: &Value) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(v.to_string()))
        .unwrap()
}

fn as_json(b: &[u8]) -> Value {
    serde_json::from_slice(b).unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(b)))
}

#[tokio::test]
async fn health_reports_modules() {
    let (s, _, b) = send(get("/api/health")).await;
    assert_eq!(s, StatusCode::OK);
    let v = as_json(&b);
    assert_eq!(v["status"], "ok");
    assert!(v["modules"].as_array().unwrap().iter().any(|m| m == "circuit"));
}

#[tokio::test]
async fn methods_lists_modules() {
    let (s, _, b) = send(get("/api/methods")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(as_json(&b)["ok"].as_array().unwrap().len() >= 10);
}

#[tokio::test]
async fn evaluates_iris_over_http() {
    let (_, _, b) = send(get("/api/project.iris")).await;
    let project = as_json(&b)["ok"].clone();
    assert_eq!(project["meta"]["id"], "iris");
    let (s, _, b) = send(post_json("/api/systems.evaluate", &project)).await;
    assert_eq!(s, StatusCode::OK);
    let ev = as_json(&b)["ok"].clone();
    assert_eq!(
        ev["results"].as_array().unwrap().len(),
        project["requirements"].as_array().unwrap().len()
    );
    assert_eq!(ev["fail"], 0);
}

#[tokio::test]
async fn method_call_with_input() {
    let (s, _, b) = send(post_json("/api/pcb.ipc2221", &json!({}))).await;
    // Either a result or an input error — never a server error.
    assert!(s == StatusCode::OK || s == StatusCode::UNPROCESSABLE_ENTITY, "{s}");
    let v = as_json(&b);
    assert!(v.get("ok").is_some() || v.get("error").is_some());
}

#[tokio::test]
async fn unknown_method_is_404() {
    let (s, _, b) = send(post_json("/api/nope.method", &json!({}))).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert!(as_json(&b)["error"].as_str().unwrap().contains("unknown module"));
}

#[tokio::test]
async fn bad_input_is_422_and_bad_json_is_400() {
    let (s, _, b) = send(post_json("/api/systems.evaluate", &json!({"not": "a project"}))).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(as_json(&b)["error"].is_string());
    let req = Request::post("/api/systems.evaluate").body(Body::from("{oops")).unwrap();
    let (s, _, b) = send(req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(as_json(&b)["error"].as_str().unwrap().contains("invalid JSON"));
}

#[tokio::test]
async fn unknown_api_route_is_json_404() {
    let (s, _, b) = send(get("/api/a/b/c")).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert!(as_json(&b)["error"].is_string());
}

#[tokio::test]
async fn package_returns_a_valid_zip() {
    let (s, h, b) = send(Request::post("/api/package").body(Body::empty()).unwrap()).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(h[header::CONTENT_TYPE], "application/zip");
    assert!(h[header::CONTENT_DISPOSITION].to_str().unwrap().contains("iris-fab-package.zip"));
    let mut zip = zip::ZipArchive::new(Cursor::new(b)).expect("valid zip");
    let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
    assert!(names.iter().any(|n| n == "README.md"), "{names:?}");
    assert!(names.iter().any(|n| n.starts_with("pcb/")), "{names:?}");
    assert!(names.iter().any(|n| n == "project.json"), "{names:?}");
    // Every entry decompresses; binary (base64) entries were decoded, e.g. STL.
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        if f.name().ends_with(".stl") {
            assert!(!buf.starts_with(b"solid") || buf.len() > 84, "stl {}", f.name());
        }
    }
    let mut readme = String::new();
    zip.by_name("README.md").unwrap().read_to_string(&mut readme).unwrap();
    assert!(readme.contains("fabrication package"));
}

#[tokio::test]
async fn package_rejects_non_projects() {
    let (s, _, _) = send(post_json("/api/package", &json!({"meta": 3}))).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn ngspice_missing_is_501() {
    let cfg = Config {
        ngspice_bin: "definitely-not-ngspice-4313".into(),
        ngspice_timeout: Duration::from_secs(5),
        ..Config::default()
    };
    let (s, _, b) = send_with(
        cfg,
        post_json("/api/ngspice", &json!({"netlist": "* t\nV1 1 0 1\nR1 1 0 1k\n.op\n"})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_IMPLEMENTED);
    assert!(as_json(&b)["error"].as_str().unwrap().contains("apt install ngspice"));
}

#[tokio::test]
async fn ngspice_empty_netlist_is_400() {
    let req = Request::post("/api/ngspice").body(Body::from("   ")).unwrap();
    let (s, _, _) = send(req).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn body_limit_is_enforced() {
    let cfg = Config {
        body_limit: 1024,
        ..Config::default()
    };
    let big = format!("{{\"x\":\"{}\"}}", "a".repeat(4096));
    let req = Request::post("/api/systems.evaluate").body(Body::from(big)).unwrap();
    let (s, _, _) = send_with(cfg, req).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn serves_static_with_spa_fallback() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<!doctype html><title>pf</title>").unwrap();
    std::fs::write(dir.path().join("app.js"), "console.log(1)").unwrap();
    let cfg = Config {
        static_dir: Some(dir.path().to_path_buf()),
        ..Config::default()
    };
    let (s, _, b) = send_with(cfg.clone(), get("/app.js")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b, b"console.log(1)");
    let (s, _, b) = send_with(cfg.clone(), get("/some/deep/link")).await;
    assert_eq!(s, StatusCode::OK);
    assert!(String::from_utf8_lossy(&b).contains("<title>pf</title>"));
    // API routes still win over the static fallback.
    let (s, _, _) = send_with(cfg, get("/api/health")).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn cors_preflight_is_allowed() {
    let req = Request::options("/api/systems.evaluate")
        .header(header::ORIGIN, "http://localhost:5173")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .unwrap();
    let (s, h, _) = send(req).await;
    assert!(s.is_success(), "{s}");
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
}
