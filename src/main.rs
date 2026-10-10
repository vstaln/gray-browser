//! browser — headless-Chromium tools: rendered fetch, screenshot, PDF.
//! Spawns chromium/chrome CLI; a per-call watchdog kills stalls.

use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use serde_json::{json, Value};

const NAME: &str = "browser";
const TIMEOUT: Duration = Duration::from_secs(45);

fn manifest() -> Value {
    json!({
        "name": NAME, "display_name": "Browser",
        "version": env!("CARGO_PKG_VERSION"), "protocol": "1.1",
        "tools": [
            {"name": "browser_fetch",
             "description": "Fetch a URL in a real headless browser and return the rendered DOM text. Use when web_fetch fails on JS-heavy pages.",
             "parameters": {"type": "object", "properties": {
                "url": {"type": "string"},
                "raw": {"type": "boolean", "description": "Return raw HTML instead of stripped text."},
                "max_chars": {"type": "integer"}}, "required": ["url"]}},
            {"name": "browser_shot",
             "description": "Screenshot a URL with headless Chromium; returns the PNG path (cat it to view).",
             "parameters": {"type": "object", "properties": {
                "url": {"type": "string"},
                "width": {"type": "integer"}, "height": {"type": "integer"},
                "path": {"type": "string"}}, "required": ["url"]}},
            {"name": "browser_pdf",
             "description": "Print a URL to PDF via headless Chromium; returns the file path.",
             "parameters": {"type": "object", "properties": {
                "url": {"type": "string"}, "path": {"type": "string"}},
             "required": ["url"]}},
        ],
        "commands": ["/browser"], "subcommands": ["browser"],
    })
}

fn chrome() -> Option<String> {
    for b in ["chromium", "chromium-browser", "google-chrome", "google-chrome-stable",
              "chrome", "brave-browser"] {
        if Command::new(b).arg("--version").stdout(Stdio::null()).stderr(Stdio::null())
            .status().map(|s| s.success()).unwrap_or(false) {
            return Some(b.to_string());
        }
    }
    None
}

