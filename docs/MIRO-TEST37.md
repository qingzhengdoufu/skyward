# Test37 UI Copy And Navigation Cleanup

2026-11-19, based on the device-accepted test36 / be3e07a. The user requested
less repetitive text and fewer duplicate entries, preserving existing
functionality. The user explicitly requested keeping the PC on this time.

## Changes

- Advanced groups its tools into Files and network, System and partitions,
  and Maintenance. Its redundant Control center entry is removed; the
  existing pull-down and browser fullscreen escape routes are unchanged.
- Terminal has one primary entry in Advanced, not another in Settings.
  The contextual Files terminal and search entry remain.
- The update mirror switch moves from Settings into Skyward update. It
  still controls the same saved preference and SetMirror effect.
- Install drops the long feature inventory. Signature, recovery retention
  and AVB options remain, with concise and more precise notes. Recovery
  retention uses one label on both install and system-update pages.
- OTA download notes drop generic post-update battery advice and duplicated
  resume instructions. The previous unconditional no-data-loss claim is
  replaced by a backup warning respecting package requirements/options.
- Browser history, bookmarks and downloads remain on the toolbar. Their
  redundant menu entries are removed. New tab lives under Tabs. The menu
  retains a single context-specific full/basic mode action, bookmark, home,
  fullscreen, keyboard, zoom, scroll and landscape operations.
- Browser Home keeps active download progress but removes its duplicate
  download row and outdated no-JavaScript description. The engine setup
  entry is named Web engine and uses engine-specific status text.
- Settings and time-zone copy is shortened, preserving the software-only
  password-storage warning and all destructive-operation confirmations.

No installer, AVB, super, 7zz, decryption, device/touch, browser engine,
GPU worker or network implementation changes. Cloud-game ICE is unresolved.
The image still uses the exact test33 browser runtime and jail.

## Validation

1630 passing test invocations, 3 pre-existing ignored. UI tests pass both
default and aera-browser builds. Host and Android all-target Clippy pass.
Native frame/resource/task checks and complete runtime/font verification
pass. Existing static system-init unused -no-pie warning remains.

New regressions cover primary tool routes, mirror preference effects,
browser menu/toolbar reachability and retained install safety options.
The old terminal test now enters through Advanced and still exercises
shell input, control keys, back/reopen and shell restart.

Independent verification matches 382 compiled source/resource files with
the Linux mirror. Compared with test36, only system/bin/recovery changes
in the ramdisk. Only pages.rs, browser.rs and app/regression.rs differ in
compiled source; installer and browser runtime are identical to the baseline.

192 offline previews cover simplified/traditional Chinese and English,
light/dark themes, 1440x3200 and 720x1600 with matching density. Hit bounds
are checked. Representative Advanced, Install, Settings, Update and browser
screens were visually inspected. These are synthetic UI checks, not live
WebKit or phone acceptance. Preview tools remain in audit-tools.

## Identity And Delivery

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test37-aera.img
- Bytes: 104857600; compressed ramdisk: 97560142.
- Image SHA256: ef5cc82a68787d022026cee423c5391d8f17bb43965fafdcd07b92c7345880ad
- ELF SHA256: 9453c9221c3b3bb63ef8b35db98949f00668c2991ca4bf44719e5759fea43d4d
- Runtime SHA256: 894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5

The user authorized source and image upload to qingzhengdoufu/skyward. Source
is staged on experiment/miro-upstream031-native without rewriting main/miro.
The browser-enabled IMG is staged only in an unpublished Release draft,
pending the complete third-party source/license/security-update bundle in
crates/skyward-browser/NOTICE.md. Do not publish that draft automatically.

At the offline checkpoint the phone is disconnected. No test37 flash, phone
copy or Android reboot has yet been performed; test36 is the last accepted
phone image. Device work remains pending reconnection, and the PC stays on.
No ROM installation, data format, AVB change or slot change is authorized.

E backup: E:/K80Pro-Skyward-Backups/miro-test37-ui-cleanup-native.
The unchanged engine/runtime dependencies are preserved in the test36 backup.

## Device Result After Reconnection

The user reconnected miro and authorized flashing test37. This result
supersedes the offline/disconnected status above. Only recovery_a was
written after product, current-slot A, bootloader-mode, unlock and size
checks. Full partition SHA256, /system/bin/recovery and /proc/433/exe match
the verified image. Runtime and jail hashes match; no active-slot setting
or vbmeta changed. Recovery PID 433 remained stable in later observations.

Startup WLAN first returned EBUSY (16), then came up, auto-connected, got
DHCP, reached the internet and synchronized time. Main user 0 data opens;
user 999 remains closed, so no secondary-user acceptance is claimed.
Final recovery crash buffer is empty. Full UI interaction and browser
acceptance remain pending user testing; the screenshot helper did not
produce a capture, so no live page screenshot acceptance is claimed.

The verified image was copied to:
/sdcard/Download/skyward-v1.0.0-miro-test37-aera.img.
Its SHA256 equals the build image. ADB reboot then returned to Android:
sys.boot_completed=1, product miro, slot _a. No ROM install, data format,
AVB modification or slot-setting operation was performed. PC stays on.

Device evidence: D:/k80ptwrp/out/test37-flash-20261119-161621.
Preserve the private raw logs under the E backup's device-acceptance
folder; only this sanitized summary belongs on GitHub.
