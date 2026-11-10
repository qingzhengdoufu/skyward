//! Backups: what one holds, as the manifest in its folder records it, and
//! the files each of its parts is kept in.

pub mod archive;
pub mod image;
pub mod store;

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use skyward_files::Ownership;
use std::io::Write;
use std::path::Path;

/// The file in a backup's folder that records what the backup holds. It is
/// written last, so a folder with one holds a whole backup.
pub const MANIFEST: &str = "backup.toml";

/// Where backups go, from the top of the place they are kept in.
pub const FOLDER: &str = "Skyward/backups";

/// The layout of a backup this recovery writes and reads.
pub const FORMAT: u32 = 1;

/// What a backup can hold, in the order the backup page lists it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    /// The slot's boot images, and the partitions that check what the
    /// phone starts.
    Boot,
    /// The slot's partitions in super.
    System,
    /// The data partition's files, all but internal storage.
    Data,
    /// The baseband's settings with the phone's own identity, and the
    /// sensors' calibration.
    Efs,
}

impl Part {
    pub const ALL: [Part; 4] = [Part::Boot, Part::System, Part::Data, Part::Efs];
}

/// Claim a new backup directory before writing or cleaning up anything.
/// A same-second timestamp must never reuse an existing backup.
pub fn create_folder(path: &Path, ownership: Ownership) -> Result<()> {
    let parent = path.parent().context("backup folder needs a parent")?;
    skyward_files::make_folders(parent, ownership)?;
    skyward_files::make_new_folder(path, ownership)
}

pub fn check_efs_selection(selected: bool, partitions: &[String]) -> Result<()> {
    ensure!(!selected || !partitions.is_empty(), "EFS backup is unavailable: this device has no verified EFS partition list");
    Ok(())
}

/// What a backup holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    /// The phone it was made on, by its codename.
    pub device: String,
    /// When it was made, in Unix seconds.
    pub made: i64,
    /// The slot it was made from, by its suffix.
    pub slot: String,
    /// The system it was made from, as its version reads.
    pub system: Option<String>,
    /// With the data partition's files, the reference of the key the
    /// partition encrypts with: one made afresh, as a factory reset makes
    /// it, has another.
    pub data_key: Option<String>,
    pub items: Vec<Item>,
}

/// One part of what a backup holds: a partition's image, or the data
/// partition's files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub part: Part,
    /// The partition's name without its slot's suffix, or `data`.
    pub name: String,
    #[serde(flatten)]
    pub stored: store::Stored,
}

impl Manifest {
    /// The manifest in the backup's `folder`.
    pub fn read(folder: &Path) -> Result<Manifest> {
        let path = folder.join(MANIFEST);
        let text = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let manifest: Manifest = toml::from_str(&text).with_context(|| format!("read {}", path.display()))?;
        ensure!(manifest.format == FORMAT, "{} is in layout {}, which this recovery does not read", path.display(), manifest.format);
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<()> {
        use std::collections::HashSet;
        let plain = |name: &str| !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0']);
        ensure!(!self.items.is_empty(), "empty backup manifest");
        let mut names = HashSet::new();
        let mut files = HashSet::new();
        for item in &self.items {
            ensure!(plain(&item.name) && names.insert(item.name.as_str()), "invalid or duplicate partition name");
            ensure!(item.part != Part::Data || item.name == "data", "invalid data backup name");
            ensure!(!item.stored.files.is_empty(), "backup part has no files");
            ensure!(item.stored.sha256.len() == 64 && item.stored.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()), "invalid backup SHA-256");
            for name in &item.stored.files {
                ensure!(plain(name) && files.insert(name.as_str()), "unsafe or duplicate backup filename");
            }
        }
        Ok(())
    }

    /// Write it into the backup's `folder`, given what `ownership` says.
    pub fn write(&self, folder: &Path, ownership: Ownership) -> Result<()> {
        let path = folder.join(MANIFEST);
        self.validate()?;
        let text = toml::to_string(self).context("write the manifest")?;
        let mut file = skyward_files::make_file(&path, ownership)?;
        file.write_all(text.as_bytes()).with_context(|| format!("write {}", path.display()))?;
        file.sync_all().with_context(|| format!("write {}", path.display()))
    }

    /// Which of [`Part::ALL`] it holds.
    pub fn parts(&self) -> [bool; 4] {
        Part::ALL.map(|p| self.items.iter().any(|i| i.part == p))
    }

    /// The bytes its files take.
    pub fn size(&self) -> u64 {
        self.items.iter().map(|i| i.stored.stored).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use store::Stored;

    #[test]
    fn existing_backup_folder_is_not_claimed_or_modified() {
        let at = std::env::temp_dir().join(format!("skyward-folder-exclusive-{}", std::process::id()));
        create_folder(&at, Ownership::Kept).unwrap();
        std::fs::write(at.join("keep"), b"existing backup").unwrap();
        assert!(create_folder(&at, Ownership::Kept).is_err());
        assert_eq!(std::fs::read(at.join("keep")).unwrap(), b"existing backup");
        std::fs::remove_dir_all(at).unwrap();
    }

    #[test]
    fn empty_efs_config_cannot_produce_a_successful_efs_backup() {
        assert!(check_efs_selection(true, &[]).is_err());
        assert!(check_efs_selection(false, &[]).is_ok());
        assert!(check_efs_selection(true, &["device-verified-partition".into()]).is_ok());
    }

    #[test]
    fn a_manifest_reads_back_as_written() {
        let at = std::env::temp_dir().join(format!("skyward-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&at).unwrap();
        let stored = |name: &str, size| Stored { files: vec![format!("{name}.000")], size, stored: size / 2, compressed: true, sha256: "ab".repeat(32) };
        let m = Manifest {
            format: FORMAT,
            device: "ishtar".into(),
            made: 1_792_561_872,
            slot: "_b".into(),
            system: Some("OS3.0.305.0.WMACNXM".into()),
            data_key: Some("00112233445566778899aabbccddeeff".into()),
            items: vec![
                Item { part: Part::Boot, name: "boot".into(), stored: stored("boot", 201_326_592) },
                Item { part: Part::Data, name: "data".into(), stored: stored("data", 9_000_000_000) },
            ],
        };
        m.write(&at, Ownership::Kept).unwrap();
        assert_eq!(Manifest::read(&at).unwrap(), m);
        assert_eq!(m.parts(), [true, false, true, false]);
        assert_eq!(m.size(), 100_663_296 + 4_500_000_000);
        for name in ["../boot", "/boot", "..", "x\\boot"] {
            let mut bad = m.clone();
            bad.items[0].name = name.into();
            assert!(bad.validate().is_err());
            let mut bad = m.clone();
            bad.items[0].stored.files = vec![name.into()];
            assert!(bad.validate().is_err());
        }
        let mut bad = m.clone();
        bad.items.push(bad.items[0].clone());
        assert!(bad.validate().is_err());
        std::fs::remove_dir_all(&at).unwrap();
    }
}
