//! File-based simg2img for install scripts. Validate the sparse map before
//! opening the output; never create a missing path below /dev.

use super::Io;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom};
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

pub(super) fn run(args: &[OsString], io: &mut Io) -> i32 {
    let result = (|| -> anyhow::Result<()> {
        anyhow::ensure!(args.len() == 2, "usage: simg2img SPARSE_IMAGE OUTPUT (one input file)");
        let input = Path::new(&args[0]);
        let output = Path::new(&args[1]);
        let file = File::open(input)?;
        let source = file.metadata()?;
        let mut expanded = crate::sparse::Reader::open(file)?;
        let existing = match std::fs::metadata(output) {
            Ok(metadata) => Some(metadata),
            Err(e) if e.kind() == io::ErrorKind::NotFound && !output.starts_with("/dev") => None,
            Err(e) => return Err(e.into()),
        };
        if let Some(target) = &existing {
            use std::os::unix::fs::MetadataExt;
            anyhow::ensure!(source.dev() != target.dev() || source.ino() != target.ino(), "input and output are the same file");
            anyhow::ensure!(target.is_file() || target.file_type().is_block_device(), "output is not a regular file or block device");
        }
        let block = existing.as_ref().is_some_and(|m| m.file_type().is_block_device());
        let mut out = OpenOptions::new().write(true).create(existing.is_none()).truncate(false).open(output)?;
        if block {
            let capacity = out.seek(SeekFrom::End(0))?;
            anyhow::ensure!(expanded.size() <= capacity, "expanded sparse image exceeds target capacity");
            out.seek(SeekFrom::Start(0))?;
        } else {
            out.set_len(0)?;
        }
        let copied = io::copy(&mut expanded, &mut out)?;
        anyhow::ensure!(copied == expanded.size(), "sparse image ended early");
        out.sync_all()?;
        Ok(())
    })();
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(io.errors, "simg2img: {error:#}");
            1
        }
    }
}
