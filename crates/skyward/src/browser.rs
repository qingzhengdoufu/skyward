//! Reading a web page, and saving a file it offers, on a thread of their
//! own that tells the screen how far they have come.

use crate::cdp::Cdp;
use anyhow::{Context, Result};
use skyward_net::http::{self, Servers};
use skyward_net::web::{self, Opened, Saving};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The basic browser's own log, which the log page exports beside the
/// recovery and kernel logs.
pub const LOG: &str = "/skyward/run/browser.log";

/// The most often how far a save has come is told.
const PROGRESS_EVERY: Duration = Duration::from_millis(200);

pub enum Task {
    /// Read the page at this address, with the cookies kept for it.
    Open {
        url: String,
        cookie: Option<String>,
    },
    /// Send a form the page carries.
    Submit {
        url: String,
        fields: Vec<(String, String)>,
        post: bool,
        cookie: Option<String>,
    },
    /// Save the file at this address to this path.
    Save {
        url: String,
        path: PathBuf,
    },
    /// Unpack the plugin package at `archive` into `dir`.
    Install {
        archive: PathBuf,
        dir: PathBuf,
    },
    /// Keep a continuous frame stream from the persistent page.
    FullStream {
        dir: PathBuf,
        url: String,
    },
    FullTap {
        x: u16,
        y: u16,
    },
    FullPinch {
        scale: f64,
    },
    FullText {
        text: String,
    },
    FullScroll {
        dy: i64,
    },
    FullReload,
    FullNavigate {
        url: String,
    },
    FullStop,
    /// Fetch the plugin package from a release URL.
    PluginDownload {
        url: String,
        path: PathBuf,
    },
}

pub enum Event {
    Navigated(String),
    Download {
        url: String,
        name: String,
    },
    /// The address turned out to hold a page or a file, or why it did not.
    Opened(Result<Opened, String>),
    /// How far the file has been saved.
    Saved(Saving),
    /// The file is whole, or why it is not.
    Ended(Result<u64, String>),
    /// The plugin was unpacked, or why it was not.
    Installed(Result<(), String>),
    /// The plugin rendered a page, or why it did not.
    Shot(Result<Vec<u8>, String>),
    /// A text field on the plugin page took focus.
    Focus(bool),
    /// How far the plugin package download has come.
    PluginSaved {
        done: u64,
        total: Option<u64>,
        speed: u64,
    },
    /// The plugin package finished, or why it did not.
    PluginEnded(Result<(), String>),
}

struct FullSession {
    child: Child,
    dir: PathBuf,
}

static FULL: OnceLock<Mutex<Option<FullSession>>> = OnceLock::new();

fn full_slot() -> &'static Mutex<Option<FullSession>> {
    FULL.get_or_init(|| Mutex::new(None))
}

fn stop_full() {
    let mut slot = full_slot().lock().expect("the full browser slot is not poisoned");
    if let Some(mut session) = slot.take() {
        let pid = session.child.id() as i32;
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
        let _ = session.child.kill();
        let _ = session.child.wait();
        let d = session.dir.display().to_string();
        let _ = Command::new("/system/bin/sh")
            .args([
                "-c",
                &format!(
                    "umount '{d}/sys' 2>/dev/null || true; umount '{d}/proc' 2>/dev/null || true; umount '{d}/dev' 2>/dev/null || true"
                ),
            ])
            .status();
    }
}

fn full_launch_script() -> &'static str {
    include_str!("chromium-launch.sh")
}

fn start_full(dir: &Path, url: &str) -> Result<()> {
    anyhow::ensure!(dir == Path::new("/data/skyward-browser/chromium"), "unexpected browser directory");
    anyhow::ensure!(dir.join("run.sh").is_file(), "Chromium run.sh is missing");
    let mut slot = full_slot().lock().expect("the full browser slot is not poisoned");
    if let Some(session) = slot.as_mut() {
        if session.child.try_wait().ok().flatten().is_none() {
            return Ok(());
        }
        slot.take();
    }
    let script = full_launch_script();
    let mut cmd = Command::new("/system/bin/sh");
    cmd.args(["-c", script])
        .env("SKYWARD_URL", url)
        .env("SKYWARD_CHROMIUM_ROOT", dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }
    let child = cmd.spawn().context("start the full browser")?;
    *slot = Some(FullSession {
        child,
        dir: dir.to_path_buf(),
    });
    Ok(())
}

fn full_started(dir: &Path, url: &str) -> Result<Cdp> {
    start_full(dir, url)?;
    Cdp::connect()
}

fn full_page_shot(cdp: &mut Cdp) -> Result<Vec<u8>> {
    cdp.screenshot()
}

fn full_result(cdp: &mut Cdp) -> (Result<Vec<u8>, String>, bool) {
    let focus = cdp.focus_is_text();
    (full_page_shot(cdp).map_err(|e| format!("{e:#}")), focus)
}

