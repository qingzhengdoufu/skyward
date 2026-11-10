//! Start an offline ADSP without the audio loader's device-tree state gate.

use anyhow::{ensure, Context, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum Started {
    AlreadyRunning,
    Requested,
}

/// This never stops or restarts a processor, changes its firmware selection,
/// or writes the audio driver's SSR switch. Firmware must already be mounted.
pub fn start_adsp(at: &Path, firmware_dir: &Path) -> Result<Started> {
    let name = fs::read_to_string(at.join("name")).context("read the remote processor's name")?;
    ensure!(name.trim() == "adsp" || name.trim().ends_with(".remoteproc-adsp"), "not the ADSP: {}", name.trim());
    let state_path = at.join("state");
    let read_state = || fs::read_to_string(&state_path).context("read the ADSP state");
    let state = read_state()?;
    if state.trim() == "running" {
        return Ok(Started::AlreadyRunning);
    }
    ensure!(state.trim() == "offline", "leave ADSP state {} unchanged", state.trim());
    let firmware = fs::read_to_string(at.join("firmware")).context("read the ADSP firmware selection")?;
    ensure!(firmware.trim() == "adsp.mdt", "leave unfamiliar ADSP firmware {} unchanged", firmware.trim());
    fs::File::open(firmware_dir.join(firmware.trim())).context("ADSP firmware is not available yet")?;
    // Recheck after the file lookup: a processor booting on its own is left alone.
    let state = read_state()?;
    if state.trim() == "running" {
        return Ok(Started::AlreadyRunning);
    }
    ensure!(state.trim() == "offline", "leave ADSP state {} unchanged", state.trim());
    fs::write(&state_path, "start").context("start the offline ADSP through remoteproc")?;
    Ok(Started::Requested)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processor(state: &str, name: &str, firmware: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (file, text) in [("name", name), ("state", state), ("firmware", firmware), ("adsp.mdt", "test firmware")] {
            fs::write(dir.path().join(file), text).unwrap();
        }
        dir
    }

    #[test]
    fn starts_only_the_offline_adsp() {
        let dir = processor("offline\n", "3000000.remoteproc-adsp\n", "adsp.mdt\n");
        assert_eq!(start_adsp(dir.path(), dir.path()).unwrap(), Started::Requested);
        assert_eq!(fs::read_to_string(dir.path().join("state")).unwrap(), "start");
    }

    #[test]
    fn a_running_adsp_needs_no_write_or_firmware_lookup() {
        let dir = processor("running\n", "adsp\n", "adsp.mdt\n");
        fs::remove_file(dir.path().join("adsp.mdt")).unwrap();
        assert_eq!(start_adsp(dir.path(), dir.path()).unwrap(), Started::AlreadyRunning);
        assert_eq!(fs::read_to_string(dir.path().join("state")).unwrap(), "running\n");
    }

    #[test]
    fn no_stop_restart_or_guessing_an_unknown_state() {
        for state in ["crashed", "suspended", "attached", "detached", "invalid", "starting", ""] {
            let dir = processor(state, "3000000.remoteproc-adsp", "adsp.mdt");
            assert!(start_adsp(dir.path(), dir.path()).is_err());
            assert_eq!(fs::read_to_string(dir.path().join("state")).unwrap(), state);
        }
    }

    #[test]
    fn leaves_other_processors_and_firmware_selections_alone() {
        for (name, firmware) in [("4080000.remoteproc-mss", "adsp.mdt"), ("adsp", "../adsp.mdt"), ("adsp", "other.mdt")] {
            let dir = processor("offline", name, firmware);
            assert!(start_adsp(dir.path(), dir.path()).is_err());
            assert_eq!(fs::read_to_string(dir.path().join("state")).unwrap(), "offline");
        }
    }

    #[test]
    fn missing_firmware_does_not_start_the_processor() {
        let dir = processor("offline", "adsp", "adsp.mdt");
        fs::remove_file(dir.path().join("adsp.mdt")).unwrap();
        assert!(start_adsp(dir.path(), dir.path()).is_err());
        assert_eq!(fs::read_to_string(dir.path().join("state")).unwrap(), "offline");
    }
}
