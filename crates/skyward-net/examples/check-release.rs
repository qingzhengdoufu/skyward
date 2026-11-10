//! Inspect a saved GitHub API response using the recovery's trust policy.

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| anyhow::anyhow!("usage: check-release RESPONSE.json"))?;
    let response = std::fs::read_to_string(path)?;
    let newest = skyward_net::github::trusted_newest(&response, "miro", "0.2.3")?;
    let build = newest.build.ok_or_else(|| anyhow::anyhow!("no miro image"))?;
    println!("verified signed release {}, image {}, {} bytes, sha256 {}", newest.version, build.name, build.size, build.sha256);
    Ok(())
}
