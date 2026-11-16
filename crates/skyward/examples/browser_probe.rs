//! Exercise the browser protocol without opening DRM or running recovery init.
#[path = "../src/cdp.rs"]
#[allow(dead_code)]
mod cdp;

use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    let target = std::env::args().nth(1);
    let mut page = cdp::Cdp::connect().context("attach existing Chromium")?;
    page.enable_downloads()?;
    for (index, height) in [2400, 1400, 2400].into_iter().enumerate() {
        page.viewport(1350, height)?;
        let title = format!("Skyward probe {index}");
        page.navigate(&format!("data:text/html,<meta charset=utf-8><title>{title}</title><body style='background:white;color:black'><h1>Skyward browser test</h1><p>Navigation and viewport capture</p><input id=q>"))?;
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let result = page.call("Runtime.evaluate", json!({"expression":"document.title","returnByValue":true}))?;
            if result["result"]["value"] == title {
                break;
            }
            ensure!(Instant::now() < deadline, "test document title never appeared");
            std::thread::sleep(Duration::from_millis(100));
        }
        let png = page.screenshot()?;
        ensure!(png.starts_with(b"\x89PNG\r\n\x1a\n") && png.len() >= 24, "invalid screenshot");
        let width = u32::from_be_bytes(png[16..20].try_into()?);
        let actual_height = u32::from_be_bytes(png[20..24].try_into()?);
        ensure!(width == 1350, "screenshot width {width}, expected 1350");
        println!(
            "LOCAL_PAGE_OK index={index} width={width} height={actual_height} bytes={}",
            png.len()
        );
        std::fs::write(format!("/tmp/skyward-browser-probe-{index}.png"), png)?;
    }
    drop(page);
    let mut page = cdp::Cdp::connect().context("reconnect to existing browser page")?;
    page.viewport(1350, 2400)?;
    println!("RECONNECT_OK");
    if let Some(url) = target {
        page.navigate(&url)?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let state = page.call("Runtime.evaluate", json!({"expression":"JSON.stringify({url:location.href,title:document.title,state:document.readyState,text:document.body?.innerText?.slice(0,200)})","returnByValue":true}))?;
            println!("NETWORK_PAGE {}", state["result"]["value"]);
            if state["result"]["value"]
                .as_str()
                .is_some_and(|value| value.contains("\"state\":\"complete\""))
            {
                break;
            }
            ensure!(Instant::now() < deadline, "network page did not finish loading");
            std::thread::sleep(Duration::from_secs(1));
        }
        std::fs::write("/tmp/skyward-browser-probe-network.png", page.screenshot()?)?;
    }
    Ok(())
}
