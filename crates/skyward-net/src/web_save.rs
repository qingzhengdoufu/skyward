//! Resumable browser transfers; unlike OTA downloads these have no trusted digest.
use crate::web::Saving;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Serialize, Deserialize)]
struct Resume {
    source: String,
    validator: Option<Validator>,
    total: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Validator {
    Etag(String),
    Modified(String),
}

impl Validator {
    fn value(&self) -> &str {
        match self {
            Self::Etag(value) | Self::Modified(value) => value,
        }
    }
}

fn validator(headers: &ureq::http::HeaderMap) -> Option<Validator> {
    if let Some(etag) = text(headers, "etag").filter(|v| v.starts_with('"') && v.ends_with('"')) {
        return Some(Validator::Etag(etag.into()));
    }
    text(headers, "last-modified")
        .filter(|v| !v.is_empty())
        .map(|v| Validator::Modified(v.into()))
}

fn text<'a>(headers: &'a ureq::http::HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn source_key(url: &str) -> String {
    let url = url.split('#').next().unwrap_or(url);
    // GitHub renews signed queries without changing the immutable asset path.
    if let Ok(uri) = url.parse::<ureq::http::Uri>()
        && uri.scheme_str() == Some("https")
        && uri.host() == Some("release-assets.githubusercontent.com")
    {
        return format!("https://release-assets.githubusercontent.com{}", uri.path());
    }
    url.to_string()
}

fn metadata_path(part: &Path) -> PathBuf {
    let mut name = part.as_os_str().to_os_string();
    name.push(".json");
    PathBuf::from(name)
}

fn read_resume(path: &Path) -> Option<Resume> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .ok()?;
    let info = file.metadata().ok()?;
    if !info.is_file() || info.len() > 8192 {
        return None;
    }
    serde_json::from_reader(file).ok()
}

fn write_resume(path: &Path, resume: &Resume) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().context("download metadata folder")?)?;
    serde_json::to_writer(&mut file, resume)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error).context("keep download resume metadata")?;
    Ok(())
}

fn content_range(value: &str) -> Option<(u64, u64, u64)> {
    let (range, total) = value.strip_prefix("bytes ")?.split_once('/')?;
    let (first, last) = range.split_once('-')?;
    let (first, last, total) = (first.parse().ok()?, last.parse().ok()?, total.parse().ok()?);
    (first <= last && last < total).then_some((first, last, total))
}

fn finish(file: File, part: &Path, path: &Path, metadata: &Path, done: u64) -> Result<u64> {
    file.sync_all().with_context(|| format!("write {} through", part.display()))?;
    let temporary = tempfile::NamedTempFile::from_parts(file, tempfile::TempPath::try_from_path(part)?);
    if let Err(error) = temporary.persist_noclobber(path) {
        let reason = error.error;
        // A racing destination must not erase a transfer that can be resumed.
        let _ = error.file.into_temp_path().keep();
        return Err(reason).with_context(|| format!("save {} without replacing an existing file", path.display()));
    }
    let _ = fs::remove_file(metadata);
    Ok(done)
}

