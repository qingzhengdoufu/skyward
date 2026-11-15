//! Read-only validation of AERA's pinned runtime. Validation does not execute it.

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub const COMPRESSED_BYTES: u64 = 40_966_036;
pub const EXPANDED_BYTES: u64 = 210_785_213;
pub const MEMBERS: u32 = 445;
pub const COMPRESSED_SHA256: &str = "4e3941bd2bcbcc53e4d1bdc7641e7d7c7b16b848b8ba21bb934119118b37f197";
pub const EXPANDED_SHA256: &str = "dd9dce87850cd6ede704ca57b92d1ef0b76b28ceefc913a654278bb7bcc5b2db";

#[derive(Debug, PartialEq, Eq)]
pub struct Inventory {
    pub members: u32,
    pub expanded_bytes: u64,
}

fn path_safe(name: &str) -> bool {
    !name.is_empty() && name.len() < 240 && !name.contains(['\0', '\\']) && name.split('/').all(|p| !matches!(p, "" | "." | ".."))
}

pub fn verify(path: &Path) -> Result<Inventory> {
    let mut source = File::open(path).context("browser runtime is not installed")?;
    ensure!(source.metadata()?.len() == COMPRESSED_BYTES, "browser runtime length mismatch");
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = source.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    ensure!(
        format!("{:x}", digest.finalize()) == COMPRESSED_SHA256,
        "browser runtime SHA-256 mismatch"
    );
    source.seek(SeekFrom::Start(0))?;
    let stream = liblzma::stream::Stream::new_stream_decoder(128 << 20, 0)?;
    let mut expanded = liblzma::read::XzDecoder::new_stream(source, stream);
    let mut hashing = Hashing {
        source: &mut expanded,
        digest: Sha256::new(),
        bytes: 0,
    };
    let inventory = inspect(&mut hashing, MEMBERS, EXPANDED_BYTES)?;
    ensure!(
        hashing.bytes == EXPANDED_BYTES && format!("{:x}", hashing.digest.finalize()) == EXPANDED_SHA256,
        "expanded browser runtime mismatch"
    );
    Ok(inventory)
}

struct Hashing<'a, R> {
    source: &'a mut R,
    digest: Sha256,
    bytes: u64,
}
impl<R: Read> Read for Hashing<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let n = self.source.read(out)?;
        self.digest.update(&out[..n]);
        self.bytes += n as u64;
        Ok(n)
    }
}

