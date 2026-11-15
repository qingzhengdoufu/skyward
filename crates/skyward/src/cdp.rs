//! A small Chrome DevTools Protocol client for the persistent full-mode
//! Chromium. It speaks just enough WebSocket and CDP to keep one page alive
//! and send it real touch, pinch and text input.

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

const PORT: u16 = 9222;
const VIEW_W: u32 = 412;
const VIEW_H: u32 = 915;
/// Render at a phone density; a whole-page frame is kept small enough to
/// decode comfortably in recovery memory.
const DEVICE_SCALE: f64 = 2.0;
pub struct Cdp {
    stream: TcpStream,
    next: u64,
    pending: VecDeque<Value>,
}

pub enum Event {
    Frame(Vec<u8>),
    Download { url: String, name: String },
    Navigated(String),
    Open(String),
}

impl Cdp {
    pub fn connect() -> Result<Cdp> {
        let deadline = Instant::now() + Duration::from_secs(15);
        let list = loop {
            match http_get("/json/list") {
                Ok(text) => break text,
                Err(e) if Instant::now() < deadline => {
                    let _ = e;
                    std::thread::sleep(Duration::from_millis(200));
                }
                Err(e) => return Err(e).context("wait for Chromium's debugging port"),
            }
        };
        let pages: Value = serde_json::from_str(&list).context("read Chromium's page list")?;
        let ws = pages
            .as_array()
            .and_then(|pages| pages.iter().find(|page| page["type"] == "page"))
            .and_then(|page| page["webSocketDebuggerUrl"].as_str())
            .context("Chromium lists no page to attach to")?;
        let path = ws
            .strip_prefix("ws://127.0.0.1:9222")
            .or_else(|| ws.strip_prefix("ws://localhost:9222"))
            .context("unexpected debugging socket")?;
        let mut stream = TcpStream::connect(("127.0.0.1", PORT)).context("connect to Chromium")?;
        stream.set_read_timeout(Some(Duration::from_secs(20)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let key = "U2t5d2FyZC1icm93c2VyIQ==";
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )?;
        let headers = read_headers(&mut stream)?;
        if !headers.starts_with("HTTP/1.1 101") && !headers.starts_with("HTTP/1.0 101") {
            bail!("Chromium refused the debugging socket: {}", headers.lines().next().unwrap_or(""));
        }
        let accept = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("sec-websocket-accept"))
            .map(|(_, value)| value.trim());
        anyhow::ensure!(accept == Some(accept_for(key).as_str()), "invalid WebSocket handshake");
        let mut cdp = Cdp {
            stream,
            next: 1,
            pending: VecDeque::new(),
        };
        cdp.call("Page.enable", json!({}))?;
        cdp.call("Page.addScriptToEvaluateOnNewDocument",json!({"source":"window.open=(url)=>{location.href=new URL(url,location.href).href;return window};document.addEventListener('click',e=>{const a=e.target.closest?.('a[target]');if(a)a.target='_self'},true)"}))?;
        cdp.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width": VIEW_W, "height": VIEW_H, "deviceScaleFactor": DEVICE_SCALE, "mobile": true}),
        )?;
        cdp.call("Emulation.setTouchEmulationEnabled", json!({"enabled": true, "maxTouchPoints": 5}))?;
        Ok(cdp)
    }

    pub fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next;
        self.next += 1;
        let message = json!({"id": id, "method": method, "params": params});
        write_frame(&mut self.stream, message.to_string().as_bytes())?;
        loop {
            let payload = read_frame(&mut self.stream)?;
            let answer: Value = match serde_json::from_slice(&payload) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if answer["id"].as_u64() != Some(id) {
                if matches!(
                    answer["method"].as_str(),
                    Some("Page.screencastFrame" | "Browser.downloadWillBegin" | "Page.frameNavigated" | "Page.windowOpen")
                ) && self.pending.len() < 8
                {
                    self.pending.push_back(answer);
                }
                continue;
            }
            if let Some(error) = answer.get("error") {
                bail!("{method}: {error}");
            }
            return Ok(answer.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    pub fn navigate(&mut self, url: &str) -> Result<()> {
        self.call("Page.navigate", json!({"url": url}))?;
        std::thread::sleep(Duration::from_millis(1200));
        Ok(())
    }

    pub fn reload(&mut self) -> Result<()> {
        self.call("Page.reload", json!({}))?;
        std::thread::sleep(Duration::from_millis(900));
        Ok(())
    }

    pub fn touch(&mut self, x: u32, y: u32) -> Result<()> {
        let point = json!([{"x": x, "y": y, "id": 1, "radiusX": 2, "radiusY": 2, "force": 1.0}]);
        self.call("Input.dispatchTouchEvent", json!({"type": "touchStart", "touchPoints": point}))?;
        std::thread::sleep(Duration::from_millis(60));
        self.call("Input.dispatchTouchEvent", json!({"type": "touchEnd", "touchPoints": []}))?;
        std::thread::sleep(Duration::from_millis(500));
        Ok(())
    }

    pub fn pinch(&mut self, x: u32, y: u32, scale: f64) -> Result<()> {
        self.call(
            "Input.synthesizePinchGesture",
            json!({"x": x, "y": y, "scaleFactor": scale, "relativeSpeed": 800}),
        )?;
        std::thread::sleep(Duration::from_millis(600));
        Ok(())
    }

    pub fn insert_text(&mut self, text: &str) -> Result<()> {
        self.call("Input.insertText", json!({"text": text}))?;
        std::thread::sleep(Duration::from_millis(300));
        Ok(())
    }

    pub fn focus_is_text(&mut self) -> bool {
        let answer = self.call(
            "Runtime.evaluate",
            json!({"expression": "(()=>{const e=document.activeElement;return e?`${e.tagName}:${e.type||''}:${e.isContentEditable}`:''})()", "returnByValue": true}),
        );
        let Ok(answer) = answer else { return false };
        let value = answer["result"]["value"].as_str().unwrap_or("");
        value.starts_with("INPUT") || value.starts_with("TEXTAREA") || value.ends_with(":true")
    }

    pub fn screenshot(&mut self) -> Result<Vec<u8>> {
        let answer = self.call(
            "Page.captureScreenshot",
            json!({"format": "jpeg", "quality": 82, "fromSurface": true}),
        )?;
        let data = answer["data"].as_str().context("screenshot holds no data")?;
        base64::engine::general_purpose::STANDARD.decode(data).context("decode screenshot")
    }

    /// Start the continuous frame stream; each [`Cdp::next_frame`] acknowledges
    /// the previous frame before returning the next.
    pub fn start_screencast(&mut self) -> Result<()> {
        self.call("Browser.setDownloadBehavior", json!({"behavior":"deny","eventsEnabled":true}))?;
        self.call(
            "Page.startScreencast",
            json!({
                "format": "jpeg",
                "quality": 72,
                "maxWidth": VIEW_W * 2,
                "maxHeight": VIEW_H * 2,
                "everyNthFrame": 1
            }),
        )?;
        Ok(())
    }

    pub fn next_event(&mut self) -> Result<Option<Event>> {
        loop {
            let event = if let Some(event) = self.pending.pop_front() {
                event
            } else {
                let mut fd = libc::pollfd {
                    fd: self.stream.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                let ready = unsafe { libc::poll(&mut fd, 1, 500) };
                if ready == 0 {
                    return Ok(None);
                }
                if ready < 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
                let payload = read_frame(&mut self.stream)?;
                match serde_json::from_slice::<Value>(&payload) {
                    Ok(value) => value,
                    Err(_) => continue,
                }
            };
            if event["method"] == "Browser.downloadWillBegin" {
                let params = &event["params"];
                return Ok(Some(Event::Download {
                    url: params["url"].as_str().unwrap_or_default().into(),
                    name: params["suggestedFilename"].as_str().unwrap_or("download.bin").into(),
                }));
            }
            if event["method"] == "Page.windowOpen"
                && let Some(url) = event["params"]["url"].as_str()
            {
                return Ok(Some(Event::Open(url.into())));
            }
            if event["method"] == "Page.frameNavigated" {
                let frame = &event["params"]["frame"];
                if frame.get("parentId").is_none()
                    && let Some(url) = frame["url"].as_str()
                {
                    return Ok(Some(Event::Navigated(url.into())));
                }
            }
            if event["method"] != "Page.screencastFrame" {
                continue;
            }
            let params = &event["params"];
            let session = params["sessionId"].as_i64().unwrap_or(0);
            let ack = json!({"id": self.next, "method": "Page.screencastFrameAck", "params": {"sessionId": session}});
            self.next += 1;
            write_frame(&mut self.stream, ack.to_string().as_bytes())?;
            let data = params["data"].as_str().context("frame holds no data")?;
            return Ok(Some(Event::Frame(
                base64::engine::general_purpose::STANDARD.decode(data).context("decode frame")?,
            )));
        }
    }

    pub fn scroll(&mut self, dy: i64) -> Result<()> {
        self.call(
            "Input.synthesizeScrollGesture",
            json!({
                "x": VIEW_W / 2,
                "y": VIEW_H / 2,
                "xDistance": 0,
                "yDistance": dy,
                "speed": 900,
                "gestureSourceType": "touch"
            }),
        )?;
        std::thread::sleep(Duration::from_millis(500));
        Ok(())
    }
}

fn http_get(path: &str) -> Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", PORT)).context("connect to Chromium")?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nConnection: close\r\n\r\n")?;
    let headers = read_headers(&mut stream)?;
    anyhow::ensure!(
        headers.starts_with("HTTP/1.1 200") || headers.starts_with("HTTP/1.0 200"),
        "Chromium page list request failed"
    );
    let length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .context("Chromium page list has no length")?;
    anyhow::ensure!(length <= 1024 * 1024, "Chromium page list is too large");
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body)?;
    String::from_utf8(body).context("invalid Chromium page list")
}

