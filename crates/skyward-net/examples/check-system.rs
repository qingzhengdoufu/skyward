//! Query the official OTA service without downloading or installing a ROM.

fn main() -> anyhow::Result<()> {
    let agent = ureq::Agent::config_builder().http_status_as_error(false)
        .timeout_global(Some(std::time::Duration::from_secs(45))).build().new_agent();
    let installed = skyward_net::release::Installed {
        device: "miro".into(), version: "OS3.0.305.0.WOMCNXM".into(), android: "16".into(), unlocked: true,
    };
    let release = skyward_net::release::ask(&agent, "https://update.miui.com/updates/miotaV3.php", &installed)?;
    if let Some(release) = release {
        println!("{}: {}", release.version, release.name);
        println!("MD5: {}", release.md5);
        println!("Length: {}", skyward_net::release::length(&agent, &release.urls[0])?);
    } else {
        println!("No full OTA available");
    }
    Ok(())
}
