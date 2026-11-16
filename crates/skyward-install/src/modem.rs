//! Release mounted modem firmware only for packages that carry a modem image.
#[cfg(target_os = "android")]
use anyhow::ensure;
use anyhow::{Context, Result};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

/// Inspect the ZIP directory, without extracting or interpreting its script.
pub fn in_package<R: Read + Seek>(file: R) -> Result<bool> {
    let zip = zip::ZipArchive::new(file).context("read firmware package directory")?;
    Ok(zip.file_names().any(firmware_entry))
}

fn firmware_entry(name: &str) -> bool {
    let Some(name) = name.rsplit('/').next() else { return false };
    ["modem.img", "NON-HLOS.bin"]
        .iter()
        .any(|base| name.eq_ignore_ascii_case(base) || name.eq_ignore_ascii_case(&format!("{base}.zst")))
}

fn selected(dir: &Path, slots: &[skyward_vab::Slot]) -> Vec<PathBuf> {
    slots.iter().map(|slot| dir.join(format!("modem{}", slot.suffix()))).collect()
}

/// Restore previous block-device limits after the script. Firmware is left
/// unmounted: its old cached FAT metadata must not be used after a write.
pub struct Prepared {
    #[cfg(target_os = "android")]
    readonly: Vec<PathBuf>,
}

#[cfg(target_os = "android")]
impl Drop for Prepared {
    fn drop(&mut self) {
        for path in &self.readonly {
            if let Err(error) = read_only(path, Some(true)) {
                let _ = skyward_sys::kmsg(&format!("modem: could not restore read-only on {}: {error:#}", path.display()));
            }
        }
    }
}

pub fn prepare(dir: &Path, slots: &[skyward_vab::Slot]) -> Result<Prepared> {
    let paths = selected(dir, slots);
    #[cfg(target_os = "android")]
    {
        // Check every target before changing any mount. Never stop firmware
        // consumers or use lazy/forced unmount to bypass a busy filesystem.
        let mut targets = Vec::new();
        for path in paths {
            if !path.try_exists()? {
                continue;
            }
            let device = skyward_block::devnum(&path)?;
            let points = skyward_block::mount::points_of(device)?;
            let holders = if points.is_empty() {
                Vec::new()
            } else {
                skyward_block::mount::holding(device)?
            };
            ensure!(
                holders.is_empty(),
                "{} firmware is still in use by {holders:?}; reboot recovery before flashing",
                path.display()
            );
            let readonly = read_only(&path, None)?;
            targets.push((path, device, unmount_order(points), readonly));
        }
        let mut prepared = Prepared { readonly: Vec::new() };
        for (path, device, points, readonly) in targets {
            for point in points {
                skyward_block::mount::umount(&point).with_context(|| format!("release modem firmware on {}", point.display()))?;
            }
            ensure!(
                skyward_block::mount::points_of(device)?.is_empty(),
                "{} still has mounted firmware aliases",
                path.display()
            );
            if readonly {
                prepared.readonly.push(path.clone());
                read_only(&path, Some(false)).with_context(|| format!("make {} writable for the installer", path.display()))?;
            }
            ensure!(!read_only(&path, None)?, "{} remains read-only", path.display());
            let _ = skyward_sys::kmsg(&format!("modem: {} is unmounted and writable", path.display()));
        }
        Ok(prepared)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = paths;
        anyhow::bail!("modem preparation is available only in Android recovery")
    }
}

#[cfg(any(target_os = "android", test))]
fn unmount_order(mut points: Vec<PathBuf>) -> Vec<PathBuf> {
    points.sort_by(|a, b| b.components().count().cmp(&a.components().count()).then_with(|| a.cmp(b)));
    points.dedup();
    points
}

#[cfg(target_os = "android")]
fn read_only(path: &Path, set: Option<bool>) -> Result<bool> {
    use std::os::fd::AsRawFd;
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    if let Some(on) = set {
        let value = libc::c_int::from(on);
        ensure!(
            unsafe { libc::ioctl(file.as_raw_fd(), 0x125d, &value) } == 0,
            "BLKROSET {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        );
    }
    let mut value: libc::c_int = 0;
    ensure!(
        unsafe { libc::ioctl(file.as_raw_fd(), 0x125e, &mut value) } == 0,
        "BLKROGET {}: {}",
        path.display(),
        std::io::Error::last_os_error()
    );
    Ok(value != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn package(entries: &[&str]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"fixture").unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn only_explicit_modem_firmware_entries_trigger_preparation() {
        for name in [
            "images/modem.img",
            "firmware-update/NON-HLOS.bin",
            "modem.img.zst",
            "MODEM.IMG",
            "NON-HLOS.bin.zst",
        ] {
            assert!(in_package(Cursor::new(package(&[name]))).unwrap(), "{name}");
        }
        for name in [
            "moden.img",
            "modemfirmware.img",
            "persist.img",
            "modem_a.img",
            "modem.img/",
            "modem.img.txt",
        ] {
            assert!(!in_package(Cursor::new(package(&[name]))).unwrap(), "{name}");
        }
    }

    #[test]
    fn ordinary_module_and_rom_packages_leave_modem_alone() {
        let zip = package(&[
            "META-INF/com/google/android/update-binary",
            "module.prop",
            "images/boot.img",
            "images/super.img.zst",
        ]);
        assert!(!in_package(Cursor::new(zip)).unwrap());
        assert!(in_package(Cursor::new(b"not a zip")).is_err());
        assert_eq!(
            selected(Path::new("/dev/block/by-name"), &[skyward_vab::Slot::B]),
            [PathBuf::from("/dev/block/by-name/modem_b")]
        );
    }

    #[test]
    fn nested_firmware_mounts_are_removed_before_their_parent() {
        let points = vec![
            "/firmware",
            "/vendor/firmware_mnt",
            "/vendor/firmware_mnt/extra",
            "/vendor/firmware",
            "/firmware",
        ];
        let order = unmount_order(points.into_iter().map(PathBuf::from).collect());
        assert_eq!(order[0], Path::new("/vendor/firmware_mnt/extra"));
        assert_eq!(order.last().unwrap(), Path::new("/firmware"));
        assert_eq!(order.len(), 4);
    }
}
