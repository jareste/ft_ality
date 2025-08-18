use std::time::Instant;
use crate::engine::Engine;
use crate::input::io_shell::{enable_raw_mode, disable_raw_mode, read_key_token};

/// Run the CLI loop: reads tokens from stdin, feeds the Engine, prints outputs.
pub fn run_cli(path: &str, debug: bool, step_timeout_ms: u64) -> Result<(), String> {
    let mut eng = Engine::from_gmr_file(path, std::time::Duration::from_millis(step_timeout_ms))?;

    if let Err(e) = enable_raw_mode() {
        return Err(format!("Error enabling raw mode: {e}"));
    }

    // Main loop
    loop {
        let keytok = match read_key_token() {
            Ok(Some(k)) => k,
            Ok(None) => continue,
            Err(e) => {
                disable_raw_mode();
                return Err(format!("input error: {e}"));
            }
        };

        if keytok == "ctrl-c" {
            break;
        }

        let outs = eng.step_keytok(&keytok, Instant::now());

        for m in outs {
            println!("{m} !!");
        }

        if debug {
            let (outputs, fail) = eng.current_state_info();
            if outputs.is_empty() {
                println!("{keytok}  ⇒  (no outputs)   [state={}, fail={}]", eng.current_state(), fail);
            } else {
                println!("{keytok}  ⇒  {}   [state={}, fail={}]", outputs.join(", "), eng.current_state(), fail);
            }
        }
    }

    disable_raw_mode();
    println!("Exiting...");
    Ok(())
}
