//! Opt-in AERA/WPE experiment. Uses the existing bounded ARP2 transport and jail.
use crate::browser::{Control, Event};
use anyhow::{Context, Result, bail, ensure};
use skyward_browser::{
    protocol::{self, Message},
    runtime,
    session::Browser,
};
use skyward_net::http::Servers;
use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub fn run(url: &str, servers: &Servers, stop: &AtomicBool, inputs: &Mutex<VecDeque<Control>>, tell: &dyn Fn(Event)) -> Result<()> {
    static OWNER: Mutex<()> = Mutex::new(());
    let _owner = OWNER.lock().expect("AERA owner is not poisoned");
    if stop.load(Ordering::Relaxed) {
        return Ok(());
    }
    prepare_gpu_firmware()?;
    // GPU setup is optional and never runs in recovery's startup path.
    if !Path::new("/dev/kgsl-3d0").exists() {
        let dirs = ["vendor_boot", "vendor_dlkm", "vendor", "system_dlkm"]
            .iter()
            .map(|name| skyward_modules::source_dir(name))
            .collect::<Result<Vec<_>>>()?;
        skyward_modules::load(&dirs, &["msm_kgsl".into()]).context("load the device's KGSL driver for AERA")?;
        for _ in 0..20 {
            if Path::new("/dev/kgsl-3d0").exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    // Do not acquire display DRM, change clocks or restart recovery services.
    for node in ["/dev/kgsl-3d0", "/dev/dma_heap/system"] {
        ensure!(Path::new(node).exists(), "AERA GPU prerequisite missing: {node}");
    }
    let dns = servers.lock().expect("DNS servers are not poisoned").clone();
    let root = runtime::prepare(Path::new(skyward_browser::RUNTIME_FILE))?;
    if stop.load(Ordering::Relaxed) {
        return Ok(());
    }
    let viewport_wait = Instant::now();
    let mut size = loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut queue = inputs.lock().expect("browser controls are not poisoned");
        if let Some(index) = queue.iter().position(|command| matches!(command, Control::Viewport { .. }))
            && let Some(Control::Viewport { width, height }) = queue.remove(index)
        {
            break (width, height);
        }
        drop(queue);
        ensure!(
            viewport_wait.elapsed() < Duration::from_secs(5),
            "native browser viewport was not provided"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let backing = size;
    let mut browser = Browser::start(root, &dns, backing)?;
    navigate(&browser, url)?;
    let mut zoom = 100.0f64;
    let mut latest = None;
    let mut changed = false;
    let mut publish_at = Instant::now();
    let mut frames = 0u64;
    let measured_at = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        ensure!(browser.running()?, "AERA worker exited; see /tmp/skyward-browser.log");
        let commands = std::mem::take(&mut *inputs.lock().expect("browser controls are not poisoned"));
        for command in commands {
            match command {
                Control::Touch { phase, x, y } => touch(&browser, phase, x, y, backing)?,
                Control::Tap { x, y } => {
                    touch(&browser, 0, x, y, backing)?;
                    touch(&browser, 2, x, y, backing)?;
                }
                Control::Viewport { width, height } => {
                    ensure!(
                        (1..=4096).contains(&width) && (1..=4096).contains(&height),
                        "AERA viewport outside bound"
                    );
                    size = (width, height);
                    changed = true;
                }
                Control::Text { text } => {
                    // ARP2 transports Unicode key values, not JavaScript or shell text.
                    for ch in text.chars().take(2048) {
                        let mut message = Message::command(9);
                        message.value = ch as u32;
                        browser.session.send(&message)?;
                    }
                }
                Control::Key { value } => {
                    ensure!(matches!(value, 8 | 13), "unsupported AERA editing key");
                    let mut message = Message::command(9);
                    message.value = value;
                    browser.session.send(&message)?;
                }
                Control::Navigate { url } => navigate(&browser, &skyward_net::web::full_url(&url))?,
                Control::Reload => browser.session.send(&Message::command(4))?,
                Control::Volume { percent } => {
                    let mut message = Message::command(17);
                    message.value = u32::from(percent.min(100));
                    browser.session.send(&message)?;
                }
                Control::Pinch { scale } => {
                    ensure!(scale.is_finite() && scale > 0.0, "invalid AERA zoom");
                    zoom = (zoom * scale).clamp(50.0, 300.0);
                    let mut message = Message::command(12);
                    message.value = zoom.round() as u32;
                    browser.session.send(&message)?;
                }
                Control::Scroll { dy } => {
                    let (start, end) = if dy < 0 { (800, 250) } else { (250, 800) };
                    touch(&browser, 0, 500, start, backing)?;
                    for step in 1..=8 {
                        touch(
                            &browser,
                            1,
                            500,
                            (i32::from(start) + (i32::from(end) - i32::from(start)) * step / 8) as u16,
                            backing,
                        )?;
                    }
                    touch(&browser, 2, 500, end, backing)?;
                }
            }
        }
        // Bounded work per turn and a single newest frame; the renderer cannot
        // grow an unbounded queue while recovery is doing other work.
        for _ in 0..8 {
            let Some(message) = browser.session.receive()? else {
                break;
            };
            match message.kind {
                32 => {
                    ensure!(
                        (message.x as u32, message.y as u32) == backing,
                        "native browser frame differs from viewport"
                    );
                    latest = Some(browser.session.copy_frame(&message)?);
                    changed = true;
                }
                33 => {
                    if message.text.starts_with("https://") || message.text.starts_with("http://") {
                        tell(Event::Navigated(protocol::address(&message.text)?));
                    }
                }
                34 if matches!(message.text.as_str(), "Load request cancelled" | "Operation was cancelled")
                    || message.text.contains("creating the snapshot") =>
                {
                    super::browser::note(&format!("AERA transient page event: {}", message.text));
                }
                34 => bail!("AERA worker: {}", message.text),
                35 => tell(Event::Focus(true)),
                36 => tell(Event::Focus(false)),
                // Downloads belong to AERA's private /data tree in this test.
                // Do not interpret worker strings as host paths or installer commands.
                37..=43 => super::browser::note(&format!("AERA event {} #{}: {}", message.kind, message.sequence, message.text)),
                _ => {}
            }
        }
        ensure!(
            latest.is_some() || measured_at.elapsed() < Duration::from_secs(30),
            "AERA did not produce a frame within 30 seconds; see /tmp/skyward-browser.log"
        );
        if changed
            && Instant::now() >= publish_at
            && let Some(bgra) = &latest
        {
            let rgba = skyward_browser::session::rgba_sized(bgra, backing.0, backing.1, size.0, size.1)?;
            tell(Event::Pixels {
                width: size.0,
                height: size.1,
                rgba,
            });
            frames += 1;
            if frames == 1 || frames.is_multiple_of(120) {
                super::browser::note(&format!(
                    "AERA published {frames} frames in {} ms, native backing {}x{}, output {}x{} (not measured panel FPS)",
                    measured_at.elapsed().as_millis(),
                    backing.0,
                    backing.1,
                    size.0,
                    size.1
                ));
            }
            publish_at = Instant::now() + Duration::from_micros(8333);
            changed = false;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    Ok(())
}

fn prepare_gpu_firmware() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // A non-recursive alias reveals vendor's original firmware hidden by the
    // modem bind mount. Copy only this GPU's files to RAM; system stays read-only.
    let alias = tempfile::Builder::new().prefix("skyward-gpu-vendor-").tempdir_in("/tmp")?;
    skyward_block::mount::bind(Path::new("/vendor"), alias.path())?;
    let result = (|| -> Result<()> {
        let target = Path::new("/lib/firmware");
        std::fs::create_dir_all(target)?;
        for name in ["gen80000_sqe.fw", "gen80000_aqe.fw", "gen80000_gmu.bin", "gen80600_gmu.bin"] {
            let source = alias.path().join("firmware").join(name);
            let metadata = std::fs::symlink_metadata(&source)?;
            ensure!(
                metadata.is_file() && metadata.len() > 0 && metadata.len() <= 2 << 20,
                "invalid GPU firmware {name}"
            );
            std::fs::copy(source, target.join(name))?;
            std::fs::set_permissions(target.join(name), std::fs::Permissions::from_mode(0o644))?;
        }
        Ok(())
    })();
    let unmounted = skyward_block::mount::umount(alias.path());
    if unmounted.is_err() {
        let _ = alias.keep();
    }
    result?;
    unmounted
}

fn navigate(browser: &Browser, url: &str) -> Result<()> {
    let mut message = Message::command(1);
    message.text = protocol::address(url).context("AERA address")?;
    browser.session.send(&message)
}

fn touch(browser: &Browser, phase: u8, x: u16, y: u16, backing: (u32, u32)) -> Result<()> {
    ensure!(phase <= 2, "invalid AERA touch phase");
    let mut message = Message::command(6 + u32::from(phase));
    message.x = (u32::from(x.min(999)) * (backing.0 / 4) / 1000) as i32;
    message.y = (u32::from(y.min(999)) * (backing.1 / 4) / 1000) as i32;
    browser.session.send(&message)
}
