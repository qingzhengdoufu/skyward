//! Bounded Android sparse decoding. DONT_CARE ranges are explicitly zeroed,
//! rather than leaving old filesystem or super metadata in the destination.

use anyhow::{ensure, Context, Result};
use std::io::{self, Read, Seek, SeekFrom};

pub const MAGIC: &[u8] = &[0x3a, 0xff, 0x26, 0xed];
const BUFFER: usize = 1 << 20;

#[derive(Debug)]
enum Data {
    Raw(u64),
    Fill([u8; 4]),
}

#[derive(Debug)]
struct Chunk {
    start: u64,
    end: u64,
    data: Data,
}

pub struct Reader<R> {
    source: R,
    chunks: Vec<Chunk>,
    size: u64,
    position: u64,
}

fn u16le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}

fn u32le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

impl<R: Read + Seek> Reader<R> {
    /// Validate all chunk bounds and any supplied CRCs before opening a target.
    pub fn open(mut source: R) -> Result<Self> {
        let length = source.seek(SeekFrom::End(0))?;
        source.seek(SeekFrom::Start(0))?;
        let mut head = [0; 28];
        source.read_exact(&mut head).context("read sparse header")?;
        ensure!(head.starts_with(MAGIC) && u16le(&head, 4) == 1 && u16le(&head, 6) == 0, "unsupported sparse format");
        let (file_header, chunk_header) = (u64::from(u16le(&head, 8)), u64::from(u16le(&head, 10)));
        let (block, blocks, count) = (u64::from(u32le(&head, 12)), u64::from(u32le(&head, 16)), u32le(&head, 20));
        ensure!(file_header >= 28 && file_header <= length && chunk_header >= 12, "invalid sparse header lengths");
        ensure!(block != 0 && block % 4 == 0 && blocks != 0, "invalid sparse block size or empty image");
        ensure!(count != 0 && count <= 262_144, "invalid or excessive sparse chunk count");
        let size = block.checked_mul(blocks).context("sparse size overflow")?;
        ensure!(size <= 2 * 1024_u64.pow(4), "sparse image exceeds 2 TiB");
        let mut chunks = Vec::new();
        let mut checks = Vec::new();
        let (mut offset, mut output) = (file_header, 0u64);
        for _ in 0..count {
            ensure!(offset.checked_add(chunk_header).is_some_and(|end| end <= length), "truncated sparse chunk header");
            source.seek(SeekFrom::Start(offset))?;
            let mut header = [0; 12];
            source.read_exact(&mut header)?;
            let bytes = u64::from(u32le(&header, 4)).checked_mul(block).context("sparse chunk overflow")?;
            let total = u64::from(u32le(&header, 8));
            ensure!(total >= chunk_header && offset.checked_add(total).is_some_and(|end| end <= length), "truncated sparse chunk");
            let payload = offset + chunk_header;
            let held = total - chunk_header;
            let end = output.checked_add(bytes).context("sparse output overflow")?;
            ensure!(end <= size, "sparse chunks exceed declared size");
            let data = match u16le(&header, 0) {
                0xcac1 => {
                    ensure!(bytes != 0 && held == bytes, "invalid sparse RAW length");
                    Some(Data::Raw(payload))
                }
                0xcac2 => {
                    ensure!(bytes != 0 && held == 4, "invalid sparse FILL length");
                    source.seek(SeekFrom::Start(payload))?;
                    let mut fill = [0; 4];
                    source.read_exact(&mut fill)?;
                    Some(Data::Fill(fill))
                }
                0xcac3 => {
                    ensure!(bytes != 0 && held == 0, "invalid sparse DONT_CARE length");
                    Some(Data::Fill([0; 4]))
                }
                0xcac4 => {
                    ensure!(bytes == 0 && held == 4, "invalid sparse CRC32 length");
                    source.seek(SeekFrom::Start(payload))?;
                    let mut crc = [0; 4];
                    source.read_exact(&mut crc)?;
                    checks.push((output, u32::from_le_bytes(crc)));
                    None
                }
                other => anyhow::bail!("unknown sparse chunk {other:#x}"),
            };
            if let Some(data) = data {
                chunks.push(Chunk { start: output, end, data });
            }
            output = end;
            offset += total;
        }
        ensure!(output == size && offset == length, "sparse block count or file length mismatch");
        let checksum = u32le(&head, 24);
        if checksum != 0 {
            checks.push((size, checksum));
        }
        let mut reader = Self { source, chunks, size, position: 0 };
        if !checks.is_empty() {
            let mut crc = crc32fast::Hasher::new();
            let mut buf = vec![0; BUFFER];
            for (until, expected) in checks {
                while reader.position < until {
                    let n = (until - reader.position).min(BUFFER as u64) as usize;
                    reader.read_exact(&mut buf[..n])?;
                    crc.update(&buf[..n]);
                }
                ensure!(crc.clone().finalize() == expected, "sparse CRC32 mismatch");
            }
            reader.position = 0;
        }
        Ok(reader)
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

impl<R: Read + Seek> Read for Reader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() || self.position >= self.size {
            return Ok(0);
        }
        let index = self.chunks.partition_point(|chunk| chunk.end <= self.position);
        let chunk = &self.chunks[index];
        let within = self.position - chunk.start;
        let n = out.len().min((chunk.end - self.position) as usize);
        match chunk.data {
            Data::Raw(offset) => {
                self.source.seek(SeekFrom::Start(offset + within))?;
                self.source.read_exact(&mut out[..n])?;
            }
            Data::Fill(fill) => {
                for (i, byte) in out[..n].iter_mut().enumerate() {
                    *byte = fill[(within as usize + i) % 4];
                }
            }
        }
        self.position += n as u64;
        Ok(n)
    }
}

