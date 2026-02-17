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
    let mut command = None;
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
                command = Some(args[i].clone());
            }
        }
        i += 1;
    }

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
    eprintln!("  scene.stats        Show scene statistics");
    eprintln!();
    eprintln!("If no command is given, starts an interactive REPL.");
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

        match client.send(cmd, None) {
            Ok(response) => print_response(cmd, &response, json_mode),
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
            match client.send(&cmd, None) {
                Ok(response) => {
                    let has_error = response.error.is_some();
                    print_response(&cmd, &response, args.json);
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
