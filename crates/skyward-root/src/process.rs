use anyhow::{Context, Result, ensure};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn drain(pipe: &mut impl Read, kept: &mut Vec<u8>) -> std::io::Result<()> {
    let mut buf = [0; 4096];
    for _ in 0..64 {
        let n = match pipe.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        kept.extend_from_slice(&buf[..n]);
        if kept.len() > 4096 {
            kept.drain(..kept.len() - 4096);
        }
    }
    Ok(())
}
fn nonblocking(pipe: &impl AsRawFd) -> std::io::Result<()> {
    // SAFETY: fcntl only changes flags of an open pipe owned by the caller.
    let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Drain both pipes with bounded memory; a stuck script cannot hold the UI forever.
pub(super) fn run(command: &mut Command, timeout: Duration) -> Result<()> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let mut stdout = child.stdout.take().context("missing stdout pipe")?;
    let mut stderr = child.stderr.take().context("missing stderr pipe")?;
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let deadline = Instant::now() + timeout;
    let status = (|| -> Result<_> {
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        loop {
            drain(&mut stdout, &mut out)?;
            drain(&mut stderr, &mut err)?;
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            ensure!(Instant::now() < deadline, "installer exceeded its time limit");
            thread::sleep(Duration::from_millis(50));
        }
    })();
    if status.is_err() {
        // SAFETY: the live child owns a process group separate from the UI.
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.wait();
    }
    if status.is_ok() {
        let _ = drain(&mut stdout, &mut out);
        let _ = drain(&mut stderr, &mut err);
    }
    ensure!(
        status?.success(),
        "installer failed: {} {}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&err)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_retains_only_the_tail() {
        let mut bytes = vec![b'a'; 16000];
        bytes.extend_from_slice(b"done");
        let mut got = Vec::new();
        drain(&mut bytes.as_slice(), &mut got).unwrap();
        assert_eq!(got.len(), 4096);
        assert!(got.ends_with(b"done"));
    }
    #[test]
    fn a_hung_process_is_stopped() {
        assert!(run(Command::new("sh").args(["-c", "sleep 60"]), Duration::from_millis(100)).is_err());
    }
}
