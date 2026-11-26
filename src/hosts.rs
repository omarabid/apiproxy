//! `/etc/hosts` management.
//!
//! Edits only the block delimited by [`MARK_START`] and [`MARK_END`],
//! elevating via `sudo tee` when direct writes are not permitted.

use std::fs;
use std::io::Write as _;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

const HOSTS_PATH: &str = "/etc/hosts";
const MARK_START: &str = "# >>> apiproxy >>>";
const MARK_END: &str = "# <<< apiproxy <<<";

/// Removes the existing apiproxy block from `lines`, if present.
fn strip_block(lines: &mut Vec<String>) {
    let Some(start) = lines.iter().position(|l| l.trim() == MARK_START) else {
        return;
    };
    let end = lines[start..]
        .iter()
        .position(|l| l.trim() == MARK_END)
        .map(|i| i + start)
        .unwrap_or(lines.len() - 1);
    lines.drain(start..=end);
}

/// Writes `lines` to `/etc/hosts`, elevating on permission errors.
fn write(lines: &[String]) -> Result<()> {
    let mut out = lines.join("\n");
    out.push('\n');
    match fs::write(HOSTS_PATH, &out) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => write_elevated(&out),
        Err(e) => Err(e.into()),
    }
}

/// Writes `content` to `/etc/hosts` through `sudo tee`.
fn write_elevated(content: &str) -> Result<()> {
    let mut child = Command::new("sudo")
        .arg("tee")
        .arg(HOSTS_PATH)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())?;
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Elevate)
    }
}

/// Rewrites the apiproxy block to map `hostname` to `bind_ip`.
pub fn apply(hostname: &str, bind_ip: &str) -> Result<()> {
    let content = fs::read_to_string(HOSTS_PATH)?;
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    strip_block(&mut lines);
    lines.push(MARK_START.to_owned());
    lines.push(format!("{bind_ip} {hostname}"));
    lines.push(MARK_END.to_owned());
    write(&lines)
}
