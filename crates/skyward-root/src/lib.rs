//! On-demand KernelSU 6.6 installer and existing-manager module installation.
//! No payloads or Magisk patcher are embedded in the recovery.
mod process;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skyward_net::http::{self, Servers};
use skyward_sys::worker::Worker as Background;
use skyward_vab::{Slot, SnapshotState};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::RawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const KMI: &str = "android15-6.6";
const MODULE: &str = "lkm-aarch64-android15-6.6_kernelsu.ko";
const KSUD: &str = "ksud-aarch64-linux-android";
const ROOT: &str = "/data/media/0/Skyward/root";
const CACHE: &str = "/data/skyward-kernelsu";
const PAYLOAD_LIMIT: u64 = 16 << 20;
const IMAGE_LIMIT: u64 = 128 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Downloading,
    BackingUp,
    Patching,
    Writing,
    Done,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Job {
    InstallKernelSu { kernel: String },
    InstallModule { path: String },
}
pub enum Event {
    Stage(Stage),
    Ended(Result<String, String>),
}
pub struct Worker(Background<Event>);
impl Worker {
    pub fn start(job: Job, servers: Servers) -> Result<Self> {
        Ok(Self(Background::start("root-install", move |tell| {
            let stage = |stage| {
                tell.tell(Event::Stage(stage));
            };
            let done = match job {
                Job::InstallKernelSu { kernel } => install(&kernel, servers, &stage),
                Job::InstallModule { path } => install_module(Path::new(&path), &stage),
            }
            .map_err(|e| format!("{e:#}").chars().take(1000).collect());
            tell.tell(Event::Ended(done));
        })?))
    }
    pub fn fd(&self) -> RawFd {
        self.0.fd()
    }
    pub fn events(&self) -> Vec<Event> {
        self.0.events()
    }
}