impl<R> Seek for Reader<R> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let next = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(n) => i128::from(self.position) + i128::from(n),
            SeekFrom::End(n) => i128::from(self.size) + i128::from(n),
        };
        self.position = u64::try_from(next).map_err(|_| io::Error::other("invalid sparse seek"))?;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sample() -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        for n in [1u16, 0, 28, 12] { bytes.extend(n.to_le_bytes()); }
        for n in [4096u32, 3, 3, 0] { bytes.extend(n.to_le_bytes()); }
        for (kind, payload) in [(0xcac1u16, vec![0x42; 4096]), (0xcac2, vec![1, 2, 3, 4]), (0xcac3, vec![])] {
            bytes.extend(kind.to_le_bytes());
            bytes.extend(0u16.to_le_bytes());
            bytes.extend(1u32.to_le_bytes());
            bytes.extend((12 + payload.len() as u32).to_le_bytes());
            bytes.extend(payload);
        }
        bytes
    }

    #[test]
    fn raw_fill_skip_and_random_reads_expand_without_a_temporary_image() {
        let mut reader = Reader::open(Cursor::new(sample())).unwrap();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes.len(), 12288);
        assert_eq!(&bytes[..4096], &[0x42; 4096]);
        assert_eq!(&bytes[4096..8192], [1, 2, 3, 4].repeat(1024));
        assert_eq!(&bytes[8192..], &[0; 4096]);
        reader.seek(SeekFrom::Start(4097)).unwrap();
        let mut part = [0; 6];
        reader.read_exact(&mut part).unwrap();
        assert_eq!(part, [2, 3, 4, 1, 2, 3]);
        assert!(reader.seek(SeekFrom::Current(-5000)).is_err());
    }

    #[test]
    fn malformed_sizes_truncation_unknown_chunks_and_bad_checksums_are_rejected() {
        let valid = sample();
        for size in [0, 12, 28, valid.len() - 1] {
            assert!(Reader::open(Cursor::new(&valid[..size])).is_err());
        }
        for (offset, value) in [(4, 2u32), (12, 3), (16, 2), (20, u32::MAX), (28, 0xffff), (36, 12), (24, 1)] {
            let mut bad = valid.clone();
            bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(Reader::open(Cursor::new(bad)).is_err(), "offset {offset}");
        }
        let mut expanded = Vec::new();
        Reader::open(Cursor::new(&valid)).unwrap().read_to_end(&mut expanded).unwrap();
        let mut checked = valid;
        checked[24..28].copy_from_slice(&crc32fast::hash(&expanded).to_le_bytes());
        assert!(Reader::open(Cursor::new(checked)).is_ok());
    }
}
