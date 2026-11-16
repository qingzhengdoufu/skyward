# Miro test20 checkpoint, 2026-11-17

Baseline: test18 / b8236974e630558563a8f719638fc3a8f8fc87b8.
No changes to installer/super/modem backend, decryption, device GPU setup,
touch driver, AVB behavior, slots or the pinned Chromium archive.

## Changes

- Convert only Bing RSS search URLs when entering Chromium, reopening from
  history or navigating in full mode. Use the existing url crate to parse
  the query correctly. Keep native-reader RSS behavior and unrelated feeds.
- Compact Android-style three-button navigation: 32dp height, centered
  252dp group, filled back triangle, home ring, recent-tasks square. Drawing
  and hit testing share the same rectangles. Preserve haptic preference;
  dragging outside a button cancels it.
- Browser address editor supports tap/drag cursor placement in either
  direction, middle insertion/backspace and a horizontally moving visible
  window. Offsets are checked UTF-8 boundaries. Password fields keep their
  append-at-end behavior; arbitrary selection/copy/paste is not implemented.
- Each download row has a delete-task icon and confirmation. Active writes
  are stopped and their worker allowed to end before part/metadata cleanup.
  Other tasks remain indexed correctly; completed destination files are not
  deleted. A per-file lock prevents deleting a part still being written.
- Chromium plugin fetch uses existing GitHub mirror candidates after the
  original source fails. Every completed plugin download and install must
  match pinned SHA256 9e6df85255386eb962bbe6b1ad5e0d2505c7b5ba02a9d7d89d3d20a778f14ea2.
  Reuse checksum-aware resumable download rather than trusting mirror bytes.
  UI completion is published only when verification ends, not just at 100%.
- Browser event sends are cancellation-aware, so a stopped worker is not
  trapped behind its final progress events in a full queue. Keep normal
  event order and existing newest-frame mailbox/backpressure.

## Verification

Full workspace tests, strict Linux/Android Clippy and release build passed.
Host Chromium navigation/input/viewport/reconnect/JavaScript/attachment
integration passed. Image unpacking verified binary identity, builtin-tool
layout and AVB digest. Source-rendered navigation, keyboard and downloads
previews were visually inspected. No on-device test20 UI validation yet.

Added regressions cover mode switching/history and RSS query preservation,
compact navigation cancellation/haptics, bidirectional address placement and
UTF-8 edits, download deletion confirmation/indexes/file-lock preservation,
plugin verification completion and cancellation of a full event queue.

Mirror HEAD probes from the PC through its existing proxy returned HTTP200,
457339582 bytes and Accept-Ranges for gh-proxy.com and ghproxy.net. ghfast.top
failed a TLS handshake. This is not proof of phone/VPN-free access and not
a full mirrored archive checksum test. Sources can change or fail later.

## Phone/GPU Incident And Remaining Work

Last independently verified phone image was test18 in recovery_a, slot A:
20964e9552fde10391bf407628342d7067c1ad10464589e84ab1359964fa13c7.
Recovery B was unchanged from earlier records.

Read-only GPU checks found msm_kgsl.ko and vendor vulkan.adreno.so, but no
/dev/kgsl-3d0. The renderD128 node existed; it does not alone prove a usable
Adreno graphics driver. No actual GPU/Vulkan device enumeration or offscreen
render succeeded during this run. No GPU acceleration is implemented or
claimed for test20; Chromium remains on its existing software path.

A diagnostic was mistakenly invoked as /system/bin/recovery load msm_kgsl.
Source main.rs dispatches by executable basename: a program called recovery
enters recovery() before parsing CLI subcommands. This invocation therefore
did NOT establish that the GPU module was loaded. It may have started a
second recovery instance and disturbed its display/USB ownership. Do not
repeat it. A future CLI diagnostic must use a distinct executable name
(for example a verified copied binary named skyward-cli), and must never
open DRM while recovery owns the display.

ADB became offline, then disappeared; fastboot and Windows PnP inventories
also showed no phone. Reconnect-offline and restarting only the host ADB
server did not restore it. We cannot distinguish phone hang, USB link loss
or another cause without physical reconnection/reboot and fresh logs.
No partition flashing or formatting was performed by the assistant during
this latest UI/GPU work. The user has been asked to reconnect or long-press
Power if recovery is stuck. Do not assert the phone is already booted.

test20 is built but NOT flashed, GPU is NOT verified, phone system reboot
is NOT completed, and PC shutdown was NOT issued. User requested this order:
finish tests, back up and flash recovery only, verify recovery, reboot phone
to Android and confirm boot, then shut down PC. Preserve that order after
connection returns. Never format data/metadata or flash modem/ROM for this.

Artifact: skyward-v1.0.0-miro-upstream031-native-stock303-test20.img
SHA256: cfe2314d20797cc6a5b3fd52171eb846fc72a226b13d8906a0c85d79fe73cc12

Test19 was an interim RSS-only build and was not flashed. Test20 contains
that fix plus this interface/download work. Use verified test20 source and
image together; older backups remain intact.
