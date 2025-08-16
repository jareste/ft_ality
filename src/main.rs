use std::env;
use std::collections::HashMap;
use ft_ality::parse::{parse_gmr_file, classify};
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
    }

    let compiled = classify(&grammar);

    if debug {
        println!("== Bindings (keyboard → internal) ==");
        for b in &compiled.bindings {
            println!("{} -> {}", b.key, b.internal);
        }
        println!("\n== Combos (internal seq → move) ==");
        for r in &compiled.combos {
            let seq = r.sequence.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ");
            println!("{seq}  =>  {}", r.move_name);
        }
        println!("----------------------\n");
        println!("Press keys — Ctrl-C to quit.\n");
    }

    let key_to_internal: HashMap<String, String> = compiled
        .bindings
        .iter()
        .map(|b| (b.key.clone(), b.internal.clone()))
        .collect();

    let _raw = RawMode::new().expect("failed to switch TTY to raw mode");

    loop {
        match read_key_token() {
            Ok(Some(keytok)) => {
                if let Some(internal) = key_to_internal.get(&keytok) {
                    println!("{}  ⇒  {}", keytok, internal);

                } else {
                    /* Same as 'is_internal' */
                    if keytok.starts_with('[') && keytok.ends_with(']') {
                        println!("(direct) {}", keytok);
                    } else {
                        println!("{}", keytok);
                    }
                }

                if keytok == "ctrl-c" { break; }
            }
            Ok(None) => continue,
            Err(e) => {
                eprintln!("input error: {e}");
                break;
            }
        }
    }
}
