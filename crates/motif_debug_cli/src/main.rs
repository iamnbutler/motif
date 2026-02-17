//! motif-debug: CLI tool for inspecting a running motif process.
//!
//! Usage:
//!   motif-debug 'scene.stats'          — single command mode
//!   motif-debug                         — REPL mode
//!   motif-debug --json 'scene.stats'   — raw JSON output
//!   motif-debug --socket /path/to/sock  — connect to specific socket

mod client;

use client::DebugClient;

struct Args {
    json: bool,
    socket: Option<String>,
    command: Option<String>,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut json = false;
    let mut socket = None;
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--json" => json = true,
            "--socket" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --socket requires a path argument");
                    std::process::exit(1);
                }
                socket = Some(args[i].clone());
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown flag '{arg}'");
                eprintln!("run 'motif-debug --help' for usage");
                std::process::exit(1);
            }
            _ => {
                positional.push(args[i].clone());
            }
        }
        i += 1;
    }

    // Join all positional args into a single command string so that
    // `motif-debug screenshot /tmp/test.png` works the same as
    // `motif-debug 'screenshot /tmp/test.png'`.
    let command = if positional.is_empty() {
        None
    } else {
        Some(positional.join(" "))
    };

    Args {
        json,
        socket,
        command,
    }
}

fn print_usage() {
    eprintln!("motif-debug - inspect a running motif process");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  motif-debug [OPTIONS] [COMMAND]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  --json             Output raw JSON (for scripting)");
    eprintln!("  --socket <path>    Connect to a specific socket path");
    eprintln!("  -h, --help         Show this help message");
    eprintln!();
    eprintln!("COMMANDS:");
    eprintln!("  scene.stats              Show scene statistics");
    eprintln!("  scene.quads              List all quads in the scene");
    eprintln!("  scene.text_runs          List all text runs in the scene");
    eprintln!("  screenshot <path.png>    Capture scene to a PNG file");
    eprintln!();
    eprintln!("If no command is given, starts an interactive REPL.");
}

/// Parse a command string into a method name and optional JSON params.
///
/// Handles commands like `screenshot /path/to/file.png` by splitting the
/// command into the method and constructing the appropriate params object.
fn parse_command(input: &str) -> (&str, Option<serde_json::Value>) {
    let trimmed = input.trim();
    if let Some(path) = trimmed.strip_prefix("screenshot ") {
        let path = path.trim();
        if path.is_empty() {
            ("screenshot", None)
        } else {
            ("screenshot", Some(serde_json::json!({ "path": path })))
        }
    } else {
        (trimmed, None)
    }
}

fn format_screenshot(value: &serde_json::Value) -> String {
    let path = value
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let w = value
        .get("size")
        .and_then(|v| v.get(0))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let h = value
        .get("size")
        .and_then(|v| v.get(1))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    format!("Screenshot saved to {path} ({w}x{h})\n")
}

