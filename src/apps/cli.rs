use std::time::Instant;
use std::time::Duration;
use crate::engine::{step_keytok, engine_from_gmr_file, current_state_info};
use crate::input::io_shell::{enable_raw_mode, disable_raw_mode, read_key_token};

pub fn run_cli(path: &str, debug: bool, step_timeout_ms: u64) -> Result<(), String> {
    let (cfg, mut st) = engine_from_gmr_file(path, Duration::from_millis(step_timeout_ms))?;

    if let Err(e) = enable_raw_mode() {
        return Err(format!("Error enabling raw mode: {e}"));
    }

    let timeout = Duration::from_millis(10_000);
    let esc_tail_timeout = Duration::from_millis(120);
    loop {
        let keytok = match read_key_token(timeout, esc_tail_timeout) {
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

        let now_ms = Instant::now().elapsed().as_millis() as u128;
        let (st2, outs) = step_keytok(&cfg, st, &keytok, now_ms);
        st = st2;

        for m in outs {
            println!("{m} !!");
        }

        if debug {
            let (outputs, fail) = current_state_info(&cfg, st);
            if outputs.is_empty() {
                println!("{keytok}  ⇒  (no outputs)   [state={}, fail={}]", st.cur_state, fail);
            } else {
                println!("{keytok}  ⇒  {}   [state={}, fail={}]", outputs.join(", "), st.cur_state, fail);
            }
        }
    }

    disable_raw_mode();
    println!("Exiting...");
    Ok(())
}
