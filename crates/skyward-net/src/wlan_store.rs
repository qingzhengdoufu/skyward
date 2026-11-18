//! Bounded, private and atomic WLAN credential storage. Mounts belong to the caller.

use crate::supplicant::{self, Saved};
use anyhow::{Context, Result, ensure};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

const MOST: u64 = 64 << 10;

pub fn read(path: &Path) -> Result<Vec<Saved>> {
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).context("open WLAN credential record"),
    };
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= MOST,
        "invalid WLAN credential record extent"
    );
    let mut text = String::new();
    (&mut file).take(MOST + 1).read_to_string(&mut text)?;
    ensure!(text.len() as u64 <= MOST, "WLAN credential record grew while reading");
    Ok(supplicant::saved_read(&text))
}

pub fn write(path: &Path, saved: &[Saved]) -> Result<()> {
    let folder = path.parent().context("WLAN record has no directory")?;
    fs::create_dir_all(folder)?;
    if saved.is_empty() {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error).context("remove WLAN credential record"),
        }
    } else {
        ensure!(saved.len() <= supplicant::SAVED_MOST, "too many WLAN credentials");
        let text = supplicant::saved_text(saved);
        ensure!(text.len() as u64 <= MOST, "WLAN credential record too large");
        ensure!(supplicant::saved_read(&text) == saved, "invalid WLAN credentials");
        let mut temporary = tempfile::Builder::new().prefix(".wlan-").tempfile_in(folder)?;
        temporary.as_file().set_permissions(fs::Permissions::from_mode(0o600))?;
        temporary.write_all(text.as_bytes())?;
        temporary.as_file().sync_all()?;
        temporary
            .persist(path)
            .map_err(|error| error.error)
            .context("commit WLAN credential record")?;
    }
    File::open(folder)?.sync_all().context("flush WLAN record directory")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supplicant::Security;

    fn credential(password: &str) -> Saved {
        Saved {
            ssid: b"Home".to_vec(),
            name: "Home".into(),
            security: Security::Psk,
            password: password.into(),
        }
    }

    #[test]
    fn records_are_private_replaceable_and_removed_when_forgotten() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("wlan");
        assert!(read(&path).unwrap().is_empty());
        write(&path, &[credential("password")]).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        write(&path, &[credential("new-password")]).unwrap();
        assert_eq!(read(&path).unwrap(), [credential("new-password")]);
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 1);
        write(&path, &[]).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn bad_replacements_preserve_the_previous_record_and_reads_reject_links() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("wlan");
        write(&path, &[credential("password")]).unwrap();
        assert!(write(&path, &[credential("bad\npassword")]).is_err());
        assert_eq!(read(&path).unwrap(), [credential("password")]);
        let link = folder.path().join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(read(&link).is_err());
        fs::write(&path, vec![b'x'; MOST as usize + 1]).unwrap();
        assert!(read(&path).is_err());
    }

    #[test]
    fn invalid_records_never_publish_or_leave_temporary_files() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("wlan");
        write(&path, &[credential("password")]).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        write(&path, &[credential("replacement")]).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert!(write(&path, &vec![credential("password"); supplicant::SAVED_MOST + 1]).is_err());
        assert_eq!(read(&path).unwrap(), [credential("replacement")]);
        assert_eq!(fs::read_dir(folder.path()).unwrap().count(), 1);
        assert!(read(folder.path()).is_err());
    }
}