pub struct Worker {
    events: Receiver<Event>,
    /// Readable while events wait.
    wake: OwnedFd,
    stop: Arc<AtomicBool>,
    /// The connection in use, which stopping cuts at once.
    agent: Arc<Mutex<Option<ureq::Agent>>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(task: Task, servers: Servers) -> Result<Worker> {
        let mut fds = [0; 2];
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
            return Err(anyhow::anyhow!("make the browser's wake pipe: {}", std::io::Error::last_os_error()));
        }
        let (wake, signal) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
        let (tx, events) = mpsc::sync_channel(4);
        let stop = Arc::new(AtomicBool::new(false));
        let agent: Arc<Mutex<Option<ureq::Agent>>> = Arc::new(Mutex::new(None));
        let (stopping, using) = (stop.clone(), agent.clone());
        let thread = thread::Builder::new()
            .name("browser".into())
            .spawn(move || {
                let tell = |event: Event| {
                    let _ = tx.send(event);
                    unsafe { libc::write(signal.as_raw_fd(), [1u8].as_ptr().cast(), 1) };
                };
                let a = http::agent(servers);
                *using.lock().expect("the agent is not held by a thread that panicked") = Some(a.clone());
                match task {
                    Task::Open { url, cookie } => {
                        note(&format!("open {url}"));
                        let opened = web::open_with(&a, &url, cookie.as_deref()).map_err(|e| format!("{e:#}"));
                        match &opened {
                            Ok(Opened::Page(page)) => note(&format!("opened {} ({})", page.url, page.blocks.len())),
                            Ok(Opened::File(file)) => note(&format!("file {} ({})", file.name, file.kind)),
                            Err(why) => note(&format!("open {url} failed: {why}")),
                        }
                        tell(Event::Opened(opened));
                    }
                    Task::Submit { url, fields, post, cookie } => {
                        note(&format!(
                            "submit {} {url} with {} fields",
                            if post { "POST" } else { "GET" },
                            fields.len()
                        ));
                        let opened = web::submit(&a, &url, &fields, post, cookie.as_deref()).map_err(|e| format!("{e:#}"));
                        match &opened {
                            Ok(Opened::Page(page)) => note(&format!("submitted to {} ({})", page.url, page.blocks.len())),
                            Ok(Opened::File(file)) => note(&format!("submitted to file {}", file.name)),
                            Err(why) => note(&format!("submit {url} failed: {why}")),
                        }
                        tell(Event::Opened(opened));
                    }
                    Task::Save { url, path } => {
                        note(&format!("save {url} to {}", path.display()));
                        let mut told = Instant::now() - PROGRESS_EVERY;
                        let mut progress = |saving: Saving| {
                            if told.elapsed() >= PROGRESS_EVERY || saving.total.is_some_and(|t| saving.done >= t) {
                                told = Instant::now();
                                tell(Event::Saved(saving));
                            }
                        };
                        let ended = web::save(&a, &url, &path, &stopping, &mut progress).map_err(|e| {
                            if stopping.load(Ordering::Relaxed) {
                                String::from("stopped as asked")
                            } else {
                                format!("{e:#}")
                            }
                        });
                        note(&match &ended {
                            Ok(size) => format!("saved {} ({size} bytes)", path.display()),
                            Err(why) => format!("save {} failed: {why}", path.display()),
                        });
                        tell(Event::Ended(ended));
                    }
                    Task::Install { archive, dir } => {
                        let ended = skyward_files::browser_plugin::install(&archive, &dir).map_err(|e| format!("{e:#}"));
                        tell(Event::Installed(ended));
                    }
                    Task::FullStream { dir, url } => {
                        let result = (|| -> Result<()> {
                            let mut cdp = full_started(&dir, &url)?;
                            cdp.navigate(&url)?;
                            tell(Event::Shot(cdp.screenshot().map_err(|e| format!("{e:#}"))));
                            cdp.start_screencast()?;
                            while !stopping.load(Ordering::Relaxed) {
                                match cdp.next_event()? {
                                    Some(crate::cdp::Event::Frame(frame)) => tell(Event::Shot(Ok(frame))),
                                    Some(crate::cdp::Event::Download { url, name }) => tell(Event::Download { url, name }),
                                    Some(crate::cdp::Event::Navigated(url)) => tell(Event::Navigated(url)),
                                    Some(crate::cdp::Event::Open(url)) => cdp.navigate(&url)?,
                                    None => {}
                                }
                            }
                            Ok(())
                        })();
                        if let Err(e) = result {
                            tell(Event::Shot(Err(format!("{e:#}"))));
                        }
                    }
                    Task::FullTap { x, y } => {
                        let result = Cdp::connect().and_then(|mut cdp| {
                            let cx = (x as u32 * crate::cdp::WIDTH / 1000).min(crate::cdp::WIDTH - 1);
                            let cy = (y as u32 * crate::cdp::HEIGHT / 1000).min(crate::cdp::HEIGHT - 1);
                            cdp.touch(cx, cy)?;
                            Ok(full_result(&mut cdp))
                        });
                        match result {
                            Ok((png, focus)) => {
                                tell(Event::Shot(png));
                                tell(Event::Focus(focus));
                            }
                            Err(e) => {
                                stop_full();
                                tell(Event::Shot(Err(format!("{e:#}"))));
                            }
                        }
                    }
                    Task::FullPinch { scale } => {
                        let result = Cdp::connect().and_then(|mut cdp| {
                            cdp.pinch(crate::cdp::WIDTH / 2, crate::cdp::HEIGHT / 2, scale)?;
                            Ok(full_result(&mut cdp))
                        });
                        match result {
                            Ok((png, focus)) => {
                                tell(Event::Shot(png));
                                tell(Event::Focus(focus));
                            }
                            Err(e) => {
                                stop_full();
                                tell(Event::Shot(Err(format!("{e:#}"))));
                            }
                        }
                    }
                    Task::FullText { text } => {
                        let result = Cdp::connect().and_then(|mut cdp| {
                            cdp.insert_text(&text)?;
                            Ok(full_result(&mut cdp))
                        });
                        match result {
                            Ok((png, _focus)) => {
                                tell(Event::Shot(png));
                                tell(Event::Focus(false));
                            }
                            Err(e) => {
                                stop_full();
                                tell(Event::Shot(Err(format!("{e:#}"))));
                            }
                        }
                    }
                    Task::FullScroll { dy } => {
                        let result = Cdp::connect().and_then(|mut cdp| {
                            cdp.scroll(dy)?;
                            Ok(full_result(&mut cdp))
                        });
                        match result {
                            Ok((png, focus)) => {
                                tell(Event::Shot(png));
                                tell(Event::Focus(focus));
                            }
                            Err(e) => {
                                stop_full();
                                tell(Event::Shot(Err(format!("{e:#}"))));
                            }
                        }
                    }
                    Task::FullReload => {
                        let result = Cdp::connect().and_then(|mut cdp| {
                            cdp.reload()?;
                            Ok(full_result(&mut cdp))
                        });
                        match result {
                            Ok((png, focus)) => {
                                tell(Event::Shot(png));
                                tell(Event::Focus(focus));
                            }
                            Err(e) => {
                                stop_full();
                                tell(Event::Shot(Err(format!("{e:#}"))));
                            }
                        }
                    }
                    Task::PluginDownload { url, path } => {
                        let ended = fetch_plugin(&a, &url, &path, &stopping, &tell);
                        tell(Event::PluginEnded(ended));
                    }
                    Task::FullNavigate { url } => {
                        let result = Cdp::connect()
                            .and_then(|mut cdp| {
                                cdp.navigate(&url)?;
                                cdp.screenshot()
                            })
                            .map_err(|e| format!("{e:#}"));
                        tell(Event::Shot(result));
                    }
                    Task::FullStop => stop_full(),
                }
            })
            .map_err(|e| anyhow::anyhow!("start the browser's worker: {e}"))?;
        Ok(Worker {
            events,
            wake,
            stop,
            agent,
            thread: Some(thread),
        })
    }

    pub fn fd(&self) -> RawFd {
        self.wake.as_raw_fd()
    }

    /// The events since the last call.
    pub fn events(&self) -> Vec<Event> {
        let mut drain = [0u8; 64];
        while unsafe { libc::read(self.wake.as_raw_fd(), drain.as_mut_ptr().cast(), drain.len()) } > 0 {}
        self.events.try_iter().collect()
    }

    /// Stop at once, cutting the connection in use.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(a) = self.agent.lock().expect("the agent is not held by a thread that panicked").take() {
            a.shutdown();
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take()
            && thread.is_finished()
        {
            let _ = thread.join();
        }
    }
}

fn note(line: &str) {
    if let Some(dir) = Path::new(LOG).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG) {
        let _ = writeln!(file, "{line}");
    }
}

fn fetch_plugin(agent: &ureq::Agent, url: &str, path: &Path, stop: &AtomicBool, tell: &dyn Fn(Event)) -> Result<(), String> {
    let started = Instant::now();
    let mut last = Instant::now() - Duration::from_secs(1);
    web::save(agent, url, path, stop, &mut |saving| {
        if last.elapsed() >= PROGRESS_EVERY || saving.total == Some(saving.done) {
            let speed = (saving.done as f64 / started.elapsed().as_secs_f64().max(0.001)) as u64;
            tell(Event::PluginSaved {
                done: saving.done,
                total: saving.total,
                speed,
            });
            last = Instant::now();
        }
    })
    .map(|_| ())
    .map_err(|e| format!("{e:#}"))
}
