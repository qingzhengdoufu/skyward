//! Bounded display transport. A trusted, separately audited jail is mandatory.

use crate::protocol::{FRAME_BYTES, MESSAGE_BYTES, Message, SHARED_BYTES};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};

pub struct Session {
    pixels: File,
    control: OwnedFd,
    last: Option<u32>,
}

impl Session {
    /// Attach only sealed, fixed-size pixels and a sequenced packet socket.
    pub fn attach(pixels: File, control: OwnedFd) -> Result<Self> {
        ensure!(pixels.metadata()?.len() == SHARED_BYTES as u64, "browser pixel extent mismatch");
        let seals = unsafe { libc::fcntl(pixels.as_raw_fd(), libc::F_GET_SEALS) };
        ensure!(
            seals >= 0 && seals & (libc::F_SEAL_GROW | libc::F_SEAL_SHRINK) == libc::F_SEAL_GROW | libc::F_SEAL_SHRINK,
            "browser pixels are not size-sealed"
        );
        let mut kind = 0i32;
        let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
        ensure!(
            unsafe {
                libc::getsockopt(
                    control.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_TYPE,
                    (&mut kind as *mut i32).cast(),
                    &mut len,
                )
            } == 0
                && kind == libc::SOCK_SEQPACKET,
            "wrong browser channel type"
        );
        Ok(Self {
            pixels,
            control,
            last: None,
        })
    }

    pub fn receive(&mut self) -> Result<Option<Message>> {
        let mut bytes = [0; MESSAGE_BYTES + 1];
        let n = unsafe {
            libc::recv(
                self.control.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
                libc::MSG_DONTWAIT | libc::MSG_TRUNC,
            )
        };
        if n < 0 {
            let e = std::io::Error::last_os_error();
            if e.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(e.into());
        }
        ensure!(n > 0 && n as usize == MESSAGE_BYTES, "browser channel closed or malformed packet");
        let message = Message::decode(&bytes[..MESSAGE_BYTES])?;
        message.validate_worker()?;
        Ok(Some(message))
    }

    /// Copy before acknowledgement; never expose an engine-owned canvas pointer.
    pub fn copy_frame(&mut self, message: &Message) -> Result<Vec<u8>> {
        let at = message.frame_offset()?;
        ensure!(
            self.last
                .is_none_or(|last| message.sequence.wrapping_sub(last) > 0 && message.sequence.wrapping_sub(last) < 1 << 31),
            "stale browser frame"
        );
        let mut out = vec![0; FRAME_BYTES];
        self.pixels.seek(SeekFrom::Start(at as u64))?;
        self.pixels.read_exact(&mut out)?;
        self.last = Some(message.sequence);
        let mut ack = Message::command(10);
        ack.sequence = message.sequence;
        self.send(&ack)?;
        Ok(out)
    }

    pub fn send(&self, message: &Message) -> Result<()> {
        message.validate_command()?;
        let bytes = message.encode()?;
        ensure!(
            unsafe {
                libc::send(
                    self.control.as_raw_fd(),
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
                )
            } == MESSAGE_BYTES as isize,
            "send browser command"
        );
        Ok(())
    }
}

/// A missing launcher is a hard gate, not permission to run WebKit as root.
pub fn launcher_available() -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(Path::new(crate::LAUNCHER))
        .context("audited browser sandbox launcher is not installed")?;
    let m = file.metadata()?;
    ensure!(
        m.is_file() && m.uid() == 0 && m.mode() & 0o022 == 0 && m.mode() & 0o111 != 0,
        "unsafe browser launcher ownership or mode"
    );
    // No launcher digest has passed device isolation tests yet. Fail closed.
    anyhow::bail!("browser launcher is not yet approved for this build")
}

pub fn new_channels() -> Result<(File, OwnedFd, OwnedFd)> {
    let fd = unsafe { libc::memfd_create(c"skyward-browser-pixels".as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    ensure!(fd >= 0, "create browser pixels");
    let file = unsafe { File::from_raw_fd(fd) };
    file.set_len(SHARED_BYTES as u64)?;
    ensure!(
        unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL) } == 0,
        "seal browser pixels"
    );
    let mut fds = [-1; 2];
    ensure!(
        unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC, 0, fds.as_mut_ptr()) } == 0,
        "create browser channel"
    );
    Ok((file, unsafe { OwnedFd::from_raw_fd(fds[0]) }, unsafe {
        OwnedFd::from_raw_fd(fds[1])
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn sealed_frame_is_copied_then_acknowledged_and_duplicates_are_refused() {
        let (mut pixels, host, worker) = new_channels().unwrap();
        pixels.seek(SeekFrom::Start(FRAME_BYTES as u64)).unwrap();
        pixels.write_all(&[42; 16]).unwrap();
        let mut session = Session::attach(pixels, host).unwrap();
        let frame = Message {
            kind: 32,
            sequence: 1,
            x: 1080,
            y: 2100,
            value: FRAME_BYTES as u32,
            text: String::new(),
        };
        let out = session.copy_frame(&frame).unwrap();
        assert_eq!(&out[..16], &[42; 16]);
        let mut wire = [0; MESSAGE_BYTES];
        let n = unsafe { libc::recv(worker.as_raw_fd(), wire.as_mut_ptr().cast(), wire.len(), libc::MSG_DONTWAIT) };
        assert_eq!(n, MESSAGE_BYTES as isize);
        let ack = Message::decode(&wire).unwrap();
        assert_eq!((ack.kind, ack.sequence), (10, 1));
        assert!(session.copy_frame(&frame).is_err());
    }
    #[test]
    fn regular_unsealed_pixels_cannot_attach() {
        let temp = tempfile::tempfile().unwrap();
        temp.set_len(SHARED_BYTES as u64).unwrap();
        let (_, host, _) = new_channels().unwrap();
        assert!(Session::attach(temp, host).is_err());
    }
}
