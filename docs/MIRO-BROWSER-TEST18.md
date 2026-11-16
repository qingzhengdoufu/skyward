# Miro full browser repair, test18, 2026-11-17

Baseline: test17 / dd364b22e7bafc081302625fa961d14839a2c406.
Only the browser worker, CDP client, stop wiring and a diagnostic example
changed. Installer, decryption, touch driver, device config, modem, super,
7za/zstd, downloads and the pinned Chromium package are unchanged.

## Fault and repair

Test17 device logs showed Page.startScreencast failures: Screencast is already
active and Not attached to an active page. Recovery itself remained PID 410
and its crash buffer was empty. Viewport changes restarted a screencast that
was already running; failed workers also left a browser process behind.

Follow the screenshot-pump approach in Rinrroy/skyward-for-haotian's
browser-inline branch (a929db4 and preceding work), not a wholesale merge:

- Remove screencast startup and restart calls. Capture device-density PNGs
  serially, paced at 100 ms while changing and 300 ms while unchanged.
- Request no replacement while a frame is pending in the UI mailbox.
- Wait for a usable page target rather than only a responding debug port;
  bring the selected page to front before capture.
- Start Chromium at about:blank, then navigate through the attached session.
- Serialize full-session ownership through cleanup. Exiting or failing a
  worker stops its browser process before another worker starts one. Remove
  the separate asynchronous FullStop task that could race a new start.
- Retry a small number of transient screenshot errors. Retain real network
  errors rather than disabling certificate validation.
- Send a mobile user agent with the actual Chromium version so mobile
  viewports receive mobile HTML instead of a clipped desktop layout.

## Upstream comparison

Checked current remote refs on 2026-11-17:

- kirklin/skyward main c5e09f6: the original upstream has no corresponding
  browser/CDP modules to provide this full-mode repair.
- Rinrroy/skyward-for-haotian browser-inline a929db4: UTF-8 entity parsing,
  frame pacing/backpressure and scroll changes. Our test16 already fixed
  the UTF-8 abort independently. This repair reuses its capture strategy.
- Jamvii/skyward-for-aurora browser-v034 ce33a6b: still starts a CDP
  screencast; the pinned browser-v0.3.4-aurora runtime remains our package.
- Jamvii/skyward-for-aurora aera-browser 79374f9 is a different WPE/WebKit
  engine integration, using a sealed shared-pixel memfd plus seqpacket
  control, with zink/Turnip on KGSL. 90c59d4 adds it; b0d77ac adds download
  integration; 79374f9 fixes download lifetime/permissions and keyboard
  response. These are not fixes to the old Chromium/CDP screencast.

Do not claim Aurora's Turnip setup is validated for miro/Adreno 830, or
silently replace the browser engine, device GPU settings or runtime. This
task leaves the AERA route as a separately assessed option.

## Verification

Full workspace tests and strict host/Android Clippy passed. Host Chromium
integration covered text input, mobile UA, repeated keyboard-sized viewports,
reconnection, HTTP link navigation, JavaScript and attachment detection.
Android release build and offline image inspection passed.

An ARM64 browser_probe example, using the same updated CDP source, ran over
ADB on the existing test17 phone without opening DRM or restarting recovery.
It changed only browser test pages and files under /tmp. It passed three
1350px-wide PNG captures at alternating viewport heights, reconnect, and a
real https://cn.bing.com navigation with readyState complete. The mobile-UA
run returned the mobile page and its screenshot was inspected. Recovery PID
410 remained unchanged and the crash buffer was empty during the probe.

This verifies the new CDP path against the actual ARM64 Chromium, not the
complete new recovery UI. Test18 is built and inspected but NOT flashed;
end-to-end opening/closing/full-mode touch in the new recovery is pending.
No audio, arbitrary-site access, frame rate or GPU acceleration is promised.

Image SHA256:
20964e9552fde10391bf407628342d7067c1ad10464589e84ab1359964fa13c7

Evidence in workspace out/: test17-live-20261117-01, test18-host-chromium.log,
test18-browser-probe-bing-mobile.log and test18-browser-probe-bing-mobile.png.
