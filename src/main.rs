use std::env;
use ft_ality::parse::parse_gmr_file;
use ft_ality::input::{RawMode, read_key_token};

fn main() {
    let path = env::args().nth(1).expect("usage: ft_ality <file.gmr> [-d|--debug]");
    let debug: bool = env::args().any(|arg| arg == "--debug" || arg == "-d");
    let grammar = parse_gmr_file(&path).expect("failed to parse file");

    if debug {
        println!("Key mappings (inferred from Σ):");
        for (i, tok) in grammar.alphabet.iter().enumerate() {
            println!("{} -> {}", i, tok.as_str());
        }
        println!("----------------------\n");

        for rule in &grammar.rules {
            let seq = rule.sequence.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ");
            println!("{seq}  =>  {}", rule.move_name);
        }
        println!("----------------------\n");
        println!("(debug) Press keys — Ctrl-C to quit.\n");
    } else {
        println!("Press keys — Ctrl-C to quit.\n");
    }

    let _raw = RawMode::new().expect("failed to switch TTY to raw mode");

    loop {
        match read_key_token() {
            Ok(Some(tok)) => {
                /* Print it as we are receiving it :) */
                println!("{}", tok);
                if tok == "ctrl-c" { break; }
            }
            Ok(None) => continue,
            Err(e) => {
                eprintln!("input error: {e}");
                break;
            }
        }
    }
}
