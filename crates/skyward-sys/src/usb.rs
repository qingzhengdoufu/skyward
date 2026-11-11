//! The functions the USB port offers, which init sets up as
//! `sys.usb.config` asks: adb while the recovery waits, with files over MTP
//! beside it while file transfer is on.

use crate::{prop, set_prop, wait_prop};
use anyhow::{ensure, Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static MTP: AtomicBool = AtomicBool::new(false);

/// The gadget controller disappears while a dual-role port serves an OTG
/// peripheral. That is not a failed recovery or a connected adb host.
pub fn gadget_state(controllers: &Path, controller: Option<&str>) -> Result<Option<String>> {
    let Some(controller) = controller.filter(|name| !name.is_empty()) else { return Ok(None) };
    let path = controllers.join(controller).join("state");
    match std::fs::read_to_string(&path) {
        Ok(state) => Ok(Some(state.trim().to_string())),
        Err(error) if matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ENODEV | libc::ENXIO)) => Ok(None),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

/// What the port offers while nothing else has it.
pub fn resting() -> &'static str {
    if MTP.load(Ordering::Relaxed) {
        "mtp,adb"
    } else {
        "adb"
    }
}

/// Whether MTP goes beside adb while the port rests: from when its function
/// has its descriptors until it goes.
pub fn set_mtp(on: bool) {
    MTP.store(on, Ordering::Relaxed);
}

/// The functions a configuration names, in the order it names them.
fn functions_of(config: &str) -> Vec<&str> {
    config.split(',').filter(|f| !f.is_empty()).collect()
}

/// Ask init for `config` and wait until it is in place. Going to "none"
/// also waits for adbd to be gone, so its function is free.
pub fn set(config: &str, timeout: Duration) -> Result<()> {
    if config != "none" && prop("sys.usb.state").as_deref() == Some(config) {
        return Ok(());
    }
    set_prop("sys.usb.config", config)?;
    wait_prop("sys.usb.state", config, timeout)?;
    if config == "none" {
        let start = Instant::now();
        while !matches!(prop("init.svc.adbd").as_deref(), None | Some("stopped")) {
            ensure!(start.elapsed() < timeout, "adbd still runs {} ms after the port went to none", timeout.as_millis());
            thread::sleep(Duration::from_millis(10));
        }
    }
    Ok(())
}

/// Hand the port to `config` through "none", which stops the programs
/// serving it and takes every function off; init then sets the port up
/// once the programs behind the new configuration are ready.
///
/// A port already serving `config` is left alone: going through "none" for
/// nothing stops adbd, and every adb session the computer had open dies with
/// it. The same goes for one that only gains a function: `adb` and
/// `mtp,adb` both serve the adb function, so turning file transfer on adds a
/// function beside it rather than replacing what is there, and init is asked
/// for the new configuration directly. Only a function that has to come off
/// needs the port taken down first.
pub fn switch(config: &str, timeout: Duration) -> Result<()> {
    let serving = prop("sys.usb.state").unwrap_or_default();
    if serving == config {
        return Ok(());
    }
    let (have, want) = (functions_of(&serving), functions_of(config));
    // Every function the port serves is served by the one asked for: nothing
    // has to come off, so nothing the computer holds has to be interrupted.
    if have.iter().all(|f| want.contains(f)) {
        return set(config, timeout);
    }
    set("none", timeout)?;
    set(config, timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_otg_role_switch_can_remove_and_restore_the_gadget_controller() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(gadget_state(root.path(), None).unwrap(), None);
        assert_eq!(gadget_state(root.path(), Some("")).unwrap(), None);
        assert_eq!(gadget_state(root.path(), Some("a600000.dwc3")).unwrap(), None);
        let controller = root.path().join("a600000.dwc3");
        std::fs::create_dir(&controller).unwrap();
        let state = controller.join("state");
        std::fs::write(&state, "configured\n").unwrap();
        assert_eq!(gadget_state(root.path(), Some("a600000.dwc3")).unwrap().as_deref(), Some("configured"));
        std::fs::remove_file(&state).unwrap();
        std::fs::remove_dir(&controller).unwrap();
        assert_eq!(gadget_state(root.path(), Some("a600000.dwc3")).unwrap(), None);
        std::fs::create_dir(&controller).unwrap();
        std::fs::write(&state, "not attached\n").unwrap();
        assert_eq!(gadget_state(root.path(), Some("a600000.dwc3")).unwrap().as_deref(), Some("not attached"));
    }

    #[test]
    fn unrelated_gadget_read_errors_are_not_reported_as_a_disconnected_cable() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("controller/state")).unwrap();
        assert!(gadget_state(root.path(), Some("controller")).is_err());
    }
}
