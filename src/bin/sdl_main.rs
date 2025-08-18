#![cfg(feature = "sdl")]

use std::env;
use ft_ality::apps::sdl::run_sdl;

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args.next().expect("usage: sdl <file.gmr> [--debug] [--timeout-ms=N] [--font=/path/font.ttf]");

    let mut debug = false;
    let mut timeout_ms: u64 = 500;
    let mut font_path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string();

    for a in args {
        if a == "--debug" || a == "-d" { debug = true; }
        else if let Some(ms) = a.strip_prefix("--timeout-ms=") {
            timeout_ms = ms.parse().expect("invalid --timeout-ms value");
        } else if let Some(fp) = a.strip_prefix("--font=") {
            font_path = fp.to_string();
        }
    }

    run_sdl(&path, debug, timeout_ms, &font_path)
}