fn run_deadline(bin: &str, args: &[String]) -> Result<std::process::Output, String> {
    let mut child = Command::new(bin).args(args)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("spawn {bin}: {e}"))?;
    let start = Instant::now();
    loop {
        if let Some(_code) = child.try_wait().map_err(|e| e.to_string())? {
            return child.wait_with_output().map_err(|e| e.to_string());
        }
        if start.elapsed() > TIMEOUT {
            let _ = child.kill();
            return Err(format!("{bin} timed out after {}s", TIMEOUT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(80));
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut depth_skip = 0i32; // inside script/style
    let mut it = html.chars().peekable();
    let mut word = String::new();
    while let Some(c) = it.next() {
        if in_tag {
            if c == '>' { in_tag = false; 
                if word.starts_with("script") || word.starts_with("style") { depth_skip += 1; }
                if word.starts_with("/script") || word.starts_with("/style") { depth_skip -= 1; }
                word.clear();
            } else if word.len() < 16 { word.push(c); }
            continue;
        }
        if c == '<' { in_tag = true; word.clear();
            if it.peek() == Some(&'/') { word.push('/'); it.next(); }
            continue;
        }
        if depth_skip <= 0 { out.push(c); }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn browser_fetch(a: &Value) -> Result<String, String> {
    let url = a["url"].as_str().ok_or("url required")?;
    let max = a["max_chars"].as_u64().unwrap_or(20000) as usize;
    let dom = match chrome() {
        // Full path: headless chromium renders JS and dumps the DOM.
        Some(bin) => {
            let out = run_deadline(&bin, &[
                "--headless=new".into(), "--disable-gpu".into(), "--no-sandbox".into(),
                "--disable-dev-shm-usage".into(), "--dump-dom".into(), url.into()])?;
            let dom = String::from_utf8_lossy(&out.stdout).to_string();
            if dom.trim().is_empty() {
                return Err(format!("empty DOM: {}",
                    String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
            }
            dom
        }
        // No browser: plain GET still works for server-rendered pages.
        None => match ureq::get(url)
            .set("User-Agent", "gray-browser/0.1 (no-js fetch)")
            .timeout(std::time::Duration::from_secs(20)).call() {
            Ok(r) => r.into_string().map_err(|e| e.to_string())?,
            Err(ureq::Error::Status(c, _)) =>
                return Err(format!("HTTP {c}")),
            Err(e) => return Err(e.to_string()),
        },
    };
    let text = if a["raw"].as_bool().unwrap_or(false) { dom } else { strip_tags(&dom) };
    Ok(if text.len() > max { format!("{}…[{max}-char cap]", &text[..max]) } else { text })
}

fn browser_shot(a: &Value) -> Result<Value, String> {
    let url = a["url"].as_str().ok_or("url required")?;
    let bin = chrome().ok_or("no chromium/chrome binary found")?;
    let dir = state_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = a["path"].as_str().map(str::to_string).unwrap_or_else(||
        dir.join(format!("shot-{}.png", std::process::id())).display().to_string());
    let w = a["width"].as_u64().unwrap_or(1280);
    let h = a["height"].as_u64().unwrap_or(900);
    run_deadline(&bin, &[
        "--headless=new".into(), "--disable-gpu".into(), "--no-sandbox".into(),
        "--disable-dev-shm-usage".into(), "--hide-scrollbars".into(),
        format!("--window-size={w},{h}"), format!("--screenshot={path}"), url.into()])?;
    if !std::path::Path::new(&path).exists() {
        return Err("chromium produced no screenshot".into());
    }
    Ok(json!({"content": format!("screenshot → {path}"),
              "images": [{"path": path}]}))
}

fn browser_pdf(a: &Value) -> Result<String, String> {
    let url = a["url"].as_str().ok_or("url required")?;
    let bin = chrome().ok_or("no chromium/chrome binary found")?;
    let dir = state_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = a["path"].as_str().map(str::to_string).unwrap_or_else(||
        dir.join(format!("page-{}.pdf", std::process::id())).display().to_string());
    run_deadline(&bin, &[
        "--headless=new".into(), "--disable-gpu".into(), "--no-sandbox".into(),
        "--disable-dev-shm-usage".into(), format!("--print-to-pdf={path}"),
        "--no-pdf-header-footer".into(), url.into()])?;
    if std::path::Path::new(&path).exists() { Ok(format!("pdf → {path}")) }
    else { Err("chromium produced no pdf".into()) }
}

fn state_dir() -> std::path::PathBuf {
    let home = std::env::var_os("GRAY_HOME").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join("browser")
}

fn tool(name: &str, args: &Value) -> Value {
    match name {
        "browser_fetch" => match browser_fetch(args) {
            Ok(c) => json!({"result": {"content": c}}),
            Err(e) => json!({"result": {"content": e, "is_error": true}}),
        },
        "browser_shot" => match browser_shot(args) {
            Ok(v) => json!({"result": v}),
            Err(e) => json!({"result": {"content": e, "is_error": true}}),
        },
        "browser_pdf" => match browser_pdf(args) {
            Ok(c) => json!({"result": {"content": c}}),
            Err(e) => json!({"result": {"content": e, "is_error": true}}),
        },
        t => json!({"result": {"content": format!("unknown tool {t}"), "is_error": true}}),
    }
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let method = v["method"].as_str().unwrap_or("");
        match v.get("id").cloned() {
            None if method == "plugin/shutdown" => return Ok(()),
            None => {}
            Some(id) => {
                let reply = if method == "plugin/manifest" {
                    json!({"id": id, "result": manifest()})
                } else if method == "tool/call" {
                    let mut r = tool(v["params"]["name"].as_str().unwrap_or(""), &v["params"]["args"]);
                    r["id"] = id; r
                } else if method == "command/run" {
                    let s = chrome().unwrap_or_else(|| "NONE".into());
                    json!({"id": id, "result": {"text": format!("browser backend: {s}")}})
                } else {
                    json!({"id": id, "error": {"code": -32601,
                        "message": format!("method not found: {method}")}})
                };
                writeln!(out, "{reply}")?; out.flush()?;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = match args.first().map(String::as_str) {
        Some("browser") => args[1..].to_vec(), _ => args };
    match args.first().map(String::as_str) {
        Some("manifest") => println!("{}", manifest()),
        Some("sidecar") => { let _ = sidecar(); }
        _ => {
            if args.is_empty() {
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    println!("browser — headless chromium tools (fetch/shot/pdf). backend: {}",
                        chrome().unwrap_or_else(|| "NONE — install chromium".into()));
                    return;
                }
                let _ = sidecar(); return;
            }
            eprintln!("unknown verb"); std::process::exit(1);
        }
    }
}
