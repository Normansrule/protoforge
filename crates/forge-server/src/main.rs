//! `forge-server` — run the Protoforge engine as a local REST server.
//!
//! ```text
//! cargo run -p forge-server -- --port 8787 --static web/dist
//! ```

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use forge_server::{app, find_on_path, Config};

const HELP: &str = "forge-server — Protoforge engine over HTTP

USAGE:
  forge-server [--host 127.0.0.1] [--port 8787] [--static web/dist]
               [--body-limit-mb 32] [--ngspice-timeout 30] [--ngspice-bin ngspice] [--no-cors]

ROUTES:
  GET  /api/health          liveness, version, engine modules
  GET  /api/methods         engine modules
  POST /api/<method>        JSON in -> {\"ok\": ...} | {\"error\": ...}   e.g. /api/systems.evaluate
  GET  /api/<method>        same with a null input                    e.g. /api/project.iris
  POST /api/package         project JSON (empty = Iris) -> fabrication package .zip
  POST /api/ngspice         SPICE netlist -> local ngspice batch run (501 if not installed)
  GET  /*                   files from --static with single-page-app fallback";

fn parse_args() -> Result<(SocketAddr, Config), String> {
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 8787;
    let mut cfg = Config::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "-h" | "--help" => return Err(String::new()),
            "--host" => host = val("--host")?,
            "-p" | "--port" => port = val("--port")?.parse().map_err(|e| format!("--port: {e}"))?,
            "--static" => cfg.static_dir = Some(PathBuf::from(val("--static")?)),
            "--body-limit-mb" => {
                let mb: usize = val("--body-limit-mb")?.parse().map_err(|e| format!("--body-limit-mb: {e}"))?;
                cfg.body_limit = mb * 1024 * 1024;
            }
            "--ngspice-timeout" => {
                let s: u64 = val("--ngspice-timeout")?.parse().map_err(|e| format!("--ngspice-timeout: {e}"))?;
                cfg.ngspice_timeout = Duration::from_secs(s);
            }
            "--ngspice-bin" => cfg.ngspice_bin = val("--ngspice-bin")?,
            "--no-cors" => cfg.cors = false,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if let Some(dir) = &cfg.static_dir {
        if !dir.join("index.html").is_file() {
            return Err(format!(
                "{} has no index.html — build the web app first (./scripts/build-wasm.sh && cd web && npm run build)",
                dir.display()
            ));
        }
    }
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| format!("bad address {host}:{port}: {e}"))?;
    Ok((addr, cfg))
}

#[tokio::main]
async fn main() -> ExitCode {
    let (addr, cfg) = match parse_args() {
        Ok(x) => x,
        Err(e) if e.is_empty() => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("error: {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: cannot listen on {addr}: {e}");
            return ExitCode::from(1);
        }
    };
    println!("protoforge server v{} listening on http://{addr}", env!("CARGO_PKG_VERSION"));
    match &cfg.static_dir {
        Some(d) => println!("  web app:  http://{addr}/  (from {})", d.display()),
        None => println!("  web app:  not served (add --static web/dist)"),
    }
    println!("  health:   curl http://{addr}/api/health");
    println!(
        "  ngspice:  {}",
        match find_on_path(&cfg.ngspice_bin) {
            Some(p) => format!("found at {}", p.display()),
            None => "not installed (POST /api/ngspice returns 501)".into(),
        }
    );
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        println!("\nshutting down");
    };
    if let Err(e) = axum::serve(listener, app(cfg)).with_graceful_shutdown(shutdown).await {
        eprintln!("server error: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
