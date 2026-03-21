use std::io::{self, IsTerminal, Write};

pub fn sanitize_for_plain_output(text: &str) -> String {
    strip_control_sequences(text, true)
}

pub fn sanitize_for_matching(text: &str) -> String {
    strip_control_sequences(text, false)
}

pub fn restore_console_state() -> io::Result<()> {
    const RESET_SEQUENCE: &[u8] = b"\x1b[0m\x1b[?25h";

    if io::stdout().is_terminal() {
        let mut stdout = io::stdout().lock();
        stdout.write_all(RESET_SEQUENCE)?;
        stdout.flush()?;
        return Ok(());
    }

    if io::stderr().is_terminal() {
        let mut stderr = io::stderr().lock();
        stderr.write_all(RESET_SEQUENCE)?;
        stderr.flush()?;
    }

    Ok(())
}

fn strip_control_sequences(text: &str, keep_sgr: bool) -> String {
    let mut output = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != 0x1b {
            let next_escape = bytes[index..]
                .iter()
                .position(|byte| *byte == 0x1b)
                .map(|offset| index + offset)
                .unwrap_or(bytes.len());
            output.push_str(&text[index..next_escape]);
            index = next_escape;
            continue;
        }

        if index + 1 >= bytes.len() {
            break;
        }

        match bytes[index + 1] {
            b'[' => {
                let mut cursor = index + 2;
                while cursor < bytes.len() {
                    let byte = bytes[cursor];
                    if (0x40..=0x7e).contains(&byte) {
                        if keep_sgr && byte == b'm' {
                            output.push_str(&text[index..=cursor]);
                        }
                        index = cursor + 1;
                        break;
                    }
                    cursor += 1;
                }

                if cursor >= bytes.len() {
                    break;
                }
            }
            b']' => {
                let mut cursor = index + 2;
                let mut terminated = false;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        0x07 => {
                            index = cursor + 1;
                            terminated = true;
                            break;
                        }
                        0x1b if cursor + 1 < bytes.len() && bytes[cursor + 1] == b'\\' => {
                            index = cursor + 2;
                            terminated = true;
                            break;
                        }
                        _ => cursor += 1,
                    }
                }

                if !terminated {
                    break;
                }
            }
            _ => {
                index += 2;
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{sanitize_for_matching, sanitize_for_plain_output};

    #[test]
    fn strips_clear_and_title_sequences_but_keeps_sgr_for_plain_output() {
        let input = "\u{1b}[2J\u{1b}[H\u{1b}]0;title\u{7}hello\u{1b}[31m world\u{1b}[0m";
        let output = sanitize_for_plain_output(input);
        assert_eq!(output, "hello\u{1b}[31m world\u{1b}[0m");
    }

    #[test]
    fn strips_all_terminal_sequences_for_matching() {
        let input = "\u{1b}[2J\u{1b}[H\u{1b}]0;title\u{7}hello\u{1b}[31m world\u{1b}[0m";
        let output = sanitize_for_matching(input);
        assert_eq!(output, "hello world");
    }
}
