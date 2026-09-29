//! `protoforge` — command-line front end to the forge-core engine.
//!
//! ```text
//! protoforge evaluate [project.json]            verify every requirement
//! protoforge metrics  [project.json]            list every computed metric
//! protoforge report   [project.json]            Markdown verification report
//! protoforge package  [project.json] [--out DIR]  write the full fabrication package
//! protoforge lint     [project.json]            structural checks
//! protoforge call <method> [json | @file.json]   call any engine method
//! protoforge methods                            list engine modules
//! protoforge new "<name>" > my-product.json     blank project template
//! ```
//!
//! With no project path the embedded Iris smart-glasses example is used.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use forge_core::project::Project;
use forge_core::systems::{self, Status};

fn usage() -> ExitCode {
    eprintln!(
        "protoforge {} — idea to atoms\n\n\
         USAGE:\n  protoforge evaluate [project.json]\n  protoforge metrics  [project.json]\n  protoforge report   [project.json]\n  \
         protoforge package  [project.json] [--out DIR]\n  protoforge lint     [project.json]\n  \
         protoforge call <method> [json | @file.json]\n  protoforge methods\n  protoforge new \"<name>\"\n\n\
         Without a project path the built-in Iris smart-glasses project is used.",
        env!("CARGO_PKG_VERSION")
    );
    ExitCode::from(2)
}

fn load(path: Option<&String>) -> Result<Project, String> {
    match path {
        None => Ok(Project::iris()),
        Some(p) => {
            let text = std::fs::read_to_string(p).map_err(|e| format!("cannot read {p}: {e}"))?;
            serde_json::from_str(&text).map_err(|e| format!("{p} is not a valid project: {e}"))
        }
    }
}

fn main() -> ExitCode {
    // `protoforge package | head` closes stdout early: exit quietly instead of panicking
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let p = info.payload();
        let msg = p
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| p.downcast_ref::<&str>().copied())
            .unwrap_or("");
        if msg.contains("Broken pipe") {
            std::process::exit(0);
        }
        default_hook(info);
    }));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else { return usage() };
    let rest = &args[1..];
    let result = match cmd.as_str() {
        "evaluate" | "eval" => evaluate(rest),
        "metrics" => metrics(rest),
        "report" => report(rest),
        "package" | "pkg" => package(rest),
        "lint" => lint(rest),
        "call" => call(rest),
        "methods" => {
            println!("{}", forge_core::api::call("api.methods", ""));
            Ok(true)
        }
        "new" => {
            let name = rest.first().map(String::as_str).unwrap_or("Untitled product");
            println!("{}", serde_json::to_string_pretty(&Project::blank(name)).unwrap());
            Ok(true)
        }
        "-h" | "--help" | "help" => return usage(),
        other => Err(format!("unknown command `{other}`")),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn positional(rest: &[String]) -> Option<&String> {
    let mut skip = false;
    for a in rest {
        if skip {
            skip = false;
            continue;
        }
        if a == "--out" {
            skip = true;
            continue;
        }
        if !a.starts_with("--") {
            return Some(a);
        }
    }
    None
}

fn evaluate(rest: &[String]) -> Result<bool, String> {
    let p = load(positional(rest))?;
    let t0 = Instant::now();
    let ev = systems::evaluate(&p);
    let ms = t0.elapsed().as_secs_f64() * 1e3;
    println!(
        "{} v{} — {} requirements ({} ms)\n",
        p.meta.name,
        p.meta.version,
        ev.results.len(),
        ms.round()
    );
    for (r, res) in p.requirements.iter().zip(&ev.results) {
        let mark = match res.status {
            Status::Pass => "PASS",
            Status::Fail if r.kind == forge_core::project::ReqKind::Goal => "GOAL",
            Status::Fail => "FAIL",
            Status::Unverified => " -- ",
        };
        let val = res.value.map(|v| format!("{v:.3} {}", res.unit)).unwrap_or_else(|| "—".into());
        println!(
            "[{mark}] {:<6} {:<42} {:>16}   need {}",
            r.id,
            truncate(&r.text, 42),
            val,
            res.criterion
        );
    }
    // "goal" requirements are stretch targets ("should"): an open goal is
    // reported but does not fail the build; a binding requirement does.
    let failed_goals = p
        .requirements
        .iter()
        .zip(&ev.results)
        .filter(|(r, res)| res.status == Status::Fail && r.kind == forge_core::project::ReqKind::Goal)
        .count();
    let binding_fails = ev.fail - failed_goals;
    println!(
        "\n{} pass · {} fail ({} binding, {} open goal{}) · {} unverified",
        ev.pass,
        ev.fail,
        binding_fails,
        failed_goals,
        if failed_goals == 1 { "" } else { "s" },
        ev.unverified
    );
    Ok(binding_fails == 0)
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n - 1).collect::<String>())
    }
}

fn metrics(rest: &[String]) -> Result<bool, String> {
    let p = load(positional(rest))?;
    for m in systems::all_metrics(&p) {
        println!("{:<30} {:>14.4} {:<8} {}", m.key, m.value, m.unit, m.note);
    }
    Ok(true)
}

fn report(rest: &[String]) -> Result<bool, String> {
    let p = load(positional(rest))?;
    let ev = systems::evaluate(&p);
    print!("{}", systems::report_markdown(&p, &ev));
    Ok(true)
}

fn lint(rest: &[String]) -> Result<bool, String> {
    let p = load(positional(rest))?;
    let issues = systems::lint(&p);
    for i in &issues {
        println!("{:<8} {:<8} {}", i.severity, i.id, i.message);
    }
    println!("{} issue(s)", issues.len());
    Ok(!issues.iter().any(|i| i.severity == "error"))
}

fn package(rest: &[String]) -> Result<bool, String> {
    let p = load(positional(rest))?;
    let out = rest
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| rest.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("out/{}", p.meta.id)));
    let t0 = Instant::now();
    let files = forge_core::mfg::package(&p);
    let mut bytes = 0usize;
    for f in &files {
        let path = out.join(&f.path);
        write_file(&path, &f.content, &f.encoding)?;
        bytes += f.content.len();
    }
    println!(
        "wrote {} files ({} kB) to {} in {:.1} s",
        files.len(),
        bytes / 1024,
        out.display(),
        t0.elapsed().as_secs_f64()
    );
    for f in &files {
        println!("  {}", f.path);
    }
    Ok(true)
}

fn write_file(path: &Path, content: &str, encoding: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    }
    let data = if encoding == "base64" {
        decode_base64(content)?
    } else {
        content.as_bytes().to_vec()
    };
    std::fs::write(path, data).map_err(|e| format!("write {}: {e}", path.display()))
}

fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let clean: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut acc = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            acc |= val(c).ok_or("invalid base64")? << (18 - 6 * i);
        }
        out.push((acc >> 16) as u8);
        if chunk.len() > 2 {
            out.push((acc >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(acc as u8);
        }
    }
    Ok(out)
}

fn call(rest: &[String]) -> Result<bool, String> {
    let method = rest.first().ok_or("usage: protoforge call <method> [json | @file.json]")?;
    let input = match rest.get(1) {
        None => String::new(),
        Some(a) if a.starts_with('@') => std::fs::read_to_string(&a[1..]).map_err(|e| format!("cannot read {}: {e}", &a[1..]))?,
        Some(a) => a.clone(),
    };
    let out = forge_core::api::call(method, &input);
    println!("{out}");
    Ok(!out.starts_with("{\"error\""))
}