fn connect(socket: Option<&str>) -> DebugClient {
    let result = match socket {
        Some(path) => DebugClient::connect(path),
        None => DebugClient::discover(),
    };

    match result {
        Ok(client) => client,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

fn format_scene_stats(value: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str("Scene Stats\n");
    out.push_str("───────────────────────\n");

    if let Some(qc) = value.get("quad_count") {
        out.push_str(&format!("  Quads:         {qc}\n"));
    }
    if let Some(tc) = value.get("text_run_count") {
        out.push_str(&format!("  Text runs:     {tc}\n"));
    }
    if let Some(vp) = value.get("viewport_size") {
        if let (Some(w), Some(h)) = (vp.get(0), vp.get(1)) {
            out.push_str(&format!("  Viewport:      {w} x {h}\n"));
        }
    }
    if let Some(sf) = value.get("scale_factor") {
        out.push_str(&format!("  Scale factor:  {sf}\n"));
    }

    out
}

fn format_scene_quads(value: &serde_json::Value) -> String {
    let mut out = String::new();
    let arr = match value.as_array() {
        Some(a) => a,
        None => return "No quad data.\n".to_string(),
    };

    if arr.is_empty() {
        return "No quads in scene.\n".to_string();
    }

    out.push_str("Scene Quads\n");
    out.push_str("───────────────────────────────────────────────────────────────\n");
    out.push_str(&format!(
        "  {:<5}  {:<20}  {:<14}  {:}\n",
        "IDX", "POSITION", "SIZE", "COLOR"
    ));
    out.push_str("  ─────  ────────────────────  ──────────────  ───────────────\n");

    for (i, q) in arr.iter().enumerate() {
        let x = q["bounds"]["x"].as_f64().unwrap_or(0.0);
        let y = q["bounds"]["y"].as_f64().unwrap_or(0.0);
        let w = q["bounds"]["w"].as_f64().unwrap_or(0.0);
        let h = q["bounds"]["h"].as_f64().unwrap_or(0.0);
        let r = q["color"]["r"].as_f64().unwrap_or(0.0);
        let g = q["color"]["g"].as_f64().unwrap_or(0.0);
        let b = q["color"]["b"].as_f64().unwrap_or(0.0);
        let a = q["color"]["a"].as_f64().unwrap_or(0.0);

        out.push_str(&format!(
            "  {:<5}  ({:>7.1}, {:>7.1})    {:>5.0} x {:<5.0}  rgba({:.2},{:.2},{:.2},{:.2})\n",
            i, x, y, w, h, r, g, b, a
        ));
    }

    out.push_str(&format!("\n  Total: {} quads\n", arr.len()));
    out
}

fn format_scene_text_runs(value: &serde_json::Value) -> String {
    let mut out = String::new();
    let arr = match value.as_array() {
        Some(a) => a,
        None => return "No text run data.\n".to_string(),
    };

    if arr.is_empty() {
        return "No text runs in scene.\n".to_string();
    }

    out.push_str("Scene Text Runs\n");
    out.push_str("───────────────────────────────────────────────────────────────\n");
    out.push_str(&format!(
        "  {:<5}  {:<20}  {:<10}  {:}\n",
        "IDX", "ORIGIN", "FONT SIZE", "GLYPHS"
    ));
    out.push_str("  ─────  ────────────────────  ──────────  ──────\n");

    for (i, tr) in arr.iter().enumerate() {
        let x = tr["origin"]["x"].as_f64().unwrap_or(0.0);
        let y = tr["origin"]["y"].as_f64().unwrap_or(0.0);
        let fs = tr["font_size"].as_f64().unwrap_or(0.0);
        let gc = tr["glyph_count"].as_u64().unwrap_or(0);

        out.push_str(&format!(
            "  {:<5}  ({:>7.1}, {:>7.1})    {:>7.1}px  {:>6}\n",
            i, x, y, fs, gc
        ));
    }

    out.push_str(&format!("\n  Total: {} text runs\n", arr.len()));
    out
}

fn print_response(method: &str, response: &motif_debug::DebugResponse, json_mode: bool) {
    if let Some(err) = &response.error {
        if json_mode {
            let json = serde_json::to_string(&response).unwrap_or_default();
            println!("{json}");
        } else {
            eprintln!("error [{}]: {}", err.code, err.message);
        }
        return;
    }

    let result = match &response.result {
        Some(v) => v,
        None => {
            if json_mode {
                println!("null");
            }
            return;
        }
    };

    if json_mode {
        let pretty = serde_json::to_string_pretty(result).unwrap_or_default();
        println!("{pretty}");
        return;
    }

    // Pretty-print known result types.
    match method {
        "scene.stats" => print!("{}", format_scene_stats(result)),
        "scene.quads" => print!("{}", format_scene_quads(result)),
        "scene.text_runs" => print!("{}", format_scene_text_runs(result)),
        "screenshot" => print!("{}", format_screenshot(result)),
        _ => {
            let pretty = serde_json::to_string_pretty(result).unwrap_or_default();
            println!("{pretty}");
        }
    }
}

fn run_repl(mut client: DebugClient, json_mode: bool) {
    let stdin = std::io::stdin();
    let mut line = String::new();

    loop {
        // Print prompt to stderr so it doesn't interfere with piped output.
        eprint!("motif> ");

        line.clear();
        match stdin.read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl+D)
                eprintln!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("error reading input: {e}");
                break;
            }
        }

        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }
        if cmd == "quit" || cmd == "exit" {
            break;
        }

        let (method, params) = parse_command(cmd);
        match client.send(method, params) {
            Ok(response) => print_response(method, &response, json_mode),
            Err(e) => {
                eprintln!("error: {e}");
                break;
            }
        }
    }
}

fn main() {
    let args = parse_args();
    let mut client = connect(args.socket.as_deref());

    match args.command {
        Some(cmd) => {
            // Single command mode.
            let (method, params) = parse_command(&cmd);
            match client.send(method, params) {
                Ok(response) => {
                    let has_error = response.error.is_some();
                    print_response(method, &response, args.json);
                    if has_error {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
        }
        None => {
            // REPL mode.
            run_repl(client, args.json);
        }
    }
}
