//! Standalone raw images, separate from the versioned multi-part backup format.

use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use skyward_files::{Cancelled, Ownership};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const FOLDER: &str = "Skyward/images";
pub const BOOT: [&str; 8] = ["boot", "init_boot", "vendor_boot", "dtbo", "recovery", "vbmeta", "vbmeta_system", "vbmeta_vendor"];

pub fn allowed(name: &str) -> bool {
    name == "super" || ["_a", "_b"].iter().any(|suffix| name.strip_suffix(suffix).is_some_and(|base| BOOT.contains(&base)))
}

fn cancelled(stop: &AtomicBool) -> Result<()> {
    if stop.load(Ordering::Relaxed) { return Err(Cancelled.into()); }
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
        let path = export(&mut Cursor::new(&bytes), bytes.len() as u64, &folder, "recovery_a", Ownership::Kept, &stop, &mut |done, check| stages.push((done, check))).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read_to_string(folder.join("recovery_a.img.sha256")).unwrap(), format!("{:x}  recovery_a.img\n", Sha256::digest(&bytes)));
        assert_eq!(stages.last(), Some(&(bytes.len() as u64, true)));
        assert!(export(&mut Cursor::new(b"new"), 3, &folder, "recovery_a", Ownership::Kept, &stop, &mut |_, _| {}).is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn short_reads_and_cancellation_remove_only_the_new_image() {
        let folder = scratch("failure");
        fs::write(folder.join("keep"), b"keep").unwrap();
        for stop in [false, true] {
            assert!(export(&mut Cursor::new(b"short"), 100, &folder, "super", Ownership::Kept, &AtomicBool::new(stop), &mut |_, _| {}).is_err());
            assert!(!folder.join("super.img").exists());
            assert!(!folder.join("super.img.sha256").exists());
        }
        assert_eq!(fs::read(folder.join("keep")).unwrap(), b"keep");
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn only_physical_boot_partitions_and_shared_super_are_allowed() {
        for name in ["boot_a", "recovery_b", "vbmeta_system_a", "super"] { assert!(allowed(name)); }
        for name in ["system_a", "vendor_b", "userdata", "data", "metadata", "persist", "modemst1", "../super", "boot", "super_a"] { assert!(!allowed(name)); }
    }

    #[test]
    fn cancellation_during_copy_or_verification_leaves_no_export() {
        let folder = scratch("cancel");
        for checking in [false, true] {
            let stop = AtomicBool::new(false);
            let bytes = vec![0x42; (4 << 20) + 3];
            assert!(export(&mut Cursor::new(&bytes), bytes.len() as u64, &folder, "boot_b", Ownership::Kept, &stop, &mut |_, check| {
                if check == checking { stop.store(true, Ordering::Relaxed); }
            }).is_err());
            assert!(!folder.join("boot_b.img").exists());
            assert!(!folder.join("boot_b.img.sha256").exists());
        }
        fs::remove_dir_all(folder).unwrap();
    }
}
