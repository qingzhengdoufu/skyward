# Test39b Stability Audit And Validation Checkpoint

2026-11-19. Based on local test38 commit 386bc1d. Development validation build.

## Incident And Evidence

The user reported an intermittent recovery exit while using Chinese search,
followed by a data-open failure. They clarified that they manually re-entered
recovery and suspected the post-ROM-install environment. The original failing
process log is unavailable: the next capture is a fresh kernel boot. Therefore
the original crash's root cause is NOT established, and data corruption or a
permanent decryption regression is NOT inferred from the report.

Test38's fresh boot decrypted owner 0 successfully. User 999 remained closed;
that alone is not an owner-data failure. Test39b also decrypted owner 0, with
/data and /sdcard accessible. No data, metadata, locksettings or key files were
deleted, formatted or restored during this investigation.

## Audit Findings And Changes

- The installer detaches the installed system before writing, but does not
  reinitialize its hardware-service environment afterward. An explicit
  Detached event now marks this state, including a partially failed detach.
  This state remains across subsequent packages in the same recovery boot.
- Home and the completed-install page offer a recovery-restart notice.
  Home does not offer the shortcut while installation is active. There is no
  fake 15-second recovery timer, forced reboot, or change to ZIP queue logic.
- Starting a new decrypt job after detach reports that recovery must restart.
  Already-open data remains open; the detach event does not relock it.
- A late keyboard release could reach a page whose text field had disappeared
  after an asynchronous state change. Text actions now require a valid field,
  and closed file-search fields ignore late keys. Browser form indexes are
  checked. Ordinary Latin and Chinese composition remain available.
- A pattern finishing after automatic unlocking could reach an unreachable
  branch. Unlock now accepts only a waiting, non-checking lock. Regression
  tests cover both keyboard and pattern releases during automatic unlock.
- Duplicate decrypt requests no longer terminate the main loop. Failure to
  create a manual decrypt worker is reported in its page instead of exiting.
- Optional diagnostics in readings, settings, decrypt errors and MTP failure
  handling no longer propagate logger failures into the recovery loop.
- Recovery stderr is /dev/null on this device. A bounded, best-effort panic
  hook now writes source location only to /skyward/run/panic.log and /dev/kmsg.
  It uses no shared logger mutex and does not record panic payloads or user
  input. Export logs includes panic.log when present. It does not catch a
  panic, hide installation errors or claim to survive native signals.

The audit concentrated on native UI state/input/rendering, the main event
loop and its async workers, install detach/cleanup, raw-backup discovery and
restore validation, control-center routing, browser worker event handling,
and logging. Full-workspace tests and host/Android Clippy also ran. This is
not a proof that every possible device-dependent path is defect-free.

Unchanged: device configuration, crypto algorithms/services, super mapping,
image/ZIP/OTA writing algorithms, installer commands, 7zz, AVB implementation,
recovery retention, backup/restore writes, browser engine/runtime and jail.
The installer change only emits a state event around its existing detach.

## Verification

- 1655 passing test invocations; 3 existing ignored invocations.
- 467 UI tests in each workspace/AERA test run, including true touch,
  composition, candidate commit, rendering, draw-ahead and navigation.
- Host and Android all-target Clippy passed. The existing static system-init
  unused -no-pie linker warning remains.
- Independent image verification compared 386 source/resource files with
  the build mirror. Relative to test38, only system/bin/recovery differs.
- 72 layout previews cover both densities, themes and all three languages.
- Only recovery_a was flashed, after checking miro, serial, slot A,
  bootloader mode, unlocked state and 100 MiB partition capacity.
- Entire recovery_a readback and /system/bin/recovery plus /proc/399/exe
  matched the expected image/ELF hashes.
- Owner data decrypted, WiFi connected and network clock synchronized.
- A live Chinese beifen -> candidate -> Backup search was screenshot-checked.
  A further 12 scripted rounds of search/candidate/result navigation completed
  with PID 399 unchanged and no panic.log. The final home screenshot and raw
  touch transcript are retained privately. This is a bounded smoke test,
  not proof the reported intermittent issue can no longer occur.
- Restore discovered both existing standalone boot_a backups. No restore,
  ROM reflash, format, AVB write or slot change was used as a test.
- The post-ROM detach notice was tested offline, not by reinstalling a ROM.

## Artifacts And Delivery

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test39b-aera.img
Bytes: 104857600; compressed ramdisk: 97578482.
IMG SHA256: b83359e67bf5aa59fb7f0a39bbd131a2833c65761e68aad044c531b807d06b77
ELF SHA256: 1367ca1f9795b2df950d4d99070d4e6403f6437675be893ada76bc5b5b55ce9d
Runtime SHA256: 894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5
Flash evidence: D:/k80ptwrp/out/test39b-flash-20261119-185421

Test38 is retained as evidence, not overwritten. No GitHub push, release
upload, online-update promotion, Android reboot or PC power operation is part
of the original checkpoint. Browser runtime distribution restrictions in
crates/skyward-browser/NOTICE.md still apply.

## GitHub Submission Authorized

The user subsequently explicitly requested pushing GitHub. This authorizes
source submission to experiment/miro-upstream031-native and staging the
matching IMG/SHA256 in an unpublished development Release draft, as for the
previous browser-enabled builds. It does not establish the original crash's
root cause or authorize a public binary release or online-update promotion.
No phone operation or PC power action is included in this delivery.

The verified IMG also exists on the phone as
/sdcard/Download/skyward-v1.0.0-miro-test39b-aera.img, with the expected SHA256.
Source, image, history and private validation evidence are retained under
E:/K80Pro-Skyward-Backups/miro-test39b-stability-checkpoint. Raw device logs,
screenshots, account details and credentials are not submitted to GitHub.
