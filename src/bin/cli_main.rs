use std::env;
use ft_ality::apps::cli::run_cli;

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args.next().expect("usage: cli <file.gmr> [--debug] [--timeout-ms N]");

    let mut debug = false;
    let mut timeout_ms: u64 = 500;

    for a in args {
        if a == "--debug" || a == "-d" { debug = true; }
        else if a == "--timeout-ms" {
        } else if let Some(ms) = a.strip_prefix("--timeout-ms=") {
            timeout_ms = ms.parse().expect("invalid --timeout-ms value");
        }
    }

    run_cli(&path, debug, timeout_ms)
}
