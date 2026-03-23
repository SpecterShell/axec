use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::paths;
use crate::protocol::SessionMeta;

const REPL_META_FILE_NAME: &str = "axrepl.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplDriver {
    Python,
    Node,
    Bash,
    Zsh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplSessionMeta {
    driver: ReplDriver,
}

pub fn infer_driver(command: &str) -> Option<ReplDriver> {
    let name = Path::new(command)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(command)
        .to_ascii_lowercase();

    if name.starts_with("python") || name == "py" {
        Some(ReplDriver::Python)
    } else if name == "node" || name == "nodejs" {
        Some(ReplDriver::Node)
    } else if name == "bash" {
        Some(ReplDriver::Bash)
    } else if name == "zsh" {
        Some(ReplDriver::Zsh)
    } else {
        None
    }
}

pub fn read_session_driver(id: &Uuid) -> Result<Option<ReplDriver>> {
    let path = repl_meta_path(id)?;
    if !path.exists() {
        return Ok(None);
    }

    let meta: ReplSessionMeta = serde_json::from_slice(&fs::read(path)?)?;
    Ok(Some(meta.driver))
}

pub fn write_session_driver(id: &Uuid, driver: ReplDriver) -> Result<()> {
    let path = repl_meta_path(id)?;
    fs::write(
        path,
        serde_json::to_vec_pretty(&ReplSessionMeta { driver })?,
    )?;
    Ok(())
}

pub fn infer_session_driver(id: &Uuid) -> Result<Option<ReplDriver>> {
    if let Some(driver) = read_session_driver(id)? {
        return Ok(Some(driver));
    }

    let meta: SessionMeta = serde_json::from_slice(&fs::read(paths::session_meta_path(id)?)?)?;
    Ok(infer_driver(&meta.command))
}

pub fn wrap_script(driver: ReplDriver, script: &str, marker: &str) -> Result<String> {
    Ok(match driver {
        ReplDriver::Python => wrap_python_script(script, marker)?,
        ReplDriver::Node => wrap_node_script(script, marker)?,
        ReplDriver::Bash | ReplDriver::Zsh => wrap_shell_script(script, marker),
    })
}

pub fn strip_completion_output(_sent_input: &str, marker: &str, output: &str) -> String {
    let mut cleaned = strip_leading_echo(output).to_string();

    if let Some(index) = cleaned.find(marker) {
        cleaned.truncate(index);
    }

    cleaned
}

fn repl_meta_path(id: &Uuid) -> Result<std::path::PathBuf> {
    Ok(paths::session_dir(id)?.join(REPL_META_FILE_NAME))
}

fn wrap_python_script(script: &str, marker: &str) -> Result<String> {
    let (marker_left, marker_right) = split_marker(marker);
    let wrapper = format!(
        "import traceback\n__axrepl_code = {}\ntry:\n    exec(compile(__axrepl_code, \"<axrepl-input>\", \"exec\"), globals(), globals())\nexcept Exception:\n    traceback.print_exc()\nprint({} + {})",
        serde_json::to_string(script)?,
        serde_json::to_string(marker_left)?,
        serde_json::to_string(marker_right)?,
    );

    Ok(format!(
        "exec(compile({}, \"<axrepl-wrapper>\", \"exec\"), globals(), globals())\n",
        serde_json::to_string(&wrapper)?,
    ))
}

fn wrap_node_script(script: &str, marker: &str) -> Result<String> {
    let (marker_left, marker_right) = split_marker(marker);
    Ok(format!(
        "(() => {{ const __axreplCode = {}; try {{ eval(__axreplCode); }} catch (err) {{ console.error(err && err.stack ? err.stack : String(err)); }} console.log({} + {}); }})()\n",
        serde_json::to_string(script)?,
        serde_json::to_string(marker_left)?,
        serde_json::to_string(marker_right)?,
    ))
}

fn wrap_shell_script(script: &str, marker: &str) -> String {
    let (marker_left, marker_right) = split_marker(marker);
    format!(
        "__axrepl_code={}; eval \"$__axrepl_code\"; printf '%s%s\\n' {} {}\n",
        ansi_c_quote(script),
        single_quote(marker_left),
        single_quote(marker_right),
    )
}

fn strip_leading_echo(output: &str) -> &str {
    if let Some(index) = output.find('\n') {
        return &output[index + 1..];
    }
    output
}

fn single_quote(text: &str) -> String {
    let mut out = String::from("'");
    for ch in text.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn ansi_c_quote(text: &str) -> String {
    let mut out = String::from("$'");
    for &byte in text.as_bytes() {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'\'' => out.push_str("\\'"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(byte as char),
            _ => out.push_str(&format!("\\x{byte:02x}")),
        }
    }
    out.push('\'');
    out
}

fn split_marker(marker: &str) -> (&str, &str) {
    let split_at = marker.len() / 2;
    marker.split_at(split_at)
}

#[cfg(test)]
mod tests {
    use super::{ReplDriver, infer_driver, strip_completion_output, wrap_script};

    #[test]
    fn infers_common_repl_commands() {
        assert_eq!(infer_driver("python3"), Some(ReplDriver::Python));
        assert_eq!(infer_driver("/usr/bin/node"), Some(ReplDriver::Node));
        assert_eq!(infer_driver("bash"), Some(ReplDriver::Bash));
        assert_eq!(infer_driver("zsh"), Some(ReplDriver::Zsh));
        assert_eq!(infer_driver("cargo"), None);
    }

    #[test]
    fn strips_echo_and_marker() {
        let sent = "print(1)\n";
        let output = "print(1)\r\n1\r\n__AXREPL_DONE__\r\n>>> ";
        assert_eq!(strip_completion_output(sent, "__AXREPL_DONE__", output), "1\r\n");
    }

    #[test]
    fn wraps_python_in_a_single_input_line() {
        let wrapped = wrap_script(ReplDriver::Python, "print(1)", "__DONE__").unwrap();
        assert!(!wrapped.is_empty());
        assert!(wrapped.ends_with('\n'));
        assert_eq!(wrapped.matches('\n').count(), 1);
    }
}