pub(crate) fn save(agent: &ureq::Agent, url: &str, path: &Path, stop: &AtomicBool, progress: &mut dyn FnMut(Saving)) -> Result<u64> {
    ensure!(crate::web::readable_url(url), "{url} is not an http or https address");
    if stop.load(Ordering::Relaxed) {
        bail!(crate::Stopped);
    }
    ensure!(!path.try_exists()?, "{} already exists", path.display());
    let dir = path.parent().context("a download has a folder to go in")?;
    fs::create_dir_all(dir).with_context(|| format!("make {}", dir.display()))?;
    let part = crate::download::partial(path);
    let metadata = metadata_path(&part);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&part)
        .with_context(|| format!("open {}", part.display()))?;
    ensure!(file.metadata()?.is_file(), "{} is not a regular download file", part.display());
    ensure!(
        unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "{} is being downloaded already",
        path.display()
    );
    let held = file.metadata()?.len();
    let key = source_key(url);
    let kept = read_resume(&metadata).filter(|r| r.source == key && r.validator.is_some() && r.total.is_none_or(|total| held <= total));
    let have = if kept.is_some() { held } else { 0 };
    let mut request = agent
        .get(url)
        .header("User-Agent", crate::web::USER_AGENT)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "identity");
    if have > 0 {
        request = request.header("Range", format!("bytes={have}-")).header(
            "If-Range",
            kept.as_ref()
                .and_then(|r| r.validator.as_ref())
                .expect("resuming has a validator")
                .value(),
        );
    }
    let mut response = request.call().with_context(|| format!("start downloading {url}"))?;
    let headers = response.headers();
    ensure!(
        text(headers, "content-encoding").is_none_or(|v| v.eq_ignore_ascii_case("identity")),
        "the server encoded a byte-range download"
    );
    let current = validator(headers);
    let (mut done, total) = match response.status().as_u16() {
        200 => (0, crate::download::content_length(headers)),
        206 if have > 0 => {
            let (first, last, total) = text(headers, "content-range")
                .and_then(content_range)
                .context("invalid Content-Range on resumed download")?;
            let previous = kept.as_ref().expect("a resumed download has metadata");
            ensure!(
                first == have && last == total - 1,
                "the server returned the wrong resumed byte range"
            );
            ensure!(
                previous.validator == current && previous.total.is_none_or(|n| n == total),
                "the file changed while it was paused"
            );
            ensure!(
                crate::download::content_length(headers).is_none_or(|n| n == last - first + 1),
                "resumed length disagrees with Content-Range"
            );
            (have, Some(total))
        }
        416 if have > 0 => {
            let total = text(headers, "content-range")
                .and_then(|s| s.strip_prefix("bytes */"))
                .and_then(|s| s.parse().ok());
            let previous = kept.as_ref().expect("a resumed download has metadata");
            ensure!(
                total == Some(have) && previous.total == total && current == previous.validator,
                "the server refused the saved byte range"
            );
            progress(Saving { done: have, total });
            return finish(file, &part, path, &metadata, have);
        }
        status => bail!("the server answered {status} for {url}"),
    };
    if stop.load(Ordering::Relaxed) {
        bail!(crate::Stopped);
    }
    if let Some(total) = total {
        let room = crate::download::free(dir)?;
        ensure!(total - done <= room, "{} more bytes are needed, {room} are free", total - done);
    }
    if done == 0 {
        file.set_len(0)?;
    }
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(done))?;
    write_resume(
        &metadata,
        &Resume {
            source: key,
            validator: current,
            total,
        },
    )?;
    let mut body = response.body_mut().with_config().limit(u64::MAX).reader();
    let mut buffer = vec![0u8; 64 << 10];
    progress(Saving { done, total });
    loop {
        if stop.load(Ordering::Relaxed) {
            bail!(crate::Stopped);
        }
        let count = body.read(&mut buffer).context("read the download; saved bytes can be resumed")?;
        if count == 0 {
            break;
        }
        ensure!(
            total.is_none_or(|n| done.checked_add(count as u64).is_some_and(|next| next <= n)),
            "the server sent more bytes than declared"
        );
        file.write_all(&buffer[..count])
            .with_context(|| format!("write {}", part.display()))?;
        done += count as u64;
        progress(Saving { done, total });
    }
    ensure!(
        total.is_none_or(|n| n == done),
        "the server stopped at {done} of {total:?} bytes; saved bytes can be resumed"
    );
    finish(file, &part, path, &metadata, done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::net::TcpListener;
    fn server(replies: Vec<(&'static str, Vec<u8>)>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (headers, bytes) in replies {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut reader = std::io::BufReader::new(&socket);
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    assert!(!line.is_empty());
                    request.push_str(&line);
                }
                requests.push(request.to_lowercase());
                socket
                    .write_all(format!("HTTP/1.1 {headers}\r\nConnection: close\r\n\r\n").as_bytes())
                    .unwrap();
                let _ = socket.write_all(&bytes);
            }
            requests
        });
        (url, worker)
    }
    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder().proxy(None).http_status_as_error(false).build().into()
    }
    fn interrupted(path: &Path, source: &str, total: u64) {
        fs::write(crate::download::partial(path), b"abcdef").unwrap();
        write_resume(
            &metadata_path(&crate::download::partial(path)),
            &Resume {
                source: source.into(),
                validator: Some(Validator::Etag("\"one\"".into())),
                total: Some(total),
            },
        )
        .unwrap();
    }
    #[test]
    fn a_connection_cut_keeps_bytes_and_retry_checks_the_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rom.zip");
        let (url, worker) = server(vec![
            ("200 OK\r\nContent-Length: 10\r\nETag: \"one\"", b"abcdef".to_vec()),
            (
                "206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 6-9/10\r\nETag: \"one\"",
                b"ghij".to_vec(),
            ),
        ]);
        assert!(save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).is_err());
        assert_eq!(fs::read(crate::download::partial(&path)).unwrap(), b"abcdef");
        let mut seen = Vec::new();
        assert_eq!(
            save(&agent(), &url, &path, &AtomicBool::new(false), &mut |s| seen.push(s)).unwrap(),
            10
        );
        assert_eq!(seen[0], Saving { done: 6, total: Some(10) });
        assert_eq!(fs::read(path).unwrap(), b"abcdefghij");
        let requests = worker.join().unwrap();
        assert!(requests[1].contains("range: bytes=6-"));
        assert!(requests[1].contains("if-range: \"one\""));
    }
    #[test]
    fn an_ignored_range_restarts_without_appending_the_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rom.zip");
        let (url, worker) = server(vec![("200 OK\r\nContent-Length: 3\r\nETag: \"two\"", b"new".to_vec())]);
        interrupted(&path, &url, 10);
        save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(fs::read(path).unwrap(), b"new");
        worker.join().unwrap();
    }
    #[test]
    fn wrong_ranges_and_changed_validators_never_modify_saved_bytes() {
        for headers in [
            "206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 5-8/10\r\nETag: \"one\"",
            "206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 6-9/10\r\nETag: \"two\"",
            "206 Partial Content\r\nContent-Length: 5\r\nContent-Range: bytes 6-9/10\r\nETag: \"one\"",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("rom.zip");
            let (url, worker) = server(vec![(headers, b"ghij".to_vec())]);
            interrupted(&path, &url, 10);
            assert!(save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).is_err());
            assert_eq!(fs::read(crate::download::partial(&path)).unwrap(), b"abcdef");
            worker.join().unwrap();
        }
    }
    #[test]
    fn pausing_preserves_a_part_and_finishing_never_overwrites_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rom.zip");
        let (url, worker) = server(vec![("200 OK\r\nContent-Length: 131072\r\nETag: \"one\"", vec![1; 131072])]);
        let stop = AtomicBool::new(false);
        assert!(
            save(&agent(), &url, &path, &stop, &mut |s| {
                if s.done > 0 {
                    stop.store(true, Ordering::Relaxed);
                }
            })
            .unwrap_err()
            .is::<crate::Stopped>()
        );
        assert!(fs::metadata(crate::download::partial(&path)).unwrap().len() > 0);
        assert!(!path.exists());
        worker.join().unwrap();
        fs::write(&path, b"existing").unwrap();
        assert!(save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).is_err());
        assert_eq!(fs::read(path).unwrap(), b"existing");
    }
    #[test]
    fn a_completed_part_is_revalidated_before_it_takes_the_final_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rom.zip");
        let (url, worker) = server(vec![(
            "416 Range Not Satisfiable\r\nContent-Length: 0\r\nContent-Range: bytes */6\r\nETag: \"one\"",
            Vec::new(),
        )]);
        interrupted(&path, &url, 6);
        assert_eq!(save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).unwrap(), 6);
        assert_eq!(fs::read(path).unwrap(), b"abcdef");
        worker.join().unwrap();
    }
    #[test]
    fn a_different_resource_does_not_resume_just_because_its_filename_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rom.zip");
        let (url, worker) = server(vec![("200 OK\r\nContent-Length: 3", b"new".to_vec())]);
        interrupted(&path, "http://elsewhere.invalid/file", 10);
        save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert!(!worker.join().unwrap()[0].contains("range:"));
        assert_eq!(fs::read(path).unwrap(), b"new");
    }
    #[test]
    fn unvalidated_parts_restart_and_expired_links_keep_existing_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rom.zip");
        let (url, worker) = server(vec![("403 Forbidden\r\nContent-Length: 0", Vec::new())]);
        interrupted(&path, &url, 10);
        assert!(save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).is_err());
        assert_eq!(fs::read(crate::download::partial(&path)).unwrap(), b"abcdef");
        worker.join().unwrap();
        let (url, worker) = server(vec![("200 OK\r\nContent-Length: 3", b"new".to_vec())]);
        write_resume(
            &metadata_path(&crate::download::partial(&path)),
            &Resume {
                source: url.clone(),
                validator: None,
                total: Some(10),
            },
        )
        .unwrap();
        save(&agent(), &url, &path, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert!(!worker.join().unwrap()[0].contains("range:"));
        assert_eq!(fs::read(path).unwrap(), b"new");
    }

    #[test]
    fn download_lock_rejects_a_second_writer_and_symlinks_are_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rom.zip");
        let part = crate::download::partial(&path);
        let held = File::create(&part).unwrap();
        assert_eq!(unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0);
        assert!(
            save(&agent(), "http://127.0.0.1:1/file", &path, &AtomicBool::new(false), &mut |_| {})
                .unwrap_err()
                .to_string()
                .contains("already")
        );
        drop(held);
        fs::remove_file(&part).unwrap();
        let target = dir.path().join("keep");
        fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(&target, &part).unwrap();
        assert!(save(&agent(), "http://127.0.0.1:1/file", &path, &AtomicBool::new(false), &mut |_| {}).is_err());
        assert_eq!(fs::read(target).unwrap(), b"keep");
    }
    #[test]
    fn only_github_asset_signatures_are_excluded_from_the_resource_identity() {
        assert_eq!(
            source_key("https://release-assets.githubusercontent.com/asset/a?sig=old"),
            source_key("https://release-assets.githubusercontent.com/asset/a?sig=new")
        );
        assert_ne!(
            source_key("https://example.com/file?id=1"),
            source_key("https://example.com/file?id=2")
        );
        assert!(content_range("bytes 10-9/20").is_none());
        assert!(content_range("bytes 1-20/20").is_none());
    }
}