/// A KMI is the kernel's Android branch, not the installed ROM's release.
pub fn compatible(kernel: &str) -> bool {
    let mut parts = kernel.split('-');
    let version = parts.next().unwrap_or_default();
    (version == "6.6" || version.starts_with("6.6.")) && parts.any(|part| part == "android15")
}
#[derive(Deserialize, Serialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}
#[derive(Clone, Deserialize, Serialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
    size: u64,
}
fn selected(release: &Release) -> Result<[Asset; 2]> {
    let find = |name: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .cloned()
            .with_context(|| format!("release has no {name}"))
    };
    Ok([find(KSUD)?, find(MODULE)?])
}
fn get(agent: &ureq::Agent, url: &str, limit: u64) -> Result<Vec<u8>> {
    // Executable resources are fetched from their official HTTPS locations.
    ensure!(
        url.starts_with("https://api.github.com/repos/tiann/KernelSU/")
            || url.starts_with("https://github.com/tiann/KernelSU/releases/download/"),
        "non-official KernelSU URL"
    );
    let mut reply = agent.get(url).header("User-Agent", "Skyward-KernelSU").call()?;
    ensure!(reply.status().is_success(), "GitHub answered {}", reply.status());
    let mut bytes = Vec::new();
    reply.body_mut().with_config().limit(limit).reader().read_to_end(&mut bytes)?;
    Ok(bytes)
}
fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut file = OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW).open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= limit,
        "invalid file or size: {}",
        path.display()
    );
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= limit, "file grew beyond size limit");
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    ensure!(
        digest(&read(path, bytes.len() as u64)?) == digest(bytes),
        "backup readback mismatch"
    );
    Ok(())
}
fn verified_asset(agent: &ureq::Agent, asset: &Asset, folder: &Path) -> Result<PathBuf> {
    ensure!([KSUD, MODULE].contains(&asset.name.as_str()), "unexpected KernelSU resource");
    ensure!(asset.size > 0 && asset.size <= PAYLOAD_LIMIT, "unexpected payload size");
    let want = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .context("release has no SHA-256 for payload")?;
    ensure!(
        want.len() == 64 && want.bytes().all(|c| c.is_ascii_hexdigit()),
        "invalid payload digest"
    );
    let path = folder.join(&asset.name);
    let matches = |bytes: &[u8]| bytes.len() as u64 == asset.size && digest(bytes).eq_ignore_ascii_case(want);
    if read(&path, PAYLOAD_LIMIT).is_ok_and(|bytes| matches(&bytes)) {
        return Ok(path);
    }
    let bytes = get(agent, &asset.browser_download_url, PAYLOAD_LIMIT)?;
    ensure!(matches(&bytes), "payload size or hash mismatch");
    let mut part = tempfile::NamedTempFile::new_in(folder)?;
    part.write_all(&bytes)?;
    part.as_file().sync_all()?;
    part.persist(&path).map_err(|e| e.error)?;
    ensure!(matches(&read(&path, PAYLOAD_LIMIT)?), "cached payload changed");
    Ok(path)
}
fn resources(agent: &ureq::Agent, cache: &Path) -> Result<(Release, PathBuf, PathBuf)> {
    let manifest = cache.join("release.json");
    let fresh = get(agent, "https://api.github.com/repos/tiann/KernelSU/releases/latest", 2 << 20);
    let release: Release = serde_json::from_slice(&match fresh {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = skyward_sys::kmsg(&format!("KernelSU: checking verified offline cache: {error:#}"));
            read(&manifest, 2 << 20).context("no verified offline cache")?
        }
    })?;
    let assets = selected(&release)?;
    let version = folder(&cache.join(digest(release.tag_name.as_bytes())), true)?;
    let ksud = verified_asset(agent, &assets[0], &version)?;
    let module = verified_asset(agent, &assets[1], &version)?;
    let mut saved = tempfile::NamedTempFile::new_in(cache)?;
    serde_json::to_writer(
        &mut saved,
        &Release {
            tag_name: release.tag_name.clone(),
            assets: assets.into(),
        },
    )?;
    saved.as_file().sync_all()?;
    saved.persist(&manifest).map_err(|e| e.error)?;
    Ok((release, ksud, module))
}
fn guard() -> Result<Slot> {
    ensure!(
        skyward_sys::bootloader_unlocked()? == Some(true),
        "bootloader unlock is not confirmed"
    );
    ensure!(
        skyward_vab::snapshot_state()? == SnapshotState::None,
        "an OTA snapshot is pending; finish it first"
    );
    skyward_vab::current_slot()?.context("current slot is unknown")
}
fn folder(path: &Path, private: bool) -> Result<PathBuf> {
    let mut part = PathBuf::new();
    for component in path.components() {
        part.push(component);
        match fs::symlink_metadata(&part) {
            Ok(metadata) => ensure!(
                metadata.is_dir() && !metadata.is_symlink(),
                "resource directory contains a link or non-directory"
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&part)?,
            Err(e) => return Err(e.into()),
        }
    }
    if private {
        ensure!(fs::metadata(path)?.uid() == 0, "resource cache must be root-owned");
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(path.to_owned())
}
fn elf(bytes: &[u8], kind: u16) -> bool {
    bytes.len() > 20
        && bytes.starts_with(b"\x7fELF\x02\x01")
        && u16::from_le_bytes([bytes[16], bytes[17]]) == kind
        && u16::from_le_bytes([bytes[18], bytes[19]]) == 183
}
fn module_kmi(bytes: &[u8]) -> bool {
    // Official asset name and digest pin the Android branch; vermagic may omit it.
    bytes
        .split(|&c| c == 0)
        .filter_map(|s| std::str::from_utf8(s).ok())
        .any(|s| s.strip_prefix("vermagic=").is_some_and(|s| s.starts_with("6.6.")))
}
fn kernel_kmi(packed: &[u8]) -> Result<String> {
    let reader: Box<dyn Read + '_> = if packed.starts_with(&[0x1f, 0x8b]) {
        Box::new(flate2::read::MultiGzDecoder::new(packed))
    } else if packed.starts_with(&[2, 0x21, 0x4c, 0x18]) || packed.starts_with(&[4, 0x22, 0x4d, 0x18]) {
        Box::new(lz4_flex::frame::FrameDecoder::new(packed))
    } else {
        Box::new(packed)
    };
    let mut bytes = Vec::new();
    reader.take(IMAGE_LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= IMAGE_LIMIT, "kernel exceeds inspection limit");
    let start = bytes
        .windows(14)
        .position(|w| w == b"Linux version ")
        .context("cannot confirm installed boot kernel KMI")?
        + 14;
    let end = bytes[start..]
        .iter()
        .position(|c| c.is_ascii_whitespace() || *c == 0)
        .context("unterminated kernel version")?
        + start;
    let version = std::str::from_utf8(&bytes[start..end])?;
    ensure!(compatible(version), "installed boot kernel is not android15-6.6: {version}");
    Ok(version.into())
}
fn partition(path: &Path) -> Result<Vec<u8>> {
    let size = skyward_block::size_bytes(path)?;
    ensure!((4096..=256 << 20).contains(&size), "unexpected boot partition size");
    let mut file = File::open(path)?;
    skyward_block::drop_cache(&file)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file).take(size + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 == size, "incomplete boot partition read");
    Ok(bytes)
}
#[derive(Serialize)]
struct Record {
    version: String,
    kmi: &'static str,
    slot: String,
    partition: String,
    backup: String,
    original_sha256: String,
    patched_sha256: String,
}
fn install(kernel: &str, servers: Servers, stage: &dyn Fn(Stage)) -> Result<String> {
    ensure!(compatible(kernel), "requires a confirmed android15-6.6 kernel KMI");
    let slot = guard()?;
    let cache = folder(Path::new(CACHE), true)?;
    let root = folder(Path::new(ROOT), false)?;
    let device = Path::new(skyward_block::BY_NAME).join(format!("init_boot{}", slot.suffix()));
    let boot = Path::new(skyward_block::BY_NAME).join(format!("boot{}", slot.suffix()));
    let boot_bytes = partition(&boot)?;
    kernel_kmi(&skyward_bootimg::BootImage::parse(&boot_bytes)?.kernel)?;
    // Never layer two root patchers over an existing modified ramdisk.
    ensure!(
        !Path::new("/data/adb/magisk/magisk").exists(),
        "Magisk detected; restore stock init_boot before installing KernelSU"
    );
    stage(Stage::Downloading);
    let agent = http::agent(servers);
    let (release, ksud, module) = resources(&agent, &cache)?;
    let module_bytes = read(&module, PAYLOAD_LIMIT)?;
    ensure!(
        elf(&read(&ksud, PAYLOAD_LIMIT)?, 3) && elf(&module_bytes, 1) && module_kmi(&module_bytes),
        "payload architecture or module KMI mismatch"
    );
    fs::set_permissions(&ksud, fs::Permissions::from_mode(0o700))?;
    stage(Stage::BackingUp);
    let original = partition(&device)?;
    let size = original.len() as u64;
    ensure!(size <= IMAGE_LIMIT, "unexpected init_boot partition size");
    let image = skyward_bootimg::BootImage::parse(&original)?;
    ensure!(
        image.kernel.is_empty() && !image.ramdisk.is_empty(),
        "not a ramdisk-only init_boot image"
    );
    ensure!(
        !skyward_install::image::rooted_ramdisk(image.ramdisk.as_slice())?,
        "init_boot already contains root; restore stock first"
    );
    let sum = digest(&original);
    let backup = root.join(format!("init_boot{}-{sum}.img", slot.suffix()));
    if backup.exists() {
        ensure!(digest(&read(&backup, IMAGE_LIMIT)?) == sum, "stock backup mismatch");
    } else {
        write_new(&backup, &original)?;
    }
    let work = tempfile::tempdir_in(&cache)?;
    let input = work.path().join("stock.img");
    write_new(&input, &original)?;
    stage(Stage::Patching);
    process::run(
        Command::new(&ksud)
            .args(["boot-patch", "-b"])
            .arg(&input)
            .arg("-m")
            .arg(module)
            .arg("-o")
            .arg(work.path())
            .args(["--out-name", "patched.img", "--kmi", KMI]),
        Duration::from_secs(120),
    )?;
    let patched = work.path().join("patched.img");
    let bytes = read(&patched, size)?;
    ensure!(
        !bytes.is_empty() && bytes.len() as u64 <= size && digest(&bytes) != sum,
        "invalid patched image size or unchanged image"
    );
    let updated = skyward_bootimg::BootImage::parse(&bytes)?;
    ensure!(
        updated.kernel == image.kernel
            && updated.header.header_version() == image.header.header_version()
            && skyward_install::image::rooted_ramdisk(updated.ramdisk.as_slice())?,
        "patched image has unexpected layout or no Root payload"
    );
    ensure!(
        guard()? == slot && digest(&partition(&device)?) == sum && digest(&partition(&boot)?) == digest(&boot_bytes),
        "slot or boot images changed during preparation"
    );
    let record = Record {
        version: release.tag_name,
        kmi: KMI,
        slot: slot.suffix().into(),
        partition: device.display().to_string(),
        backup: backup.display().to_string(),
        original_sha256: sum,
        patched_sha256: digest(&bytes),
    };
    let record_path = root.join(format!("init_boot{}-{}.json", slot.suffix(), record.patched_sha256));
    if !record_path.exists() {
        write_new(&record_path, &serde_json::to_vec_pretty(&record)?)?;
    }
    stage(Stage::Writing);
    skyward_install::image::flash(&mut File::open(&patched)?, bytes.len() as u64, &[device], &mut |_, _| {})?;
    stage(Stage::Done);
    Ok(format!(
        "KernelSU {}: init_boot{}; backup: {}",
        record.version,
        slot.suffix(),
        backup.display()
    ))
}