fn read_headers(stream: &mut TcpStream) -> Result<String> {
    let mut bytes = Vec::new();
    let mut one = [0u8; 1];
    while bytes.len() < 16384 {
        let n = stream.read(&mut one)?;
        if n == 0 {
            break;
        }
        bytes.push(one[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<()> {
    write_opcode(stream, 1, payload)
}

fn write_opcode(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<()> {
    let mut header = Vec::with_capacity(14);
    header.push(0x80 | opcode);
    let mut mask = [0u8; 4];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut mask)?;
    if payload.len() < 126 {
        header.push(0x80 | payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        header.push(0x80 | 126);
        header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        header.push(0x80 | 127);
        header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    header.extend_from_slice(&mask);
    stream.write_all(&header)?;
    let mut masked = payload.to_vec();
    for (i, byte) in masked.iter_mut().enumerate() {
        *byte ^= mask[i % 4];
    }
    stream.write_all(&masked)?;
    stream.flush()?;
    Ok(())
}

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut whole = Vec::new();
    loop {
        let mut two = [0u8; 2];
        stream.read_exact(&mut two)?;
        let fin = two[0] & 0x80 != 0;
        let opcode = two[0] & 0x0f;
        let masked = two[1] & 0x80 != 0;
        let mut len = (two[1] & 0x7f) as u64;
        if len == 126 {
            let mut bytes = [0u8; 2];
            stream.read_exact(&mut bytes)?;
            len = u16::from_be_bytes(bytes) as u64;
        } else if len == 127 {
            let mut bytes = [0u8; 8];
            stream.read_exact(&mut bytes)?;
            len = u64::from_be_bytes(bytes);
        }
        if len > 16 * 1024 * 1024 || len.saturating_add(whole.len() as u64) > 16 * 1024 * 1024 {
            bail!("Chromium sent an oversized frame");
        }
        let mask = if masked {
            let mut mask = [0u8; 4];
            stream.read_exact(&mut mask)?;
            Some(mask)
        } else {
            None
        };
        let mut payload = vec![0u8; len as usize];
        stream.read_exact(&mut payload)?;
        if let Some(mask) = mask {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[i % 4];
            }
        }
        if opcode == 0x8 {
            return Err(anyhow!("Chromium closed the debugging socket"));
        }
        if opcode == 0x9 {
            write_opcode(stream, 0xA, &payload)?;
            continue;
        }
        if opcode == 0xA {
            continue;
        }
        whole.extend_from_slice(&payload);
        if fin {
            return Ok(whole);
        }
    }
}

pub const WIDTH: u32 = VIEW_W;
pub const HEIGHT: u32 = VIEW_H;

fn accept_for(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let server = listener.accept().unwrap().0;
        (client, server)
    }
    #[test]
    fn websocket_handshake_matches_the_rfc_example() {
        assert_eq!(accept_for("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
    #[test]
    fn fragmented_frames_are_joined_and_oversized_frames_are_refused() {
        let (mut client, mut server) = pair();
        server.write_all(&[0x01, 2, b'a', b'b', 0x80, 2, b'c', b'd']).unwrap();
        assert_eq!(read_frame(&mut client).unwrap(), b"abcd");
        server.write_all(&[0x81, 127]).unwrap();
        server.write_all(&(17u64 * 1024 * 1024).to_be_bytes()).unwrap();
        assert!(read_frame(&mut client).unwrap_err().to_string().contains("oversized"));
    }
    #[test]
    fn outgoing_frames_are_masked_and_can_be_decoded() {
        let (mut client, mut server) = pair();
        write_frame(&mut client, b"test").unwrap();
        assert_eq!(read_frame(&mut server).unwrap(), b"test");
    }
    #[test]
    fn an_idle_screencast_is_not_an_error_and_does_not_wait_forever() {
        let (client, _server) = pair();
        let mut cdp = Cdp {
            stream: client,
            next: 1,
            pending: VecDeque::new(),
        };
        let start = Instant::now();
        assert!(cdp.next_event().unwrap().is_none());
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn events_before_a_command_reply_are_kept_for_the_stream() {
        let (client, mut server) = pair();
        let writer = std::thread::spawn(move || {
            let call: Value = serde_json::from_slice(&read_frame(&mut server).unwrap()).unwrap();
            write_frame(
                &mut server,
                json!({"method":"Page.screencastFrame","params":{"sessionId":1,"data":"dGVzdA=="}})
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
            write_frame(&mut server, json!({"id":call["id"],"result":{}}).to_string().as_bytes()).unwrap();
            let ack: Value = serde_json::from_slice(&read_frame(&mut server).unwrap()).unwrap();
            assert_eq!(ack["method"], "Page.screencastFrameAck");
        });
        let mut cdp = Cdp {
            stream: client,
            next: 1,
            pending: VecDeque::new(),
        };
        cdp.call("Page.startScreencast", json!({})).unwrap();
        assert!(matches!(cdp.next_event().unwrap(),Some(Event::Frame(data)) if data==b"test"));
        writer.join().unwrap();
    }

    #[test]
    #[ignore = "requires a host Chromium executable"]
    fn host_chromium_navigation_input_and_screencast() {
        struct Running(std::process::Child);
        impl Drop for Running {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let binary = std::env::var("SKYWARD_TEST_CHROME").unwrap();
        let port = TcpListener::bind(("127.0.0.1", PORT)).expect("test CDP port must be free");
        drop(port);
        let profile = tempfile::tempdir().unwrap();
        let _chrome = Running(
            std::process::Command::new(binary)
                .args([
                    "--headless=new",
                    "--no-sandbox",
                    "--disable-gpu",
                    "--disable-dev-shm-usage",
                    "--remote-debugging-address=127.0.0.1",
                    "--remote-debugging-port=9222",
                    "--no-first-run",
                    "--no-default-browser-check",
                ])
                .arg(format!("--user-data-dir={}", profile.path().display()))
                .arg("about:blank")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
        let mut cdp = Cdp::connect().unwrap();
        cdp.navigate("data:text/html,<title>One</title><h1>Skyward browser</h1><input id='q' style='position:absolute;left:10px;top:100px;width:300px;height:50px'>").unwrap();
        let shot = cdp.screenshot().unwrap();
        assert!(shot.starts_with(&[0xff, 0xd8]));
        cdp.touch(30, 120).unwrap();
        assert!(cdp.focus_is_text());
        cdp.insert_text("recovery test").unwrap();
        let value = cdp
            .call(
                "Runtime.evaluate",
                json!({"expression":"document.getElementById('q').value","returnByValue":true}),
            )
            .unwrap();
        assert_eq!(value["result"]["value"], "recovery test");
        cdp.navigate("data:text/html,<title>Two</title><h1>Navigation passed</h1>").unwrap();
        let title = cdp
            .call("Runtime.evaluate", json!({"expression":"document.title","returnByValue":true}))
            .unwrap();
        assert_eq!(title["result"]["value"], "Two");
        cdp.start_screencast().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(Event::Frame(frame)) = cdp.next_event().unwrap() {
                assert!(frame.starts_with(&[0xff, 0xd8]));
                if let Ok(path) = std::env::var("SKYWARD_BROWSER_SHOT") {
                    std::fs::write(path, frame).unwrap();
                }
                break;
            }
            assert!(Instant::now() < deadline, "Chromium did not stream a frame");
        }
        let server = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = server.local_addr().unwrap();
        let response = std::thread::spawn(move || {
            let (mut socket, _) = server.accept().unwrap();
            let _ = read_headers(&mut socket);
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=recovery.img\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest").unwrap();
        });
        cdp.navigate(&format!("http://{address}/download")).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(Event::Download { url, name }) = cdp.next_event().unwrap() {
                assert!(url.ends_with("/download"));
                assert_eq!(name, "recovery.img");
                break;
            }
            assert!(Instant::now() < deadline, "Chromium did not report the attachment download");
        }
        response.join().unwrap();
    }
}
