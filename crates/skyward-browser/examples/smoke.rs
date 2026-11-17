use anyhow::{Context, Result};
use skyward_browser::{protocol::Message, runtime, session::Browser};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let address = std::env::args().nth(1).unwrap_or_else(|| "aera://test".into());
    let prepared = runtime::prepare(Path::new("/tmp/skyward-browser-runtime.xz"))?;
    let mut browser = Browser::start(prepared, &["192.168.0.1".parse()?], (1440, 2492))?;
    let mut nav = Message::command(1);
    nav.text = address;
    browser.session.send(&nav)?;
    let until = Instant::now() + Duration::from_secs(45);
    let mut frames = 0;
    while Instant::now() < until {
        if !browser.running()? {
            anyhow::bail!("browser exited; inspect /tmp/skyward-browser.log");
        }
        while let Some(message) = browser.session.receive()? {
            if message.kind == 32 {
                let data = browser.session.copy_frame(&message)?;
                frames += 1;
                let unique = data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .take(500000)
                    .collect::<std::collections::HashSet<_>>()
                    .len();
                println!("frame {frames}, sequence {}, sampled colors {unique}", message.sequence);
                std::fs::write("/tmp/skyward-browser-frame.bgra", &data)?;
            } else {
                println!("event {} progress {} {}", message.kind, message.value, message.text);
            }
        }
        if frames >= 3 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    anyhow::ensure!(frames > 0, "no browser frame within timeout");
    std::fs::metadata("/tmp/skyward-browser-frame.bgra").context("no frame")?;
    Ok(())
}
