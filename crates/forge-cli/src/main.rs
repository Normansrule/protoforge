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
//! protoforge new "<name>" --kind wearable --with ble,display,battery --life 12h
//!                                               starter project from the new-product wizard
//! protoforge wizard [@answers.json | json]      wizard answers → project.json (notes on stderr)
//! protoforge wizard --options                   kinds, capabilities and targets
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
         protoforge call <method> [json | @file.json]\n  protoforge methods\n  protoforge new \"<name>\"\n  \
         protoforge new \"<name>\" --kind <kind> [--with cap,cap] [--idea TEXT] [--users TEXT]\n                 \
         [--life 12h] [--mass 50g] [--cost 20] [--qty 1000] [--retail 99] [--size 45x38x14]\n                 \
         [--range 10m] [--set target=value]\n  protoforge wizard [@answers.json | json]\n  protoforge wizard --options\n\n\
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
        "new" => new(rest),
        "wizard" => wizard(rest),
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

// ---------------------------------------------------------------------------
// New-product wizard
// ---------------------------------------------------------------------------

/// `protoforge new "<name>"` — blank project, or the wizard when `--kind` is given.
fn new(rest: &[String]) -> Result<bool, String> {
    let mut name: Option<String> = None;
    let mut a = forge_core::systems::wizard::Answers::default();
    let mut wizard = false;
    let mut i = 0;
    while i < rest.len() {
        let arg = &rest[i];
        if let Some(flag) = arg.strip_prefix("--") {
            let val = rest.get(i + 1).ok_or_else(|| format!("--{flag} needs a value"))?.clone();
            i += 2;
            wizard = true;
            let mut target = |k: &str, v: f64| {
                a.targets.insert(k.to_string(), v);
            };
            match flag {
                "kind" => a.kind = val,
                "with" | "caps" => a.capabilities = Some(val.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()),
                "idea" => a.idea = val,
                "users" => a.users = val,
                "author" => a.author = val,
                "life" => target("life_h", quantity(&val, Unit::Hours)?),
                "mass" => target("mass_g", quantity(&val, Unit::Grams)?),
                "cost" => target("cost_usd", quantity(&val, Unit::Plain)?),
                "qty" | "quantity" => target("quantity", quantity(&val, Unit::Plain)?),
                "retail" | "price" => target("retail_usd", quantity(&val, Unit::Plain)?),
                "range" => target("range_m", quantity(&val, Unit::Metres)?),
                "size" => {
                    let dims: Vec<f64> = val
                        .split(['x', 'X', '×'])
                        .map(|d| quantity(d, Unit::Millimetres))
                        .collect::<Result<_, _>>()?;
                    if dims.len() != 3 {
                        return Err(format!("--size wants L×W×H, e.g. 45x38x14 (got `{val}`)"));
                    }
                    target("length_mm", dims[0]);
                    target("width_mm", dims[1]);
                    target("height_mm", dims[2]);
                }
                "set" => {
                    let (k, v) = val.split_once('=').ok_or_else(|| format!("--set wants key=value (got `{val}`)"))?;
                    target(k.trim(), quantity(v, Unit::Plain)?);
                }
                other => return Err(format!("unknown option --{other} for `new`")),
            }
        } else {
            if name.is_none() {
                name = Some(arg.clone());
            }
            i += 1;
        }
    }
    let name = name.unwrap_or_else(|| "Untitled product".into());
    if !wizard {
        println!("{}", serde_json::to_string_pretty(&Project::blank(&name)).unwrap());
        return Ok(true);
    }
    if a.kind.is_empty() {
        return Err("the wizard needs --kind (run `protoforge wizard --options` for the list)".into());
    }
    a.name = name;
    run_wizard(&a)
}

/// `protoforge wizard [@answers.json | json]` or `--options`.
fn wizard(rest: &[String]) -> Result<bool, String> {
    if rest.first().map(String::as_str) == Some("--options") {
        let o = forge_core::systems::wizard::catalog::options();
        println!("{}", serde_json::to_string_pretty(&o).unwrap());
        return Ok(true);
    }
    let text = match rest.first() {
        None => return Err("usage: protoforge wizard [@answers.json | json] (or --options)".into()),
        Some(a) if a.starts_with('@') => std::fs::read_to_string(&a[1..]).map_err(|e| format!("cannot read {}: {e}", &a[1..]))?,
        Some(a) => a.clone(),
    };
    let a: forge_core::systems::wizard::Answers = serde_json::from_str(&text).map_err(|e| format!("answers are not valid JSON: {e}"))?;
    run_wizard(&a)
}

