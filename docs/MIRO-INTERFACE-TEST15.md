# Miro Interface and Browser Test15

## Status

Version remains 1.0.0. Test15 is built and verified offline, not flashed.
The user's latest instruction is to show previews and not flash the phone.
Do not flash until a new explicit request. No automatic shutdown is requested
for this session.

The phone was previously flashed with test12 only on recovery_a. Its SHA-256 is
180db2469056b228ec69300a8123fa195503174e57c47f2d35e8ce837e01ff49.
The checksum-verified Aurora Chromium package was subsequently installed at
/data/skyward-browser/chromium after owner storage was decrypted. Chromium's
version command returned 156.0.8078.79. No system, data or metadata formatting,
ROM flashing, AVB changes, or slot changes were performed in this session.

## Interface

- Follow the user's Android control-center reference: large clock/date, a dark
  three-row/four-column shortcut grid, a filled brightness slider, status and
  green download progress. Only supported recovery functions are exposed.
- Pull from the status bar to reveal the panel with the finger; release snaps
  it open or closed. Swipe upward or use Back to dismiss it. The underlying
  page and its scroll position are retained. Brightness dragging stays separate.
- Add Back, Home and Tasks at the bottom. Content, browser tools and keyboards
  reserve their space. Fastboot keeps its original dedicated key workflow.
- Home parks the page rather than cancelling an installation/download. Tasks
  can resume it. Returning from the browser address field reuses its page
  rather than pushing a new recovery page for every URL.

## Native Browser

The native HTML reader remains the default. It draws text with the recovery's
fonts directly. It supports the existing limited HTML/CSS/forms/cookies, but
does not execute JavaScript or provide a complete modern browser engine.

- Compact the address/title area, remove the duplicate browser title/card and
  repeated URL so the initial viewport contains more of the actual document.
- Split long paragraphs and plain text instead of silently dropping characters
  after 4000. Preserve code whitespace and render more than 40 code lines.
- Preserve document headers, sidebars and noscript fallbacks rather than limiting
  extraction to the first main/article region.
- Measure styled headings and paragraphs with the same font/size/pitch used to
  render them. Wrap long link labels. Retain full document title/description.
- Respect quoted greater-than characters when finding the end of an HTML tag.

This is not a guarantee of every character on every website. The page reader
still bounds HTML at 2 MiB and 400 blocks, with a truncation indication. Dynamic
JavaScript content and character sets outside the current parser/font support
may require Chromium. No unsupported rendering capability is claimed.

## External Chromium

- Replace per-gesture connection creation and fixed delays with controls queued
  to a single persistent CDP session. Consecutive touch moves coalesce without
  dropping the initial down or final up event.
- Forward down/move/up immediately instead of simulating scrolling only on
  finger release. Viewport dimensions follow the actual content area, including
  the on-screen keyboard.
- Use device-density lossless PNG captures. Host testing showed Chromium's
  screencast still delivered CSS-pixel-sized images, so those frames are used
  as invalidations, not displayed. The subsequent PNG capture is verified at
  1350 pixels wide in the host integration test.
- Keep only the newest frame and cache decoded pixels for band rendering.
  Separate running-session controls from one-shot page/download/install tasks.

Full mode remains experimental. Lossless capture may still be costly on a phone;
there is no zero-copy/native Chromium surface or GPU acceleration claim. No
audio fix, latency guarantee or ARM64 interaction validation is claimed for test15.

## Verification and Artifacts

- Full workspace tests passed, including 413 UI and 84 network tests.
- Host and Android all-target Clippy with warnings denied passed.
- Android release binary, static system-init and recovery packer built.
  The existing nonfatal system-init linker warning about -no-pie remains.
- Real host Chromium test passed: navigation, input, lossless capture/stream
  width, and attachment-download events.
- Image independently unpacked: AVB digest, 100 MiB size, recovery binary,
  device config, OMAPI and builtin tool symlinks verified.
- Chinese/English, light/dark source-rendered previews generated and inspected.
  They are previews, not captures from the phone.
- No edits to device configuration, decryption backend, touch/haptic backend,
  install/super logic or builtin 7za/unzip/zstd/avbctl/resetprop implementations.

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test15.img

SHA-256: 63a701c6eb603fc3d21af28dba7929034c6aa027464eb56ff0eea5162f74ec05

Next verification requires explicit flashing approval, then phone checks of
data decryption, navigation, brightness, shade gestures and both browser modes.
