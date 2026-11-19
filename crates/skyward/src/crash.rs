//! Best-effort panic locations, without credentials or panic payloads.
use std::fmt::{self, Write};

struct Record {
    bytes: [u8; 768],
    len: usize,
}

impl Write for Record {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let mut end = text.len().min(self.bytes.len() - self.len);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        self.bytes[self.len..self.len + end].copy_from_slice(&text.as_bytes()[..end]);
        self.len += end;
        Ok(())
    }
}

pub fn install() {
    std::panic::set_hook(Box::new(|panic| {
        let mut record = Record { bytes: [0; 768], len: 0 };
        if let Some(at) = panic.location() {
            let _ = writeln!(
                record,
                "skyward panic at {}:{}:{} (payload omitted)",
                at.file(),
                at.line(),
                at.column()
            );
        } else {
            let _ = writeln!(record, "skyward panic (location unavailable; payload omitted)");
        }
        // No shared logger lock: the panic may have happened inside it.
        for (path, create) in [(c"/skyward/run/panic.log", libc::O_CREAT), (c"/dev/kmsg", 0)] {
            let flags = libc::O_WRONLY | libc::O_APPEND | create | libc::O_CLOEXEC | libc::O_NONBLOCK;
            let fd = unsafe { libc::open(path.as_ptr(), flags, 0o600) };
            if fd >= 0 {
                unsafe {
                    libc::write(fd, record.bytes.as_ptr().cast(), record.len);
                    libc::close(fd);
                }
            }
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_bounded_and_keep_unicode_boundaries() {
        let mut record = Record { bytes: [0; 768], len: 0 };
        write!(record, "{}", "路".repeat(400)).unwrap();
        writeln!(record, "ignored after capacity").unwrap();
        assert_eq!(record.len, 768);
        assert!(std::str::from_utf8(&record.bytes[..record.len]).is_ok());
    }
}
