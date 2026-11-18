# Test34 Timed Control-Center Navigation

2026-11-19, private development build based on test33 / b01760b.

## Root Cause And Fix

The WiFi tile's long hold is emitted by App::tick, not its touch release.
Opening WLAN emits ScanWlan as well as haptic feedback. Pages::tick only
accepted motor effects and aborted on ScanWlan through unreachable!. This
explains the recovery restart reported after holding WiFi in test33.

Timed actions now use the same FIFO dispatcher as touch/key actions, including
follow-up effects and handler errors. Host tests cover non-motor timed effects,
FIFO order, error propagation, destination composition and consumed release.
No installer, super, 7zz, AVB, decryption, device or browser change is included.

## Verification

Build: 1616 passing test invocations, 3 existing ignored. Host and Android
all-target Clippy, native frame/resource/task checks and complete browser
runtime/font extraction pass. Independent verification compares 381 source
and resource files with the Linux build mirror. Only system/bin/recovery
changes in the ramdisk relative to test33; the browser runtime is identical.

Only recovery_a was flashed after product/slot/size/unlock checks. The full
partition, running ELF and runtime hashes match. Slot remains A. An actual
800 ms WiFi hold opens WLAN, displays the connected network and scans nearby
access points. PID 432 remains the same before/after; the crash buffer is
empty. Screenshot: out/test34-flash-20261119-044753/ui-wifi-long-hold.png.
Subsequent log inspection still showed PID 432 with repeated successful scans.

Startup WLAN initially returned EBUSY (16); one short toggle brought it up,
auto-connected, acquired DHCP and synchronized time. This existing startup
issue is recorded, not claimed fixed. Data was not decrypted during this
boot, so test34 makes no new decryption acceptance claim. No ROM install,
format, AVB modification or slot-setting operation was performed.

The user later disconnected the phone. Browser changes are paused and
test33 cloud-game findings remain unresolved; preserve that source/progress.

## Identity

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test34-aera.img
- Image bytes: 104857600; compressed ramdisk: 97560441.
- Image SHA256: a64bb9c6efa004704e47b73261abe442005a8102b6f31021233ec826dadbf82d
- ELF SHA256: 9f6be5fcb242b64444b7c61fd269fee228d7a7c14c794c0fa95498aac71c8d6e
- Runtime SHA256: 894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5
- Flash evidence: D:/k80ptwrp/out/test34-flash-20261119-044753.
- Build and independent reports: D:/k80ptwrp/out/*test34*.

The AVB footer's digest is verified, but algorithm NONE is unsigned and is
not an OEM-authenticated image. No public distribution is authorized.
