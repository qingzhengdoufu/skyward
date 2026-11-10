//! The commands install scripts run, run as the scripts run them: by name,
//! from a folder of links to this binary on their PATH, in a pipe.

use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

/// Bytes that do not repeat, so they keep their size compressed.
fn noise(len: usize) -> Vec<u8> {
    let mut state = 0x6a09_e667_f3bc_c908u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

/// A ROM's zip, with its install script and its super image as zstd
/// compressed it, stored as such zips store it.
fn rom(dir: &Path, super_zst: &[u8]) -> PathBuf {
    rom_with_script(dir, super_zst, b"#!/sbin/sh\n")
}

fn rom_with_script(dir: &Path, super_zst: &[u8], updater: &[u8]) -> PathBuf {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let script = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    w.start_file("META-INF/com/google/android/update-binary", script).unwrap();
    w.write_all(updater).unwrap();
    w.start_file("images/super.img.zst", SimpleFileOptions::default().compression_method(CompressionMethod::Stored)).unwrap();
    w.write_all(super_zst).unwrap();
    let path = dir.join("rom.zip");
    fs::write(&path, w.finish().unwrap().into_inner()).unwrap();
    path
}

/// Run `script` with sh, the commands linked to this binary on its PATH
/// as the ramdisk links them, and `ZIPFILE` and `OUT` set.
fn script(dir: &Path, script: &str, zip: &Path, out: &Path) -> ExitStatus {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for command in skyward_config::ramdisk::COMMANDS {
        let link = bin.join(Path::new(command).file_name().unwrap());
        if !link.exists() {
            symlink(env!("CARGO_BIN_EXE_skyward"), link).unwrap();
        }
    }
    Command::new("sh")
        .args(["-c", script])
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("ZIPFILE", zip)
        .env("OUT", out)
        .status()
        .unwrap()
}

/// The two ways ROMs' scripts pour their super image out of their zip.
const FLASH_SUPER: [&str; 2] = [
    r#"7za e -so "$ZIPFILE" "images/super.img.zst" | zstd -c -d > "$OUT""#,
    r#"unzip -p "$ZIPFILE" images/super.img.zst | zstd -c -d > "$OUT""#,
];

#[test]
fn a_rom_s_script_pours_its_super_image_out_of_the_zip_through_zstd() {
    let dir = tempfile::tempdir().unwrap();
    let image = noise(8 << 20);
    let zip = rom(dir.path(), &zstd::encode_all(image.as_slice(), 3).unwrap());
    let out = dir.path().join("super");
    for flash in FLASH_SUPER {
        assert!(script(dir.path(), flash, &zip, &out).success(), "{flash}");
        assert!(fs::read(&out).unwrap() == image, "{flash}");
        fs::remove_file(&out).unwrap();
    }
}

#[test]
fn a_super_image_cut_short_in_the_zip_fails_the_pipe() {
    let dir = tempfile::tempdir().unwrap();
    let whole = zstd::encode_all(noise(1 << 20).as_slice(), 3).unwrap();
    let zip = rom(dir.path(), &whole[..whole.len() / 2]);
    for flash in FLASH_SUPER {
        assert!(!script(dir.path(), flash, &zip, &dir.path().join("super")).success(), "{flash}");
    }
}

fn sparse(raw: &[u8]) -> Vec<u8> {
    assert_eq!(raw.len() % 4096, 0);
    let mut out = vec![0x3a, 0xff, 0x26, 0xed];
    for n in [1u16, 0, 28, 12] { out.extend(n.to_le_bytes()); }
    for n in [4096u32, raw.len() as u32 / 4096, 1, 0] { out.extend(n.to_le_bytes()); }
    out.extend(0xcac1u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend((raw.len() as u32 / 4096).to_le_bytes());
    out.extend((12 + raw.len() as u32).to_le_bytes());
    out.extend(raw);
    out
}

#[test]
fn a_script_extracts_decompresses_and_expands_a_sparse_partition_image() {
    let dir = tempfile::tempdir().unwrap();
    let image = noise(2 << 20);
    let packed = sparse(&image);
    let zip = rom(dir.path(), &zstd::encode_all(packed.as_slice(), 3).unwrap());
    let output = dir.path().join("partition-file");
    for extract in ["7za e -so", "unzip -p"] {
        let command = format!(r#"{extract} "$ZIPFILE" images/super.img.zst | zstd -dc > "$OUT.sparse" && simg2img "$OUT.sparse" "$OUT""#);
        assert!(script(dir.path(), &command, &zip, &output).success());
        assert_eq!(fs::read(&output).unwrap(), image);
    }
    let pipe = r#"unzip -p "$ZIPFILE" images/super.img.zst | zstd -dc | simg2img - "$OUT""#;
    assert!(script(dir.path(), pipe, &zip, &output).success());
    assert_eq!(fs::read(&output).unwrap(), image);
}

#[test]
fn malformed_sparse_and_same_input_output_do_not_destroy_the_output() {
    let dir = tempfile::tempdir().unwrap();
    let zip = rom(dir.path(), b"unused");
    let output = dir.path().join("partition-file");
    fs::write(&output, b"existing partition bytes").unwrap();
    fs::write(output.with_extension("bad"), b"bad sparse image").unwrap();
    assert!(!script(dir.path(), r#"simg2img "$OUT.bad" "$OUT""#, &zip, &output).success());
    assert_eq!(fs::read(&output).unwrap(), b"existing partition bytes");
    let valid = sparse(&vec![0x53; 4096]);
    fs::write(&output, &valid).unwrap();
    assert!(!script(dir.path(), r#"simg2img "$OUT" "$OUT""#, &zip, &output).success());
    assert_eq!(fs::read(&output).unwrap(), valid);
    assert!(!script(dir.path(), "simg2img", &zip, &output).success());
}

#[test]
fn script_tools_are_dispatched_as_commands_without_starting_recovery_ui() {
    let dir = tempfile::tempdir().unwrap();
    let zip = rom(dir.path(), b"unused");
    for command in ["avbctl", "resetprop --help", "zstd --help", "unzip --help"] {
        let status = script(dir.path(), command, &zip, &dir.path().join("unused"));
        if command == "avbctl" { assert_eq!(status.code(), Some(64)); }
        else { assert!(status.success(), "{command}"); }
    }
}

#[test]
fn update_binary_receives_api_pipe_and_zip_path_then_flashes_a_regular_file() {
    let dir = tempfile::tempdir().unwrap();
    let image = noise(4096 * 17);
    let packed = sparse(&image);
    let updater = br#"#!/sbin/sh
set -eu
[ "$1" = 3 ] || exit 64
PIPEFD="$2"
ZIPFILE="$3"
eval "printf 'ui_print Starting image install\\n' >&$PIPEFD"
unzip -p "$ZIPFILE" images/super.img.zst | zstd -dc > "$OUT.sparse"
simg2img "$OUT.sparse" "$OUT"
eval "printf 'set_progress 1\\nui_print Image complete\\n' >&$PIPEFD"
"#;
    let zip = rom_with_script(dir.path(), &zstd::encode_all(packed.as_slice(), 3).unwrap(), updater);
    assert_eq!(skyward_install::package::identify(Cursor::new(fs::read(&zip).unwrap())).unwrap(), skyward_install::package::Package::Script);
    let mut archive = zip::ZipArchive::new(File::open(&zip).unwrap()).unwrap();
    let updater_path = dir.path().join("update-binary");
    let mut saved = File::create(&updater_path).unwrap();
    std::io::copy(&mut archive.by_name(skyward_install::package::UPDATE_BINARY).unwrap(), &mut saved).unwrap();
    drop(saved);
    let output = dir.path().join("simulated-system");
    // The outer shell gives fd 3 to the updater, as recovery gives its pipe.
    let command = format!(r#"sh '{}' 3 3 "$ZIPFILE" 3> "$OUT.ui""#, updater_path.display());
    assert!(script(dir.path(), &command, &zip, &output).success());
    assert_eq!(fs::read(&output).unwrap(), image);
    let lines = fs::read_to_string(dir.path().join("simulated-system.ui")).unwrap();
    let requests: Vec<_> = lines.lines().filter_map(skyward_install::script::parse).collect();
    assert_eq!(requests, [
        skyward_install::script::Request::Print("Starting image install".into()),
        skyward_install::script::Request::SetProgress(1.0),
        skyward_install::script::Request::Print("Image complete".into()),
    ]);
}