fn run_wizard(a: &forge_core::systems::wizard::Answers) -> Result<bool, String> {
    let out = forge_core::systems::wizard::generate(a)?;
    println!("{}", serde_json::to_string_pretty(&out.project).unwrap());
    let s = &out.summary;
    eprintln!(
        "{} — {} requirements ({} bound to metrics, {} by test/inspection), {} g, {} mW average, {} USD parts at volume",
        out.project.meta.name,
        out.project.requirements.len(),
        s.bound_requirements,
        s.test_requirements,
        s.mass_g.round(),
        (s.avg_power_mw * 100.0).round() / 100.0,
        (s.bom_volume_usd * 100.0).round() / 100.0
    );
    eprintln!("capabilities: {}", out.capabilities.join(", "));
    for e in &out.explanations {
        eprintln!("  • {}: {}", e.title, e.result);
    }
    for w in &out.warnings {
        eprintln!("  [{}] {}", w.level, w.text);
    }
    Ok(true)
}

#[derive(Clone, Copy)]
enum Unit {
    Plain,
    Hours,
    Grams,
    Metres,
    Millimetres,
}

/// Parse "12h", "2d", "1y", "30min", "50g", "1.2kg", "10m", "45mm", "4.5cm" or a plain number.
fn quantity(s: &str, unit: Unit) -> Result<f64, String> {
    let t = s.trim().to_lowercase().replace([',', '$', '_'], "");
    let split = t
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .unwrap_or(t.len());
    let (num, suffix) = t.split_at(split);
    let x: f64 = num.parse().map_err(|_| format!("`{s}` is not a number"))?;
    let suffix = suffix.trim().trim_start_matches(' ');
    let k = match (unit, suffix) {
        (_, "") => 1.0,
        (Unit::Hours, "h" | "hr" | "hrs" | "hour" | "hours") => 1.0,
        (Unit::Hours, "min" | "m" | "minutes") => 1.0 / 60.0,
        (Unit::Hours, "d" | "day" | "days") => 24.0,
        (Unit::Hours, "w" | "wk" | "week" | "weeks") => 168.0,
        (Unit::Hours, "y" | "yr" | "year" | "years") => 8760.0,
        (Unit::Grams, "g") => 1.0,
        (Unit::Grams, "kg") => 1000.0,
        (Unit::Metres, "m") => 1.0,
        (Unit::Metres, "km") => 1000.0,
        (Unit::Metres, "cm") => 0.01,
        (Unit::Millimetres, "mm") => 1.0,
        (Unit::Millimetres, "cm") => 10.0,
        (Unit::Millimetres, "m") => 1000.0,
        (Unit::Plain, "k") => 1000.0,
        (Unit::Plain, "usd") => 1.0,
        _ => return Err(format!("unknown unit `{suffix}` in `{s}`")),
    };
    Ok(x * k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quantities() {
        assert_eq!(quantity("12h", Unit::Hours).unwrap(), 12.0);
        assert_eq!(quantity("2d", Unit::Hours).unwrap(), 48.0);
        assert_eq!(quantity("1y", Unit::Hours).unwrap(), 8760.0);
        assert_eq!(quantity("30min", Unit::Hours).unwrap(), 0.5);
        assert_eq!(quantity("1.5kg", Unit::Grams).unwrap(), 1500.0);
        assert_eq!(quantity("4.5cm", Unit::Millimetres).unwrap(), 45.0);
        assert_eq!(quantity("1,000", Unit::Plain).unwrap(), 1000.0);
        assert_eq!(quantity("$25", Unit::Plain).unwrap(), 25.0);
        assert_eq!(quantity("5k", Unit::Plain).unwrap(), 5000.0);
        assert!(quantity("12 parsecs", Unit::Hours).is_err());
    }
}