fn validate_module(file: File) -> Result<()> {
    let mut zip = zip::ZipArchive::new(file)?;
    let mut prop = zip.by_name("module.prop").context("not a Root module: module.prop is missing")?;
    ensure!(prop.size() <= 64 << 10, "module.prop is too large");
    let mut text = String::new();
    Read::by_ref(&mut prop).take((64 << 10) + 1).read_to_string(&mut text)?;
    ensure!(
        text.len() <= 64 << 10
            && text
                .lines()
                .any(|line| line.trim().strip_prefix("id=").is_some_and(|id| !id.is_empty()
                    && id.len() <= 128
                    && id.bytes().next().is_some_and(|c| c.is_ascii_alphabetic())
                    && id.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c)))),
        "module.prop has no valid module id"
    );
    Ok(())
}
fn install_module(path: &Path, stage: &dyn Fn(Stage)) -> Result<String> {
    ensure!(
        path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")),
        "choose a module ZIP"
    );
    let providers = [
        ("/data/adb/ksu/bin/ksud", true),
        ("/data/adb/ksud", true),
        ("/data/adb/magisk/magisk", false),
    ];
    let found: Vec<_> = providers.into_iter().filter(|(path, _)| Path::new(path).is_file()).collect();
    let is_ksu = found.iter().any(|(_, k)| *k);
    let is_magisk = found.iter().any(|(_, k)| !*k);
    ensure!(is_ksu != is_magisk, "no unique installed Root manager found");
    let (program, ksu) = found.first().context("no installed Root manager")?;
    let cache = folder(Path::new(CACHE), true)?;
    let work = tempfile::tempdir_in(cache)?;
    let mut source = OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW).open(path)?;
    let metadata = source.metadata()?;
    const LIMIT: u64 = 512 << 20;
    ensure!(metadata.is_file() && metadata.len() <= LIMIT, "invalid module ZIP size");
    let staged = work.path().join("module.zip");
    let mut output = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&staged)?;
    let count = std::io::copy(&mut Read::by_ref(&mut source).take(LIMIT + 1), &mut output)?;
    ensure!(count == metadata.len() && count <= LIMIT, "module ZIP changed during staging");
    output.sync_all()?;
    drop(output);
    validate_module(File::open(&staged)?)?;
    stage(Stage::Patching);
    // The manager receives a stable private copy, never a removable source file.
    let mut command = Command::new(program);
    command
        .args(if *ksu {
            &["module", "install"][..]
        } else {
            &["--install-module"][..]
        })
        .arg(staged);
    process::run(&mut command, Duration::from_secs(600))?;
    stage(Stage::Done);
    Ok("Module installed; restart Android to apply".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kmi_uses_the_kernel_branch_not_the_rom_android_version() {
        assert!(compatible("6.6.98-android15-8-g123"));
        assert!(!compatible("6.6.98-android17-1"));
        assert!(!compatible("6.6.98"));
        assert!(!compatible("6.12.1-android15-1"));
    }
    #[test]
    fn only_six_six_and_ksud_are_selected() {
        let asset = |name: &str| Asset {
            name: name.into(),
            browser_download_url: String::new(),
            digest: None,
            size: 1,
        };
        let release = Release {
            tag_name: String::new(),
            assets: vec![
                asset(MODULE),
                asset(KSUD),
                asset("KernelSU.apk"),
                asset("lkm-aarch64-android16-6.12_kernelsu.ko"),
            ],
        };
        let found = selected(&release).unwrap();
        assert_eq!((&found[0].name, &found[1].name), (&KSUD.to_string(), &MODULE.to_string()));
    }
    #[test]
    fn module_and_boot_kmi_are_checked_independently() {
        assert!(module_kmi(b"ELF\0vermagic=6.6.98-android15-8 SMP preempt\0"));
        assert!(module_kmi(
            b"vermagic=6.6.127-4k-g46a034eca005-dirty SMP preempt mod_unload modversions aarch64\0"
        ));
        assert!(!module_kmi(b"vermagic=6.12.1-android16-1 SMP\0"));
        assert_eq!(
            kernel_kmi(b"prefix\0Linux version 6.6.98-android15-8 (builder)").unwrap(),
            "6.6.98-android15-8"
        );
        assert!(kernel_kmi(b"Linux version 6.6.98-android17-1 (builder)").is_err());
    }
    #[test]
    fn cached_reads_reject_symlinks_and_oversized_files() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        fs::write(&real, [0; 32]).unwrap();
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(read(&link, 100).is_err());
        assert!(read(&real, 16).is_err());
    }
    #[test]
    fn a_rom_zip_cannot_be_mistaken_for_a_root_module() {
        for (name, body, valid) in [
            ("module.prop", "id=test_module\nname=Test\n", true),
            ("payload.bin", "payload", false),
            ("module.prop", "id=../bad", false),
        ] {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            let mut zip = zip::ZipWriter::new(file.as_file_mut());
            zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
            zip.finish().unwrap();
            assert_eq!(validate_module(File::open(file.path()).unwrap()).is_ok(), valid);
        }
    }
}
