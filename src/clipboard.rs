use base64::Engine;
use std::io::{self, Write};

/// Ask the terminal to place `text` on the system clipboard via OSC 52.
///
/// Emitted unwrapped: modern tmux (`set-clipboard external|on`, default since
/// 2.6) intercepts and forwards OSC 52 itself, whereas the `\ePtmux;`
/// passthrough is dropped by the default `allow-passthrough off`. Terminals
/// without OSC 52 support will simply ignore the sequence.
pub fn copy(text: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    out.write_all(&osc52(text))?;
    out.flush()
}

fn osc52(text: &str) -> Vec<u8> {
    let payload = base64::engine::general_purpose::STANDARD.encode(text);
    format!("\x1b]52;c;{payload}\x07").into_bytes()
}

#[cfg(test)]
mod tests {
    #[test]
    fn osc52_shape() {
        assert_eq!(super::osc52("hi"), b"\x1b]52;c;aGk=\x07");
    }
}
