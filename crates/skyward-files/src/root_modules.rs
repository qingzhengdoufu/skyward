//! Offline Magisk/KernelSU/APatch module markers, never module scripts.
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::Read,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

const ROOTS: [&str; 3] = ["modules", "ksu/modules", "ap/modules"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub path: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub enabled: bool,
    pub removing: bool,
}

fn directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
}

pub fn read(base: &Path) -> Result<(String, Vec<Module>)> {
    if !base.exists() {
        return Ok((String::new(), Vec::new()));
    }
    ensure!(directory(base), "root storage is not a directory");
    let mut detected = Vec::new();
    for (name, file) in [("Magisk", "magisk/magisk"), ("KernelSU", "ksud"), ("APatch", "apd")] {
        if base.join(file).is_file() {
            detected.push(name);
        }
    }
    let mut modules = Vec::new();
    for root in ROOTS {
        let root = base.join(root);
        if !root.exists() {
            continue;
        }
        ensure!(directory(&root), "module root must not be a symlink");
        for entry in fs::read_dir(&root)? {
            let path = entry?.path();
            if !directory(&path) {
                continue;
            }
            let file = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path.join("module.prop"));
            let Ok(file) = file else { continue };
            let mut text = String::new();
            file.take(65537).read_to_string(&mut text)?;
            if text.len() > 65536 {
                continue;
            }
            let prop = |key: &str| {
                text.lines()
                    .filter_map(|l| l.split_once('='))
                    .find(|(k, _)| k.trim() == key)
                    .map(|(_, v)| v.trim().to_string())
                    .unwrap_or_default()
            };
            let id = prop("id");
            if id.is_empty() {
                continue;
            }
            let name = prop("name");
            modules.push(Module {
                path: path.to_string_lossy().into(),
                id: id.clone(),
                name: if name.is_empty() { id } else { name },
                version: prop("version"),
                author: prop("author"),
                enabled: !path.join("disable").exists(),
                removing: path.join("remove").exists(),
            });
            ensure!(modules.len() <= 1024, "too many root modules");
        }
    }
    modules.sort_by_key(|m| m.name.to_lowercase());
    Ok((detected.join(" / "), modules))
}

fn checked(base: &Path, path: &Path) -> Result<PathBuf> {
    ensure!(directory(base), "root storage unavailable");
    let relative = path.strip_prefix(base).context("module outside root storage")?;
    ensure!(ROOTS.iter().any(|r| relative.parent() == Some(Path::new(r))), "invalid module path");
    let leaf = relative.file_name().context("module has no name")?;
    ensure!(leaf != "." && leaf != "..", "invalid module name");
    let mut current = base.to_path_buf();
    for part in relative.components() {
        ensure!(matches!(part, std::path::Component::Normal(_)), "unsafe module path");
        current.push(part);
        ensure!(directory(&current), "module path changed or is a symlink");
    }
    ensure!(path.join("module.prop").is_file(), "not a root module");
    Ok(current)
}

pub fn marker(base: &Path, path: &Path, name: &str, present: bool) -> Result<()> {
    ensure!(matches!(name, "disable" | "remove"), "unknown module marker");
    let path = checked(base, path)?.join(name);
    if present {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o644)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path);
        match file {
            Ok(file) => {
                file.set_permissions(fs::Permissions::from_mode(0o644))?;
                file.sync_all()?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                ensure!(fs::symlink_metadata(&path)?.is_file(), "unsafe module marker")
            }
            Err(e) => return Err(e.into()),
        }
    } else if path.symlink_metadata().is_ok() {
        ensure!(fs::symlink_metadata(&path)?.is_file(), "unsafe module marker");
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markers_never_execute_or_delete_the_module() {
        let root = std::env::temp_dir().join(format!("skyward-module-{}", std::process::id()));
        fs::create_dir_all(root.join("modules/test")).unwrap();
        let path = root.join("modules/test");
        fs::write(path.join("module.prop"), "id=test\nname=Test\nversion=1\n").unwrap();
        fs::write(path.join("service.sh"), "exit 99").unwrap();
        marker(&root, &path, "disable", true).unwrap();
        assert!(!read(&root).unwrap().1[0].enabled);
        marker(&root, &path, "disable", false).unwrap();
        marker(&root, &path, "remove", true).unwrap();
        assert!(read(&root).unwrap().1[0].removing && path.join("service.sh").exists());
        assert!(marker(&root, Path::new("/data/system"), "remove", true).is_err());
        std::os::unix::fs::symlink(&path, root.join("modules/link")).unwrap();
        assert!(marker(&root, &root.join("modules/link"), "disable", true).is_err());
        assert_eq!(read(&root).unwrap().1.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
