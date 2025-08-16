use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct RawMode {
    active: bool,
}

impl RawMode {
    pub fn new() -> io::Result<Self> {
        #[cfg(unix)]
        {
            let status = Command::new("sh")
                .arg("-c")
                .arg("stty -echo -icanon time 1 min 0")
                .stdin(Stdio::inherit())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if !status.success() {
                return Err(io::Error::new(io::ErrorKind::Other, "stty failed"));
            }
            Ok(Self { active: true })
        }
        #[cfg(not(unix))]
        {
            Err(io::Error::new(io::ErrorKind::Unsupported, "Raw mode is not supported on this platform"))
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if self.active {
            let _ = Command::new("sh").arg("-c").arg("stty sane").status();
        }
    }
}

fn read_byte(timeout: Duration) -> io::Result<Option<u8>> {
    let start = Instant::now();
    let mut buf = [0u8; 1];
    loop {
        match io::stdin().read(&mut buf) {
            Ok(1) => return Ok(Some(buf[0])),
            Ok(0) => {
                if start.elapsed() >= timeout {
                    return Ok(None);
                }
                continue;
            }
            Ok(_) => unreachable!(),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

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

fn decode_escape_sequence() -> String {
    let timeout = Duration::from_millis(120);

    let b1 = read_byte(timeout).ok().flatten();
    if b1 != Some(b'[') {
        if let Some(c) = b1 {
            if (0x20..=0x7E).contains(&c) {
                let key = (c as char).to_string().to_lowercase();
                return format!("alt-{}", key);
            }
        }
        return "esc".to_string();
    }

    let mut params: Vec<u8> = Vec::new();
    let mut final_byte: Option<u8> = None;

    for _ in 0..6 {
        if let Some(b) = read_byte(timeout).ok().flatten() {
            if (b'A'..=b'Z').contains(&b) || b"~@".contains(&b) {
                final_byte = Some(b);
                break;
            } else {
                params.push(b);
            }
        } else {
            break;
        }
    }

    let parse_mod = |p: &[u8]| -> u8 {
        let s = String::from_utf8_lossy(p);
        if let Some(idx) = s.find(';') {
            if let Some(m) = s[idx + 1..].split(';').next() {
                if let Ok(v) = m.parse::<u8>() {
                    return v;
                }
            }
        }
        0
    };

    let fin = final_byte.unwrap_or(b'?');
    let m = parse_mod(&params);
    let mp = mod_prefix(m);

    match fin {
        b'A' => format!("{mp}up"),
        b'B' => format!("{mp}down"),
        b'C' => format!("{mp}right"),
        b'D' => format!("{mp}left"),
        b'~' => {
            let s = String::from_utf8_lossy(&params);
            let code = s.split(';').next().unwrap_or("");
            match code {
                "3" => format!("{mp}delete"),
                _ => "esc".to_string(),
            }
        }
        _ => "esc".to_string(),
    }
}

fn ctrl_combo(b: u8) -> Option<String> {
    if (1..=26).contains(&b) {
        let ch = ((b - 1) + b'a') as char;
        return Some(format!("ctrl-{}", ch));
    }
    None
}

pub fn read_key_token() -> io::Result<Option<String>> {
    #[cfg(not(unix))]
    {
        Err(io::Error::new(io::ErrorKind::Unsupported, "read_key_token is not supported on this platform"))
    }

    #[cfg(unix)]
    {
        let timeout = Duration::from_millis(10_000);
        let b = match read_byte(timeout)? {
            Some(b) => b,
            None => return Ok(None),
        };

        if let Some(tok) = ctrl_combo(b) {
            return Ok(Some(tok));
        }

        if b == b' ' {
            return Ok(Some("space".into()));
        }

        if b == b'\r' || b == b'\n' {
            return Ok(Some("enter".into()));
        }

        if b == 0x7f || b == 0x08 {
            return Ok(Some("backspace".into()));
        }

        if b == 0x1B {
            return Ok(Some(decode_escape_sequence()));
        }

        if (0x20..=0x7E).contains(&b) {
            let ch = b as char;
            if ch.is_ascii_uppercase() {
                let lower = ch.to_ascii_lowercase();
                return Ok(Some(format!("shift-{}", lower)));
            } else {
                return Ok(Some(ch.to_string()));
            }
        }

        Ok(None)
    }
}
