fn main()->anyhow::Result<()> {
    let path=std::env::args().nth(1).ok_or_else(||anyhow::anyhow!("runtime path required"))?;
    println!("{:?}",skyward_browser::runtime::verify(std::path::Path::new(&path))?);
    Ok(())
}
