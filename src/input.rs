use std::time::Duration;

fn mod_prefix(n: u8) -> &'static str {
    match n {
        2 => "shift-",
        3 => "alt-",
        4 => "shift-alt-",
        5 => "ctrl-",
        6 => "shift-ctrl-",
        7 => "alt-ctrl-",
        8 => "shift-alt-ctrl-",
        _ => "",
    }
}

fn ctrl_combo(b: u8) -> Option<String> {
    (1..=26)
        .contains(&b)
        .then(|| ((b - 1) + b'a') as char)
        .map(|ch| format!("ctrl-{ch}"))
}

fn parse_csi_mod(params_ascii: &str) -> u8 {
    params_ascii
        .split(';')
        .nth(1)
        .and_then(|m| m.parse::<u8>().ok())
        .unwrap_or(0)
}

fn read_csi_tail<F>(next_byte: F, timeout: Duration, max_steps: usize)
    -> Option<(String, u8)>
where
    F: FnMut(Duration) -> Option<u8>,
{
    fn go<F>(mut next: F, timeout: Duration, steps: usize, acc: String) -> Option<(String, u8)>
    where
        F: FnMut(Duration) -> Option<u8>,
    {
        if steps == 0 {
            return None;
        }
        let b = next(timeout)?;
        if (b'A'..=b'Z').contains(&b) || b"~@".contains(&b) {
            Some((acc, b))
        } else {
            let mut acc2 = acc;
            acc2.push(b as char);
            go(next, timeout, steps - 1, acc2)
        }
    }
    go(next_byte, timeout, max_steps, String::new())
}

/* Just a helper to decode escape sequences.
 * It reads the next byte and checks if it is an escape sequence.
 * If it is, it decodes it into a string representation.
 * For example, it decodes "esc [ A" into "up".
 * If it is not an escape sequence, it returns "esc".
 */
fn decode_escape_sequence<F>(mut next_byte: F) -> String
where
    F: FnMut(Duration) -> Option<u8>,
{
    let timeout = Duration::from_millis(120);

    match next_byte(timeout) {
        Some(b'[') => {
            if let Some((params, fin)) = read_csi_tail(&mut next_byte, timeout, 6) {
                let m = parse_csi_mod(&params);
                let mp = mod_prefix(m);
                match fin {
                    b'A' => format!("{mp}up"),
                    b'B' => format!("{mp}down"),
                    b'C' => format!("{mp}right"),
                    b'D' => format!("{mp}left"),
                    b'~' => {
                        let code = params.split(';').next().unwrap_or("");
                        match code {
                            "3" => format!("{mp}delete"),
                            _ => "esc".to_string(),
                        }
                    }
                    _ => "esc".to_string(),
                }
            } else {
                "esc".to_string()
            }
        }
        Some(c) if (0x20..=0x7E).contains(&c) => {
            format!("alt-{}", (c as char).to_ascii_lowercase())
        }
        _ => "esc".to_string(),
    }
}

/* This function reads a single token from the input.
 * It waits for a key press and returns the token as a string.
 * It supports escape sequences, control characters, and printable characters.
 */
pub fn decode_one_token<F>(mut next_byte: F) -> Option<String>
where
    F: FnMut(Duration) -> Option<u8>,
{
    let timeout = Duration::from_millis(10_000);
    let b = next_byte(timeout)?;

    if let Some(tok) = ctrl_combo(b) {
        return Some(tok);
    }
    if b == b' ' {
        return Some("space".into());
    }
    if b == b'\r' || b == b'\n' {
        return Some("enter".into());
    }
    if b == 0x7f || b == 0x08 {
        return Some("backspace".into());
    }
    if b == 0x1B {
        return Some(decode_escape_sequence(&mut next_byte));
    }
    if (0x20..=0x7E).contains(&b) {
        let ch = b as char;
        return Some(if ch.is_ascii_uppercase() {
            format!("shift-{}", ch.to_ascii_lowercase())
        } else {
            ch.to_string()
        });
    }
    None
}

#[cfg(unix)]
pub mod io_shell {
    use super::*;
    use std::io::{self, Read};
    use std::process::{Command, Stdio};
    use std::time::Instant;

    /* Enable raw mode for the terminal.
     * This disables line buffering and echoing, allowing for immediate input reading.
     * It uses the `stty` command to set the terminal attributes.
     */
    pub fn enable_raw_mode() -> io::Result<()> {
        let status = Command::new("sh")
            .arg("-c")
            .arg("stty -echo -icanon time 1 min 0")
            .stdin(Stdio::inherit())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "stty failed"))
        }
    }

    pub fn disable_raw_mode() {
        let _ = Command::new("sh").arg("-c").arg("stty sane").status();
    }

    /* Read a single byte from stdin with a timeout.
     * Returns Some(byte) if a byte is read, or None if the timeout expires.
     */
    pub fn stdin_next_byte(timeout: Duration) -> Option<u8> {
        let start = Instant::now();
        let mut buf = [0u8; 1];
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        loop {
            match handle.read(&mut buf) {
                Ok(1) => return Some(buf[0]),
                Ok(0) => {
                    if start.elapsed() >= timeout {
                        return None;
                    }
                    continue;
                }
                Ok(_) => unreachable!(),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return None,
            }
        }
    }

    /* Read a key token from stdin.
     * It uses the `decode_one_token` function to read and decode the input.
     * Returns Some(token) if a token is read, or None if the timeout expires.
     */
    pub fn read_key_token() -> io::Result<Option<String>> {
        Ok(super::decode_one_token(|t| stdin_next_byte(t)))
    }
}
