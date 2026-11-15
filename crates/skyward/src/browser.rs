use anyhow::{Context, Result, ensure};
use skyward_config::DeviceConfig;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

pub fn prepare(cfg: &DeviceConfig) -> Result<tempfile::TempDir> {
    ensure!(cfg.device.codename == "miro", "browser GPU setup is not configured for this device");
    // A nonrecursive bind exposes vendor's own firmware behind stock init's
    // modem overmount, without removing that mount from the live recovery.
    let staging = tempfile::Builder::new().prefix("skyward-gpu-").tempdir_in("/tmp")?;
    skyward_block::mount::bind(Path::new("/vendor"), staging.path())?;
    let read = (|| -> Result<()> {
        fs::create_dir_all("/etc/firmware")?;
        fs::set_permissions("/etc/firmware", fs::Permissions::from_mode(0o755))?;
        for name in ["gen80000_sqe.fw", "gen80000_aqe.fw", "gen80000_gmu.bin"] {
            let from = staging.path().join("firmware").join(name);
            let to = Path::new("/etc/firmware").join(name);
            fs::copy(&from, &to).with_context(|| format!("prepare browser GPU firmware {name}"))?;
            fs::set_permissions(to, fs::Permissions::from_mode(0o644))?;
        }
        Ok(())
    })();
    let unmounted = skyward_block::mount::umount(staging.path());
    read?;
    unmounted?;
    crate::load_modules(cfg, &["msm_kgsl".into(), "governor_msm_adreno_tz".into()])?;
    skyward_sys::wait_file(Path::new("/dev/kgsl-3d0"), std::time::Duration::from_secs(5))?;
    let runtime = skyward_browser::runtime::prepare(Path::new(skyward_browser::RUNTIME_FILE))?;
    let font = runtime.path().join("usr/share/fonts/aera/MiSansVF.ttf");
    fs::copy("/skyward/MiSansVF.ttf", &font).context("prepare browser CJK font")?;
    fs::set_permissions(font, fs::Permissions::from_mode(0o644))?;
    Ok(runtime)
}
