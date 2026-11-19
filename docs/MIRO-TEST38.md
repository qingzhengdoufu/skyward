# Test38 Image Restore, Shared Input And Script Warnings

2026-11-19, uncommitted validation build based on test37 / bfdac6b.
The user requested building/flashing before GitHub submission, then testing
personally. Do not commit/push/upload this revision until acceptance.
After acceptance and submission the user requested PC hibernation, not
shutdown. No PC power operation is authorized before that sequence finishes.

## Image Backup Restore

The old restore page scanned Skyward/backups but raw partition exports were
stored in Skyward/images. Restore now combines those roots in available
internal storage and USB. Existing image/checksum pairs need no migration.
Each export directory must hold exactly one regular allowed-partition IMG
and its matching SHA-256 sidecar; malformed/incomplete candidates are not
offered. Symlink image/directory entries are refused.

The confirmation page names the original partition, exact size and location.
It does not silently retarget boot_a to boot_b after a slot switch. Raw
exports have no recorded device identity; the page warns to confirm origin.
Shared super is explicitly both-slot content and requires reboot after
restoration. No userdata/metadata/identity partition restore is added.

Before writing: reject an in-flight OTA, invalid target/name, changed backup
metadata, partition size mismatch and full source SHA-256 mismatch. Keep
the verified file descriptor open, detach logical partitions before shared
super, flush writes, invalidate cached target pages and hash the entire
restored partition. Cancellation applies before writing, not mid-partition.
Regular-file host tests exercise the same copy/readback implementation.
No real phone restore was performed during validation.

## Chinese And English Text Input

One shared candidate-strip renderer reuses the existing offline AOSP pinyin
decoder. Browser address/page input retains its existing state and routing.
Native fields now use a separate composition state for feature/partition
search, file search, rename, WLAN name/password, letter unlock, terminal,
editor and basic-browser forms. Numeric PIN/pattern controls are unchanged.
Language defaults to English on page/field entry; composition clears when
changing pages or credential fields and is not persisted in settings.
No dictionary learning/selection API is invoked.

Candidates and composing spelling occupy reserved space above keys. Terminal
and editor control rows are moved clear, reducing the visible text rows.
Backspace first edits composition; Space/Enter with active spelling commits
one candidate without submitting a field or running a shell command. A
subsequent Enter retains the page's original action. Shell Ctrl shortcuts
remain keys, not pinyin. Tests cover Chinese/English edits, Unicode deletion,
field resets, candidate geometry and terminal/editor behavior.

## Script Output Warnings

No shell scanner, unknown-command skipping, no-op checkDevice, injected
set -e/pipefail or exit-code rewrite is added. The installer's output path
classifies actual missing-command/file, permission/read-only, capacity,
I/O and selected unpack diagnostics. Six advisory categories are deduplicated.
Raw output still goes to install-script.log. Diagnostics may be benign or
miss other failures: they are not proof a command failed or all partitions
were verified. An exit-0 script with warnings shows an amber completed-with-
warnings result. Original nonzero outcomes remain failures.

## Verification And Device State

1644 passing test invocations, 3 existing ignored. Host and Android
all-target Clippy pass. Native frame/resource/task and full runtime/font
checks pass. Existing system-init unused -no-pie linker warning remains.
216 UI previews cover simplified/traditional Chinese and English, both
themes, 720x1600 and 1440x3200. Representative search, image restore and
warning screens were visually inspected.

Independent verification compares 385 source/resource files with the Linux
mirror; relative to test37 only system/bin/recovery changes in the ramdisk.
Crypto, device config, AVB/super/7za command logic and browser runtime remain
unchanged. Test33 browser/WebRTC limitations, including unresolved cloud-game
ICE and absent hardware sound, remain.

Only recovery_a flashed after identity/slot/unlock/size checks. Full partition
and running /proc/399/exe match. Slot remains _a. Main user 0 decrypts, WiFi
connects, internet/time sync work, final crash buffer empty. User 999 remains
closed; no secondary-user acceptance is claimed. Existing boot_a image pair
is still present under Skyward/images/20261119-170607-boot_a. No install,
real restore, data format, AVB change or slot-setting action was performed.
Phone remains in recovery for user testing; PC is still on.

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test38-aera.img
- Bytes: 104857600; compressed ramdisk: 97576719.
- Image SHA256: 4ffc1ed63ec9391e509d3e0d4430d5b1ab56739fbb43b77d79be22f69b4cd58b
- ELF SHA256: 40fb51b4c1232572333426ba2559b4159abc282357417569b0d10747a9ee64e8
- Runtime SHA256: 894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5
- Flash evidence: D:/k80ptwrp/out/test38-flash-20261119-180210.
- Backup: E:/K80Pro-Skyward-Backups/miro-test38-input-restore-validation.

Pending: user acceptance of restore discovery, real Chinese input and script
warning presentation. Do not flash a ROM or restore partitions just to
demonstrate the UI. Preserve the unpublished browser-runtime distribution
gate: no public IMG release without resolving NOTICE.md requirements.

## User Acceptance And Delivery

The user subsequently reported all tested functions working and requested
the image copied to phone storage. This supersedes the pending/uncommitted
delivery status above and authorizes committing/pushing the accepted source.
The user report is not a claim that every image/partition or secondary user
was exhaustively tested. No additional real partition restore was performed
by the agent for acceptance.

Test38 IMG is saved as /sdcard/Download/skyward-v1.0.0-miro-test38-aera.img.
The phone file and entire recovery_a partition both have the expected
4ffc1ed63ec9391e509d3e0d4430d5b1ab56739fbb43b77d79be22f69b4cd58b
SHA256; /proc/399/exe matches the test38 ELF. Slot remains _a. The copied
file is media_rw:media_rw 0660 with the existing Android media label.

The user's subsequent script-ROM test reached exit 0, and recovery retention
wrote test38 back to recovery_a successfully. The known undefined checkDevice
and FRP end-of-device messages remain in the raw script log, as intended:
diagnostics do not skip commands or rewrite the script's result. Full
corresponding super readback and new-ROM Android boot are not claimed.
Raw acceptance logs are private; only this summary is pushed to GitHub.

Source is synchronized on experiment/miro-upstream031-native, without
rewriting main/miro. IMG and SHA256SUMS go to an unpublished test38 Release
draft, not a public release or automatic online update. Third-party runtime
distribution requirements still apply. Local accepted source/history and
delivery evidence are retained in the E backup's accepted-delivery folder.

Phone stays in recovery. PC hibernation follows only after the requested
GitHub synchronization and verified backups complete. This document records
the requested sequence, not proof the computer actually entered hibernation.
