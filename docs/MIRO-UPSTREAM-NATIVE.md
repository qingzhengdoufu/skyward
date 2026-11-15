# Miro native upstream integration

Checked 2026-11-15. This supersedes MIRO-UPSTREAM031-MERGE.md for the
experiment/miro-upstream031-native branch only. Previous images and source
branches remain unchanged. Device version remains 1.0.0.

## Upstream coverage

The target is kirklin/skyward v0.3.1 (bd34c05), plus main through
c5e09f6eb644e75e051b309f98d6745ccbc97d81. It is not the haotian fork.

- Adopt upstream's split App modules and Pages runtime, with one input and
  effect dispatcher. Local controls are integrated into those modules.
- Adopt the common Worker/Teller model for decrypt, backup, updates, WLAN
  and readings too. The earlier installer, file, wipe, merge, partition and
  system workers continue to use upstream infrastructure.
- Enable stock init/manifest key-service discovery on miro. Run one discovered
  chain, with in-process OMAPI between secure-element registration and Weaver.
  Remove configured_keys, the hand-coded key chain and standalone OMAPI packing.
- Include post-release cc68936 library search/bundle ordering and c5e09f6
  node-readiness waiting. Restore the previously omitted upstream device files,
  including dada's revised secure-element and Peach v2 configuration.
- Adopt upstream haptics sample/waveform lifetime implementation, with a small
  explicit miro stored-waveform mode. No unused constant-effect option remains.
- Retain upstream Binder serving, program parsing, metadata encryption helpers,
  dual-system boot-set fingerprint protection, MTP and packer descriptions.
- Build with Rust 1.100.0, Edition 2024, resolver 3 and Android NDK r29.
  The previous Rust 1.99.1 toolchain and successful baseline are preserved.

## Intentional differences, not omitted upstream work

This is not a byte-identical upstream build. Feature preservation requires
these local patches; host tests cannot establish phone behavior.

1. Miro peripherals: black-frame keep_panel_on and software double tap, tested
   Qualcomm waveform 0 with 0.1..0.4 strength, Peach v2 firmware/readiness,
   remembered WLAN networks and reconnect. No hardware-wake experiment.
2. Charging safety: start only an offline ADSP, never stop/restart a running
   ADSP or request SSR. The packer removes stock SSR writes as before.
3. Crypto: sibling spblob/secdiscardable selection, locksettings/users reading,
   typed credential failures and bounded Weaver calls remain. Explicit NFC
   service description and service socket compatibility remain supported;
   they do not start a second key chain. Restart backoff stays five seconds.
4. Install: local raw-super recognition, 4 MiB write/verify path, flushing,
   local/FUSE handling, script paths, stall reports, AVB controls and the
   post-install option remain. Builtin sevenzip/unzip/zstd are not replaced.
   Upstream Worker and ensure_installed() are active, so the entire install
   path is NOT identical to the previously phone-tested minimal build.
5. Backup: current-slot independent images only; shared super remains and is
   explicitly labelled. Unconfigured EFS is hidden rather than guessed.
6. UI: intro, adjusted corners/themes, USB management/notifications, search,
   image export, password messages and remembered credentials remain.
   Backlight/haptic failures are nonfatal; socket requests omit payloads in
   logs. Conflicting partition/decrypt operations are refused visibly.
7. Updates: qingzhengdoufu/skyward signed-release trust and mirror handling stay.
   Upstream automatic multi-device publishing is not enabled. CI is manual or
   repository-gated; this work does not publish or tag a release.
8. Plugin list remains a manifest viewer. Original upstream has no executable
   plugin system. Haotian experimental plugin execution is not imported.
9. Upstream device configs are available, but haotian/miro retain local device
   adaptations. This is a miro image, not a tested multi-device release.

## 7za versus 7zz

Neither v0.3.1 nor main at the above commit adds an external 7zz. Both dispatch
the builtin 7za implementation. This image keeps that path and does not add
7zz/7z/simg2img executables. Compatibility is limited to the builtin supported
commands/formats; it must not be advertised as the complete upstream 7-Zip CLI.

## AERA browser attempt

Reference: AERA-Recovery/android_bootable_recovery, aera-16.0,
e438f6de62090a79e81daf00b449b48da7377753.

The new skyward-browser crate implements the 2072-byte protocol, fixed-size
sealed shared-frame transport, frame sequencing/acknowledgement and bounded
runtime verification. The pinned real runtime passed both compressed and
expanded SHA-256, length and all 445 archive-member checks. Alias targets must
be safe regular members; duplicate paths, cycles and parent-file collisions
are rejected. No archive extraction or payload execution was performed.

An Android ARM64 AERA jail/minijail binary was built locally for investigation,
not executed, approved or packed. It still needs a Skyward-specific sandbox
review. The advanced-page prototype accepts addresses and checks the runtime,
but the launcher deliberately refuses to start. IT CANNOT DISPLAY WEB PAGES.

Still required: safe extraction, adapted launcher and resource policy, frame
rendering into Skyward, touch/keyboard forwarding, process teardown, download
scope, device seccomp/namespace/GPU/DNS tests and upstream third-party
source/license/security-update distribution requirements. The 39 MiB compressed
runtime, jail and their libraries are not bundled or published. The recovery
does not start a browser during boot or allow WebKit to run as root.

## Verification

- Full workspace tests passed, including 403 UI tests and eight browser tests.
- Host and Android workspace/all-targets Clippy with -D warnings passed.
- Android release, static system-init and host packer builds passed. The static
  helper has the existing nonfatal unused -no-pie linker warning.
- Independent image unpack verified 100 MiB size, AVB hash descriptor, stock
  inventory, embedded binary identity, miro config and builtin command links.
  This remains an unsigned image for an unlocked bootloader.
- Chinese/English light/dark targeted page previews were generated and reviewed.
- No phone access, flashing, reboot, slot/AVB changes or data operations.

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test3.img
SHA256: 8d5ee2c737cd08bd171734454ee5a9392cf3cbe32fc1955bd8e9acb7a0a23a86

Boot, decryption retries, WiFi, haptics, touch after dark, USB/MTP and a real
ZIP/super installation require device validation before stable publication.
