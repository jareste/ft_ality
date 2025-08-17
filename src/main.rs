use std::env;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use ft_ality::parse::{parse_gmr_file, classify};
use ft_ality::automaton::Automaton;
use ft_ality::input::io_shell::{enable_raw_mode, disable_raw_mode, read_key_token};

const STEP_TIMEOUT: Duration = Duration::from_millis(500);
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

fn main() {
    let path = env::args().nth(1).expect("usage: ft_ality <file.gmr> [-d|--debug]");
    let debug = env::args().any(|a| a == "--debug" || a == "-d");

    let grammar  = parse_gmr_file(&path).expect("failed to parse file");
    let compiled = classify(&grammar);
    let automaton = Automaton::from_combos(&compiled.combos);

    let key_to_internal: HashMap<String, String> = compiled
        .bindings.iter()
        .map(|b| (b.key.clone(), b.internal.clone()))
        .collect();

    if let Err(e) = enable_raw_mode() {
        eprintln!("Error enabling raw mode: {e}");
        return;
    }

    let keys = std::iter::from_fn(|| {
        loop {
            match read_key_token() {
                Ok(Some(k)) => return Some((Instant::now(), k)),
                Ok(None)    => continue,
                Err(e)      => { eprintln!("input error: {e}"); return None; }
            }
        }
    });

    keys
    .take_while(|(_, k)| k != "ctrl-c")
    .filter_map(|(now, keytok)| {
        if let Some(internal) = key_to_internal.get(&keytok) {
            if debug { println!("{keytok}  ⇒  {internal}"); }
            Some((now, keytok, internal.clone()))
        } else {
            if debug {
                if keytok.starts_with('[') && keytok.ends_with(']') {
                    println!("(direct) {}", keytok);
                } else {
                    eprintln!("{RED}Unrecognized: {keytok}{RESET}");
                }
            }
            None
        }
    })
    .scan((0usize, None::<Instant>), |(state, last), (now, keytok, internal)| {
        /* Decide baseline state based on elapsed time since last key */
        let reset = last.map_or(false, |prev| now.duration_since(prev) > STEP_TIMEOUT);
        let base_state = if reset { 0 } else { *state };

        /* Pure transition: δ(base_state, token) */
        let (next, outs) = automaton.step(base_state, &internal);

        /* Update scan's carried state */
        *state = next;
        *last = Some(now);

        Some((keytok, next, outs, reset))
    })
    .for_each(|(keytok, state, outs, reset)| {
        if reset && debug {
            eprintln!("[timeout] state reset to 0 before processing {keytok}");
        }
        for m in outs {
            println!("{m} !!");
        }
        if debug {
            let (outputs, _fail) = automaton.state_info(state);
            if outputs.is_empty() {
                println!("{keytok}  ⇒  (no outputs)");
            } else {
                println!("{keytok}  ⇒  {}", outputs.join(", "));
            }
        }
    });

    disable_raw_mode();
    println!("Exiting...");
}
