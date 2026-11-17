//! Bounded SNTP sampling. Two independent servers must agree before use.
use anyhow::{Context, Result, bail, ensure};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const EPOCH: u64 = 2_208_988_800;
const MIN_UNIX: u64 = 1_704_067_200;
const MAX_UNIX: u64 = 4_102_444_800;
const WAIT: Duration = Duration::from_secs(3);

/// Unix milliseconds at observation time. Plain SNTP is not authenticated;
/// nonce/source matching and agreement reduce accidental or single-source errors.
pub fn sample(servers: &[Ipv4Addr]) -> Result<u64> {
    let mut samples: Vec<(u64, Instant)> = Vec::new();
    for name in ["ntp.aliyun.com", "time.cloudflare.com", "ntp.tencent.com"] {
        let found = crate::dns::lookup(name, servers);
        let Ok(found) = found else { continue };
        let Some(address) = found.first() else { continue };
        let Ok(now) = ask(SocketAddr::from((*address, 123))) else {
            continue;
        };
        let at = Instant::now();
        for &(previous, observed) in &samples {
            let previous = previous + observed.elapsed().as_millis() as u64;
            if previous.abs_diff(now) <= 3000 {
                return Ok(now + at.elapsed().as_millis() as u64);
            }
        }
        samples.push((now, at));
    }
    bail!("network time unavailable or independent servers did not agree")
}

fn ask(address: SocketAddr) -> Result<u64> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.connect(address)?;
    socket.set_read_timeout(Some(WAIT))?;
    socket.set_write_timeout(Some(WAIT))?;
    let mut cookie = [0u8; 8];
    let got = unsafe { libc::getrandom(cookie.as_mut_ptr().cast(), cookie.len(), 0) };
    ensure!(got == cookie.len() as isize, "network time nonce unavailable");
    let mut request = [0u8; 48];
    request[0] = 0x23; // NTPv4 client.
    request[40..48].copy_from_slice(&cookie);
    let start = Instant::now();
    socket.send(&request).context("send network time request")?;
    let mut reply = [0u8; 513];
    let size = socket.recv(&mut reply).context("read network time reply")?;
    reply_ms(&reply[..size], cookie, start.elapsed())
}

fn timestamp(bytes: &[u8]) -> Result<u64> {
    let seconds = u32::from_be_bytes(bytes[..4].try_into()?) as u64;
    let fraction = u32::from_be_bytes(bytes[4..8].try_into()?) as u64;
    // Select era 0/1 by a fixed supported interval, never the incorrect local clock.
    let seconds = if seconds < EPOCH { seconds + (1u64 << 32) } else { seconds };
    let unix = seconds - EPOCH;
    ensure!((MIN_UNIX..MAX_UNIX).contains(&unix), "network time outside supported interval");
    Ok(unix * 1000 + ((fraction * 1000) >> 32))
}

fn reply_ms(reply: &[u8], cookie: [u8; 8], elapsed: Duration) -> Result<u64> {
    ensure!((48..=512).contains(&reply.len()), "invalid network time packet length");
    let header = reply[0];
    ensure!(
        header >> 6 != 3 && matches!((header >> 3) & 7, 3 | 4) && header & 7 == 4,
        "unsynchronized or invalid NTP server"
    );
    ensure!((1..=15).contains(&reply[1]), "NTP server refused synchronization");
    ensure!(reply[24..32] == cookie, "network time reply does not match request");
    ensure!(elapsed <= WAIT, "network time round trip too long");
    let received = timestamp(&reply[32..40])?;
    let sent = timestamp(&reply[40..48])?;
    let roundtrip = elapsed.as_millis() as u64;
    ensure!(
        sent >= received && sent - received <= roundtrip + 10,
        "invalid NTP server processing time"
    );
    Ok(sent + roundtrip.saturating_sub(sent - received) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(unix: u64) -> [u8; 48] {
        let mut out = [0; 48];
        out[0] = 0x24;
        out[1] = 2;
        out[24..32].copy_from_slice(&[7; 8]);
        let seconds = (unix + EPOCH) as u32;
        out[32..36].copy_from_slice(&seconds.to_be_bytes());
        out[40..44].copy_from_slice(&seconds.to_be_bytes());
        out
    }

    #[test]
    fn ntp_eras_and_roundtrip_are_independent_of_local_time() {
        for unix in [1_793_000_000, 2_100_000_000, 4_000_000_000] {
            assert_eq!(
                reply_ms(&packet(unix), [7; 8], Duration::from_millis(100)).unwrap(),
                unix * 1000 + 50
            );
        }
    }

    #[test]
    fn malformed_stale_unsynchronized_and_unreasonable_replies_are_refused() {
        let good = packet(1_793_000_000);
        assert!(reply_ms(&good[..47], [7; 8], Duration::ZERO).is_err());
        assert!(reply_ms(&good, [8; 8], Duration::ZERO).is_err());
        assert!(reply_ms(&good, [7; 8], Duration::from_secs(4)).is_err());
        for index in [0, 1] {
            let mut bad = good;
            bad[index] = 0;
            assert!(reply_ms(&bad, [7; 8], Duration::ZERO).is_err());
        }
        let mut bad = good;
        bad[0] |= 0xc0;
        assert!(reply_ms(&bad, [7; 8], Duration::ZERO).is_err());
        assert!(reply_ms(&packet(1_500_000_000), [7; 8], Duration::ZERO).is_err());
    }
}
