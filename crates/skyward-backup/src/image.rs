//! Standalone raw images, separate from the versioned multi-part backup format.

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use skyward_files::{Cancelled, Ownership};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copy {
    pub folder: String,
    pub partition: String,
    pub size: u64,
    pub sha256: String,
    pub made: i64,
}

/// Read only metadata here; the entire image is checked before any restore.
pub fn copies(root: &Path) -> Result<Vec<Copy>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && !entry.file_name().to_string_lossy().starts_with('.')
            && let Ok(copy) = read_copy(&entry.path())
        {
            found.push(copy);
        }
    }
    found.sort_by(|a, b| b.made.cmp(&a.made).then_with(|| a.folder.cmp(&b.folder)));
    Ok(found)
}

pub fn read_copy(folder: &Path) -> Result<Copy> {
    ensure!(fs::symlink_metadata(folder)?.file_type().is_dir(), "not an image backup directory");
    let mut images = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "img") {
            ensure!(entry.file_type()?.is_file(), "image is not a regular file");
            images.push(path);
        }
    }
    ensure!(images.len() == 1, "an image backup must hold exactly one image");
    let path = &images[0];
    let partition = path.file_stem().and_then(|name| name.to_str()).context("invalid image name")?;
    ensure!(allowed(partition), "unsupported image backup partition");
    let checksum = folder.join(format!("{partition}.img.sha256"));
    let metadata = fs::symlink_metadata(&checksum)?;
    ensure!(metadata.is_file() && metadata.len() <= 512, "invalid checksum file");
    let sum = fs::read_to_string(checksum)?;
    let fields: Vec<_> = sum.split_whitespace().collect();
    let filename = format!("{partition}.img");
    ensure!(
        fields.len() == 2 && fields[0].len() == 64 && fields[0].bytes().all(|c| c.is_ascii_hexdigit()) && fields[1] == filename,
        "invalid image checksum record"
    );
    let metadata = fs::metadata(path)?;
    ensure!(metadata.len() > 0, "empty image backup");
    let made = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs() as i64);
    Ok(Copy {
        folder: folder
            .file_name()
            .and_then(|name| name.to_str())
            .context("invalid backup folder")?
            .into(),
        partition: partition.into(),
        size: metadata.len(),
        sha256: fields[0].to_ascii_lowercase(),
        made,
    })
}

/// Return the same open file whose complete contents passed verification.
pub fn verified(folder: &Path, expected: &Copy, stop: &AtomicBool, progress: &mut dyn FnMut(u64)) -> Result<File> {
    use std::io::{Seek, SeekFrom};
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    };
    ensure!(read_copy(folder)? == *expected, "image backup metadata changed");
    let directory = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(folder)?;
    let name = std::ffi::CString::new(Path::new(&format!("{}.img", expected.partition)).as_os_str().as_bytes())?;
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    ensure!(fd >= 0, "open verified image: {}", std::io::Error::last_os_error());
    let mut file = unsafe { File::from_raw_fd(fd) };
    ensure!(
        file.metadata()?.is_file() && file.metadata()?.len() == expected.size,
        "image changed"
    );
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 4 << 20];
    let mut done = 0;
    while done < expected.size {
        cancelled(stop)?;
        let count = (expected.size - done).min(buffer.len() as u64) as usize;
        file.read_exact(&mut buffer[..count])?;
        digest.update(&buffer[..count]);
        done += count as u64;
        progress(done);
    }
    ensure!(format!("{:x}", digest.finalize()) == expected.sha256, "image SHA-256 mismatch");
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}

