use std::fs;
use std::io::Write as _;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

const HOSTS_PATH: &str = "/etc/hosts";
const MARK_START: &str = "# >>> apiproxy >>>";
const MARK_END: &str = "# <<< apiproxy <<<";

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

fn write(lines: &[String]) -> Result<()> {
    let mut out = lines.join("\n");
    out.push('\n');
    match fs::write(HOSTS_PATH, &out) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => write_elevated(&out),
        Err(e) => Err(e.into()),
    }
}

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

pub fn apply(hostname: &str, bind_ip: &str) -> Result<()> {
    let content = fs::read_to_string(HOSTS_PATH)?;
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    strip_block(&mut lines);
    lines.push(MARK_START.to_owned());
    lines.push(format!("{bind_ip} {hostname}"));
    lines.push(MARK_END.to_owned());
    write(&lines)
}
