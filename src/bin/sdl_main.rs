#![cfg(feature = "sdl")]

use std::env;
use ft_ality::apps::sdl::run_sdl;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let path = args.first()
        .ok_or("usage: cli <file.gmr> [--debug] [--timeout-ms N]")?
        .clone();

    let (debug, timeout_ms, font_path) = args.iter().skip(1).fold(
        (false, 500, "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string()),
        |(debug, timeout_ms, font_path), arg| {
            if arg == "--debug" || arg == "-d" {
                (true, timeout_ms, font_path)
            } else if let Some(ms) = arg.strip_prefix("--timeout-ms=") {
                let parsed_ms = ms.parse().expect("invalid --timeout-ms value");
                (debug, parsed_ms, font_path)
            } else if let Some(fp) = arg.strip_prefix("--font=") {
                (debug, timeout_ms, fp.to_string())
            } else {
                (debug, timeout_ms, font_path)
            }
        },
    );

    run_sdl(&path, debug, timeout_ms, &font_path)
}
