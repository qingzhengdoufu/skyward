fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| anyhow::anyhow!("runtime path required"))?;
    println!("{:?}", skyward_browser::runtime::verify(std::path::Path::new(&path))?);
    if let Some(font) = std::env::args().nth(2) {
        let root = skyward_browser::runtime::prepare_with_font(std::path::Path::new(&path), std::path::Path::new(&font))?;
        let installed = root.path().join("usr/share/fonts/skyward/MiSansVF.ttf");
        anyhow::ensure!(std::fs::read(installed)? == std::fs::read(font)?, "font bytes differ");
        println!("Verified complete extraction and Chinese font integration");
    }
    Ok(())
}
