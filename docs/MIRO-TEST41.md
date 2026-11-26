# Miro Test41: Gestures and On-Demand Root Installation

Base: test40, commit `234939783705372f7c0360f7b4aef973e40ddd71`.
This remains a private browser-enabled build under the restrictions in
`crates/skyward-browser/NOTICE.md`.

## Scope

- A 24 dp bottom gesture area replaces the three-button navigation bar.
  Swipe up at least 52 dp and release to return to Skyward home. Pause
  after swiping up for 300 ms to open recent tasks. Existing immersive
  browser edge-exit gestures remain unchanged.
- Recent tasks retain their page stack and scroll position. Preview
  thumbnails are 240 pixels wide, limited to twelve task groups. Invalid
  detail pages fall back to a valid parent. Credential entry pages do
  not create preview cards. Switching pages does not stop an installer.
- The file manager's plus button opens a new-folder dialog using the
  existing English/Chinese input method and ownership-aware exclusive
  directory creation. The terminal remains available through its
  existing home/control-center entry.
- KernelSU installation is explicit, not a ROM-install post-action.
  Only official ARM64 `ksud-aarch64-linux-android` and
  `lkm-aarch64-android15-6.6_kernelsu.ko` resources are downloaded.
  No APK, Magisk installer, other KMI module, or Root payload is embedded.
- Module ZIPs are staged privately and validated for a root-level
  `module.prop` before the existing KernelSU or Magisk manager executes
  them. This does not install Magisk and does not change ROM ZIP handling.

## Root Safety and Storage

KernelSU requires decrypted data, an attached system, a confirmed unlocked
bootloader, a known current slot, and no pending OTA snapshots. The recovery
kernel and installed boot kernel must identify the android15-6.6 KMI.
Downloaded resources must match official release size and SHA-256 metadata;
their ELF architecture and module vermagic are checked before execution.
Matching cached resources can be reused after a network failure. The
SHA-256 metadata is an integrity check delivered over HTTPS, not an
independent publisher signature.

Resources: `/data/skyward-kernelsu/`. Temporary patch/module staging is
removed when the worker ends. Cached releases are not automatically deleted.
Stock init_boot backup and preparation record:
`/data/media/0/Skyward/root/`.

Only current-slot init_boot is written. Existing Root ramdisks are refused
instead of patched twice. The original backup is read back; the patched
image is parsed and checked for the expected ramdisk-only layout and Root
payload. Slot, boot and init_boot are checked again before writing. The
existing image writer performs full readback verification. No automatic
reboot is performed. The manager APK must be installed separately in Android.

Module scripts run with recovery privileges and must be trusted. Script
output is bounded; patching has a 120-second limit and module installation
a 600-second limit. A script timeout does not roll back its partial changes.

## Deliberately Unchanged

No automatic snapshot merge was added. Existing manual merge and recovery
retention code are unchanged. ROM installation, ZIP extraction, update_engine,
slot switching, AVB handling, recovery restoration, modem preparation, device
configuration, decryption, touch, WiFi, vibration, and browser runtime remain
the test40 implementations. The only addition to image.rs is a read-only
Root ramdisk marker query for the separate Root workflow.

## Validation

See `out/build-test41-native.log`, `out/preview-test41.log`, and
`out/test41-independent-report.json` in the local build workspace.
The independent verifier checks protected source paths and compares the
ramdisk inventory with test40. Test41 changes only system/bin/recovery
inside the image. This build has not yet been flashed or tested on-device;
no Root install has been executed.

Reference implementation reviewed: Kinaxie/skyward at
`127930f9d8f002716c552c0559cbe7e1c2966fb4`. Root payloads are obtained
on demand from `https://github.com/tiann/KernelSU/releases/latest`.