/// The already verified source stays open; the target is checked and read back in full.
pub fn restore_to(source: &mut File, expected: &Copy, target: &Path, progress: &mut dyn FnMut(u64)) -> Result<()> {
    use std::io::{Seek, SeekFrom};
    use std::os::fd::AsRawFd;
    source.seek(SeekFrom::Start(0))?;
    let mut output = fs::OpenOptions::new().write(true).open(target)?;
    let metadata = output.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if metadata.file_type().is_block_device() {
            let mut size = 0u64;
            ensure!(
                unsafe { libc::ioctl(output.as_raw_fd(), 0x8008_1272u32 as _, &mut size) } == 0,
                "read target size"
            );
            ensure!(size == expected.size, "target partition size mismatch");
        } else {
            ensure!(metadata.is_file() && metadata.len() == expected.size, "target image size mismatch");
        }
    }
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 4 << 20];
    let mut done = 0;
    while done < expected.size {
        let count = (expected.size - done).min(buffer.len() as u64) as usize;
        source.read_exact(&mut buffer[..count])?;
        digest.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
        done += count as u64;
        progress(done);
    }
    ensure!(
        format!("{:x}", digest.finalize()) == expected.sha256,
        "image changed while restoring"
    );
    output.sync_all()?;
    let rc = unsafe { libc::posix_fadvise(output.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
    ensure!(rc == 0, "drop restored target cache: {rc}");
    let actual = crate::store::digest(File::open(target)?, expected.size, &mut |read| progress(expected.size + read))?;
    ensure!(actual == expected.sha256, "restored image readback SHA-256 mismatch");
    Ok(())
}

pub const FOLDER: &str = "Skyward/images";
pub const BOOT: [&str; 8] = [
    "boot",
    "init_boot",
    "vendor_boot",
    "dtbo",
    "recovery",
    "vbmeta",
    "vbmeta_system",
    "vbmeta_vendor",
];

pub fn allowed(name: &str) -> bool {
    name == "super"
        || ["_a", "_b"]
            .iter()
            .any(|suffix| name.strip_suffix(suffix).is_some_and(|base| BOOT.contains(&base)))
}

/// Shared super is explicit; every independent image belongs to this slot.
pub fn in_slot(name: &str, suffix: &str) -> bool {
    matches!(suffix, "_a" | "_b") && allowed(name) && (name == "super" || name.ends_with(suffix))
}

fn cancelled(stop: &AtomicBool) -> Result<()> {
    if stop.load(Ordering::Relaxed) {
        return Err(Cancelled.into());
    }
    Ok(())
}

/// The caller creates a unique folder and supplies an already read-only source.
/// Only this function's own newly-created files are removed on failure.
#[allow(clippy::too_many_arguments)]
pub fn export(
    source: &mut impl Read,
    size: u64,
    folder: &Path,
    name: &str,
    ownership: Ownership,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(u64, bool),
) -> Result<PathBuf> {
    ensure!(allowed(name), "partition is not allowed for image export");
    ensure!(size > 0, "empty partition");
    let path = folder.join(format!("{name}.img"));
    let checksum = folder.join(format!("{name}.img.sha256"));
    let mut out = skyward_files::make_file(&path, ownership)?;
    let mut checksum_created = false;
    let result = (|| -> Result<()> {
        let mut digest = Sha256::new();
        let mut buffer = vec![0; 4 << 20];
        let mut done = 0;
        while done < size {
            cancelled(stop)?;
            let n = (size - done).min(buffer.len() as u64) as usize;
            source.read_exact(&mut buffer[..n]).context("read the source partition")?;
            out.write_all(&buffer[..n]).context("write the raw image")?;
            digest.update(&buffer[..n]);
            done += n as u64;
            progress(done, false);
        }
        out.sync_all().context("flush the raw image")?;
        drop(out);
        let expected = digest.finalize();
        let mut back = File::open(&path)?;
        ensure!(back.metadata()?.len() == size, "image length mismatch");
        let mut actual = Sha256::new();
        done = 0;
        while done < size {
            cancelled(stop)?;
            let n = (size - done).min(buffer.len() as u64) as usize;
            back.read_exact(&mut buffer[..n])?;
            actual.update(&buffer[..n]);
            done += n as u64;
            progress(done, true);
        }
        ensure!(actual.finalize() == expected, "image read-back checksum mismatch");
        cancelled(stop)?;
        let mut saved = skyward_files::make_file(&checksum, ownership)?;
        checksum_created = true;
        writeln!(saved, "{expected:x}  {name}.img")?;
        saved.sync_all()?;
        File::open(folder)?.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        let removed = fs::remove_file(&path);
        let sum_removed = if checksum_created { fs::remove_file(&checksum) } else { Ok(()) };
        removed.context("remove incomplete image")?;
        sum_removed.context("remove incomplete checksum")?;
        return Err(error);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn scratch(tag: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("skyward-image-{tag}-{}", std::process::id()));
        fs::create_dir(&folder).unwrap();
        folder
    }

    #[test]
    fn raw_image_and_checksum_are_verified_and_existing_files_are_not_replaced() {
        let folder = scratch("complete");
        let bytes = vec![0x53; (4 << 20) + 17];
        let stop = AtomicBool::new(false);
        let mut stages = Vec::new();
        let path = export(
            &mut Cursor::new(&bytes),
            bytes.len() as u64,
            &folder,
            "recovery_a",
            Ownership::Kept,
            &stop,
            &mut |done, check| stages.push((done, check)),
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            fs::read_to_string(folder.join("recovery_a.img.sha256")).unwrap(),
            format!("{:x}  recovery_a.img\n", Sha256::digest(&bytes))
        );
        assert_eq!(stages.last(), Some(&(bytes.len() as u64, true)));
        assert!(
            export(
                &mut Cursor::new(b"new"),
                3,
                &folder,
                "recovery_a",
                Ownership::Kept,
                &stop,
                &mut |_, _| {}
            )
            .is_err()
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn short_reads_and_cancellation_remove_only_the_new_image() {
        let folder = scratch("failure");
        fs::write(folder.join("keep"), b"keep").unwrap();
        for stop in [false, true] {
            assert!(
                export(
                    &mut Cursor::new(b"short"),
                    100,
                    &folder,
                    "super",
                    Ownership::Kept,
                    &AtomicBool::new(stop),
                    &mut |_, _| {}
                )
                .is_err()
            );
            assert!(!folder.join("super.img").exists());
            assert!(!folder.join("super.img.sha256").exists());
        }
        assert_eq!(fs::read(folder.join("keep")).unwrap(), b"keep");
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn only_physical_boot_partitions_and_shared_super_are_allowed() {
        for name in ["boot_a", "recovery_b", "vbmeta_system_a", "super"] {
            assert!(allowed(name));
        }
        for name in [
            "system_a", "vendor_b", "userdata", "data", "metadata", "persist", "modemst1", "../super", "boot", "super_a",
        ] {
            assert!(!allowed(name));
        }
    }

    #[test]
    fn image_selection_is_limited_to_the_running_slot_and_shared_super() {
        for suffix in ["_a", "_b"] {
            for base in BOOT {
                assert!(in_slot(&format!("{base}{suffix}"), suffix));
                let other = if suffix == "_a" { "_b" } else { "_a" };
                assert!(!in_slot(&format!("{base}{other}"), suffix));
            }
            assert!(in_slot("super", suffix));
            for forbidden in ["userdata", "metadata", "system_a", "persist", "modem_a", "boot", "super_a"] {
                assert!(!in_slot(forbidden, suffix));
            }
        }
        for unknown in ["", "a", "_c"] {
            assert!(!in_slot("boot_a", unknown));
            assert!(!in_slot("super", unknown));
        }
    }

    #[test]
    fn cancellation_during_copy_or_verification_leaves_no_export() {
        let folder = scratch("cancel");
        for checking in [false, true] {
            let stop = AtomicBool::new(false);
            let bytes = vec![0x42; (4 << 20) + 3];
            assert!(
                export(
                    &mut Cursor::new(&bytes),
                    bytes.len() as u64,
                    &folder,
                    "boot_b",
                    Ownership::Kept,
                    &stop,
                    &mut |_, check| {
                        if check == checking {
                            stop.store(true, Ordering::Relaxed);
                        }
                    }
                )
                .is_err()
            );
            assert!(!folder.join("boot_b.img").exists());
            assert!(!folder.join("boot_b.img.sha256").exists());
        }
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn exported_image_is_discovered_verified_and_restored_to_a_regular_test_target() {
        let root = scratch("restore-root");
        let folder = root.join("20261119-170607-boot_a");
        fs::create_dir(&folder).unwrap();
        let bytes = vec![0x75; (4 << 20) + 13];
        export(
            &mut Cursor::new(&bytes),
            bytes.len() as u64,
            &folder,
            "boot_a",
            Ownership::Kept,
            &AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap();
        let copies = copies(&root).unwrap();
        assert_eq!(copies.len(), 1);
        let held = &copies[0];
        assert_eq!(held.partition, "boot_a");
        let mut source = verified(&folder, held, &AtomicBool::new(false), &mut |_| {}).unwrap();
        let target = root.join("target");
        fs::write(&target, vec![0; bytes.len()]).unwrap();
        let mut last = 0;
        restore_to(&mut source, held, &target, &mut |done| last = done).unwrap();
        assert_eq!(fs::read(&target).unwrap(), bytes);
        assert_eq!(last, 2 * bytes.len() as u64);
        let wrong = root.join("wrong-size");
        fs::write(&wrong, b"keep").unwrap();
        assert!(restore_to(&mut source, held, &wrong, &mut |_| {}).is_err());
        assert_eq!(fs::read(wrong).unwrap(), b"keep");
        fs::write(folder.join("boot_a.img"), vec![0x76; bytes.len()]).unwrap();
        assert!(verified(&folder, held, &AtomicBool::new(false), &mut |_| {}).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_checksums_links_and_unsafe_partition_names_are_not_restore_candidates() {
        use std::os::unix::fs::symlink;
        let root = scratch("unsafe-root");
        assert!(copies(&root.join("absent")).unwrap().is_empty());
        let folder = root.join("backup");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("boot_a.img"), b"original").unwrap();
        assert!(read_copy(&folder).is_err());
        fs::write(folder.join("boot_a.img.sha256"), format!("{}  ../boot_a.img\n", "ab".repeat(32))).unwrap();
        assert!(copies(&root).unwrap().is_empty());
        fs::remove_file(folder.join("boot_a.img")).unwrap();
        symlink(root.join("outside"), folder.join("boot_a.img")).unwrap();
        assert!(read_copy(&folder).is_err());
        fs::remove_file(folder.join("boot_a.img")).unwrap();
        fs::write(folder.join("userdata.img"), b"not allowed").unwrap();
        assert!(read_copy(&folder).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancellation_during_verification_never_touches_a_target() {
        let root = scratch("verify-stop");
        let bytes = vec![0x52; (4 << 20) + 3];
        export(
            &mut Cursor::new(&bytes),
            bytes.len() as u64,
            &root,
            "super",
            Ownership::Kept,
            &AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap();
        let held = read_copy(&root).unwrap();
        let stop = AtomicBool::new(false);
        assert!(verified(&root, &held, &stop, &mut |_| stop.store(true, Ordering::Relaxed)).is_err());
        assert_eq!(fs::read(root.join("super.img")).unwrap(), bytes);
        fs::remove_dir_all(root).unwrap();
    }
}