/// Enforce the bounded AERAWEB1 format without trusting member paths or modes.
fn inspect(source: &mut impl Read, expected: u32, cap: u64) -> Result<Inventory> {
    let mut header = [0; 12];
    source.read_exact(&mut header)?;
    ensure!(&header[..8] == b"AERAWEB1", "unknown browser archive");
    let count = u32::from_le_bytes(header[8..].try_into()?);
    ensure!(
        count == expected && (1..=4096).contains(&count),
        "invalid browser archive member count"
    );
    let mut used = 12u64;
    let mut names = HashSet::new();
    let mut regular = HashSet::new();
    let mut aliases = Vec::new();
    let mut buffer = [0; 65536];
    for _ in 0..count {
        source.read_exact(&mut header)?;
        used = used.checked_add(12).context("archive overflow")?;
        let len = u16::from_le_bytes(header[..2].try_into()?) as usize;
        let mode = u16::from_le_bytes(header[2..4].try_into()?);
        let size = u64::from_le_bytes(header[4..].try_into()?);
        ensure!(
            (1..240).contains(&len) && matches!(mode, 0 | 0o644 | 0o755) && size <= 100 << 20,
            "invalid browser archive member"
        );
        let mut name = vec![0; len];
        source.read_exact(&mut name)?;
        used += len as u64;
        let name = String::from_utf8(name)?;
        ensure!(path_safe(&name) && names.insert(name.clone()), "unsafe or duplicate browser path");
        let padding = (4 - used % 4) % 4;
        source.read_exact(&mut buffer[..padding as usize])?;
        ensure!(
            buffer[..padding as usize].iter().all(|b| *b == 0),
            "invalid browser archive padding"
        );
        used = used
            .checked_add(padding)
            .and_then(|n| n.checked_add(size))
            .context("archive overflow")?;
        ensure!(used <= cap, "browser archive exceeds its bound");
        if mode == 0 {
            ensure!(size < 240, "oversized browser alias");
            let mut target = vec![0; size as usize];
            source.read_exact(&mut target)?;
            let target = String::from_utf8(target)?;
            ensure!(path_safe(&target), "unsafe browser alias target");
            aliases.push((name, target));
        } else {
            regular.insert(name);
            let mut left = size;
            while left > 0 {
                let n = left.min(buffer.len() as u64) as usize;
                source.read_exact(&mut buffer[..n])?;
                left -= n as u64;
            }
        }
    }
    for (name, target) in aliases {
        ensure!(regular.contains(&target), "browser alias must name a regular member: {name}");
    }
    for name in &names {
        for (at, _) in name.match_indices('/') {
            ensure!(!names.contains(&name[..at]), "browser member is also a parent directory");
        }
    }
    let mut extra = [0];
    ensure!(source.read(&mut extra)? == 0 && used == cap, "browser archive extent mismatch");
    Ok(Inventory {
        members: count,
        expanded_bytes: used,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(name: &str, mode: u16, data: &[u8]) -> Vec<u8> {
        members(&[(name, mode, data)])
    }
    fn members(entries: &[(&str, u16, &[u8])]) -> Vec<u8> {
        let mut out = b"AERAWEB1".to_vec();
        out.extend((entries.len() as u32).to_le_bytes());
        for (name, mode, data) in entries {
            out.extend((name.len() as u16).to_le_bytes());
            out.extend(mode.to_le_bytes());
            out.extend((data.len() as u64).to_le_bytes());
            out.extend(name.as_bytes());
            while !out.len().is_multiple_of(4) {
                out.push(0);
            }
            out.extend_from_slice(data);
        }
        out
    }
    #[test]
    fn bounded_archive_accepts_only_regular_safe_members() {
        let data = archive("usr/bin/worker", 0o755, b"ELF");
        assert_eq!(inspect(&mut &data[..], 1, data.len() as u64).unwrap().members, 1);
        for name in ["/data", "../data", "a/../b", "a//b", "a\\b"] {
            let bad = archive(name, 0o644, b"x");
            assert!(inspect(&mut &bad[..], 1, bad.len() as u64).is_err());
        }
        for mode in [0o4755, 0o777, 0o20644] {
            let bad = archive("worker", mode, b"x");
            assert!(inspect(&mut &bad[..], 1, bad.len() as u64).is_err());
        }
        assert!(inspect(&mut &data[..data.len() - 1], 1, data.len() as u64).is_err());
    }
    #[test]
    fn aliases_must_resolve_directly_to_safe_regular_members() {
        let good = members(&[("lib/alias", 0, b"lib/real"), ("lib/real", 0o755, b"ELF")]);
        inspect(&mut &good[..], 2, good.len() as u64).unwrap();
        for target in ["../outside", "/data", "lib/missing", "lib/alias", ""] {
            let bad = members(&[("lib/alias", 0, target.as_bytes()), ("lib/real", 0o755, b"ELF")]);
            assert!(inspect(&mut &bad[..], 2, bad.len() as u64).is_err());
        }
        let cycle = members(&[("a", 0, b"b"), ("b", 0, b"a")]);
        assert!(inspect(&mut &cycle[..], 2, cycle.len() as u64).is_err());
        for entries in [
            [("lib", 0o644, &b"x"[..]), ("lib/file", 0o644, &b"x"[..])],
            [("lib/file", 0o644, &b"x"[..]), ("lib/file", 0o644, &b"x"[..])],
        ] {
            let bad = members(&entries);
            assert!(inspect(&mut &bad[..], 2, bad.len() as u64).is_err());
        }
    }
    #[test]
    fn wrong_payload_is_rejected_before_decompression() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(verify(file.path()).is_err());
    }
}
