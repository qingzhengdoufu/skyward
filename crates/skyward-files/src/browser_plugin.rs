//! Install the pinned Aurora rootfs beside the existing installation first.
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::{Component, Path},
};

pub const ROOT: &str = "/data/skyward-browser/chromium";
pub const URL: &str = "https://github.com/Jamvii/skyward-for-aurora/releases/download/browser-v0.3.4-aurora/chromium.zip";
pub const SHA256: &str = "9e6df85255386eb962bbe6b1ad5e0d2505c7b5ba02a9d7d89d3d20a778f14ea2";
pub const LAUNCHER: &str = include_str!("chromium-run.sh");

/// Update existing roots too, atomically, without replacing the browser package.
pub fn refresh_launcher(dir: &Path) -> Result<()> {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(LAUNCHER.as_bytes())?;
    file.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file().set_permissions(fs::Permissions::from_mode(0o755))?;
    }
    file.persist(dir.join("run.sh"))
        .map_err(|e| e.error)
        .context("update Chromium launcher")?;
    Ok(())
}

pub fn install(archive: &Path, dir: &Path) -> Result<()> {
    let mut input = fs::File::open(archive)?;
    let mut digest = Sha256::new();
    let mut chunk = [0u8; 128 * 1024];
    loop {
        let count = input.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        digest.update(&chunk[..count]);
    }
    ensure!(format!("{:x}", digest.finalize()) == SHA256, "Chromium package SHA-256 mismatch");
    let parent = dir.parent().context("browser directory has no parent")?;
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new().prefix("chromium-install-").tempdir_in(parent)?;
    unpack(&mut zip::ZipArchive::new(fs::File::open(archive)?)?, staging.path())?;
    ensure!(staging.path().join("run.sh").is_file(), "missing run.sh");
    ensure!(
        staging.path().join("usr/lib/chromium/chromium").is_file(),
        "missing Chromium executable"
    );
    // Supply a persistent-session launcher with certificate verification enabled.
    fs::write(staging.path().join("run.sh"), LAUNCHER)?;
    let previous = dir.with_file_name("chromium-previous");
    ensure!(!previous.exists(), "previous Chromium installation is still present");
    let existed = dir.exists();
    if existed {
        fs::rename(dir, &previous)?;
    }
    if let Err(error) = fs::rename(staging.path(), dir) {
        if existed {
            let _ = fs::rename(&previous, dir);
        }
        return Err(error.into());
    }
    if existed {
        let _ = fs::remove_dir_all(previous);
    }
    Ok(())
}

fn unpack<R: io::Read + io::Seek>(zip: &mut zip::ZipArchive<R>, dir: &Path) -> Result<()> {
    ensure!(zip.len() <= 30_000, "too many browser files");
    let mut total = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let relative = entry.enclosed_name().context("browser archive path escapes its directory")?;
        ensure!(
            relative.components().all(|p| matches!(p, Component::Normal(_))),
            "invalid browser path"
        );
        let mode = entry.unix_mode().unwrap_or(0o100644);
        ensure!(
            matches!(mode & 0o170000, 0 | 0o100000 | 0o040000),
            "browser archive contains a special file"
        );
        total = total.checked_add(entry.size()).context("browser archive is too large")?;
        ensure!(total <= 3 * 1024 * 1024 * 1024, "browser archive exceeds 3 GiB");
        let path = dir.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
        ensure!(io::copy(&mut entry, &mut out)? == entry.size(), "truncated browser file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(mode & 0o755))?;
        }
    }
    if total == 0 {
        bail!("empty browser archive");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    #[ignore = "requires the pinned Aurora release archive"]
    fn pinned_release_installs_and_preserves_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let archive = std::env::var("SKYWARD_AURORA_ARCHIVE").unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("chromium");
        install(Path::new(&archive), &root).unwrap();
        let binary = root.join("usr/lib/chromium/chromium");
        assert_ne!(fs::metadata(binary).unwrap().permissions().mode() & 0o111, 0);
        assert!(
            fs::read_to_string(root.join("run.sh"))
                .unwrap()
                .contains("--remote-debugging-address=127.0.0.1")
        );
    }
    #[test]
    fn rejects_parent_traversal_and_keeps_files_inside_staging() {
        let mut writer = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        writer.start_file("../outside", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"test").unwrap();
        let cursor = writer.finish().unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert!(unpack(&mut zip::ZipArchive::new(cursor).unwrap(), dir.path()).is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
    #[test]
    fn a_failed_checksum_does_not_touch_an_existing_installation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("chromium");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("keep"), b"keep").unwrap();
        let archive = dir.path().join("bad.zip");
        fs::write(&archive, b"not a package").unwrap();
        assert!(install(&archive, &root).is_err());
        assert_eq!(fs::read(root.join("keep")).unwrap(), b"keep");
    }
    #[test]
    fn existing_roots_receive_the_launcher_and_software_vulkan_configuration() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("run.sh"), b"old launcher").unwrap();
        fs::write(dir.path().join("keep"), b"browser files").unwrap();
        refresh_launcher(dir.path()).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("run.sh")).unwrap(), LAUNCHER);
        assert_eq!(fs::read(dir.path().join("keep")).unwrap(), b"browser files");
        assert!(LAUNCHER.contains("VK_DRIVER_FILES=/usr/lib/chromium/vk_swiftshader_icd.json"));
        assert!(!LAUNCHER.contains("ignore-certificate-errors"));
    }
}
