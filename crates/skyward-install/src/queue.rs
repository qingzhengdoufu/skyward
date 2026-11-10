//! ZIP queue rules shared by the UI and installer. No partition I/O here.

use crate::{package::{self, Package}, Outcome};
use anyhow::{ensure, Context, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

pub const LIMIT: usize = 10;

pub fn check_paths(paths: &[PathBuf]) -> Result<()> {
    ensure!(!paths.is_empty() && paths.len() <= LIMIT, "queue must contain 1..=10 ZIPs");
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        ensure!(path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("zip")), "queue accepts ZIP files only");
        let real = std::fs::canonicalize(path).with_context(|| format!("resolve {}", path.display()))?;
        ensure!(seen.insert(real), "duplicate ZIP in queue");
        check_script(path)?;
    }
    Ok(())
}

pub fn check_script(path: &Path) -> Result<()> {
    let file = File::open(path)?;
    ensure!(matches!(package::identify(BufReader::new(file))?, Package::Script), "queue accepts update-binary script ZIPs only; use single install for OTA, images or second systems");
    Ok(())
}

pub fn may_continue(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Ran { wipe_cache: false })
}

pub fn digest(path: &Path) -> Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    std::io::copy(&mut file, &mut hash)?;
    Ok(hash.finalize().into())
}

/// Execute in the reviewed order; a failed/refused/cancelled package never
/// permits the callback for a later package to run.
pub fn run(paths: &[PathBuf], mut install: impl FnMut(usize, &Path) -> Outcome) -> Outcome {
    let mut last = Outcome::Cancelled;
    for (index, path) in paths.iter().enumerate() {
        last = install(index, path);
        if !may_continue(&last) { break; }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Failure, Refusal};

    #[test]
    fn only_successful_scripts_without_deferred_wipes_continue() {
        assert!(may_continue(&Outcome::Ran { wipe_cache: false }));
        for outcome in [Outcome::Ran { wipe_cache: true }, Outcome::Cancelled,
            Outcome::Refused(Refusal::Unknown), Outcome::Failed(Failure::Script(1))] {
            assert!(!may_continue(&outcome));
        }
    }

    #[test]
    fn queue_stops_at_first_failure_and_keeps_order() {
        let paths = ["a.zip", "b.zip", "c.zip"].map(PathBuf::from);
        let mut ran = Vec::new();
        let end = run(&paths, |index, path| {
            ran.push(path.to_path_buf());
            if index == 1 { Outcome::Failed(Failure::Script(7)) }
            else { Outcome::Ran { wipe_cache: false } }
        });
        assert_eq!(ran, paths[..2]);
        assert_eq!(end, Outcome::Failed(Failure::Script(7)));
        let mut ran = Vec::new();
        run(&paths, |_, path| { ran.push(path.to_path_buf()); Outcome::Ran { wipe_cache: false } });
        assert_eq!(ran, paths);
    }

    #[test]
    fn preflight_rejects_duplicates_non_scripts_missing_and_excessive_paths() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        zip.start_file(package::UPDATE_BINARY, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"#!/sbin/sh\nexit 0\n").unwrap();
        zip.finish().unwrap();
        assert!(check_paths(std::slice::from_ref(&path)).is_ok());
        assert!(check_paths(&[]).is_err());
        assert!(check_paths(&vec![path.clone(); 11]).is_err());
        assert!(check_paths(&[path.clone(), path.clone()]).is_err());
        let other = dir.path().join("other.zip");
        std::fs::write(&other, b"not a script").unwrap();
        assert!(check_paths(&[path, other]).is_err());
        assert!(check_paths(&[dir.path().join("boot.img")]).is_err());
    }
}
