fn main() -> anyhow::Result<()> {
    let servers = std::env::args().skip(1).map(|ip| ip.parse()).collect::<Result<Vec<_>, _>>()?;
    println!("network_unix_ms={}", skyward_net::time::sample(&servers)?);
    Ok(())
}
