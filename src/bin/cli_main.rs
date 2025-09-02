use std::env;
use ft_ality::apps::cli::run_cli;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();

    let path = args.first()
        .ok_or("usage: cli <file.gmr> [--debug] [--timeout-ms N]")?
        .clone();

    let (debug, timeout_ms) = args.iter().skip(1).fold((false, 500), |(debug, timeout_ms), arg| {
        if arg == "--debug" || arg == "-d" {
            (true, timeout_ms)
        } else if let Some(ms) = arg.strip_prefix("--timeout-ms=") {
            let parsed_ms = ms.parse().expect("invalid --timeout-ms value");
            (debug, parsed_ms)
        } else {
            (debug, timeout_ms)
        }
    });

    run_cli(&path, debug, timeout_ms)
}
