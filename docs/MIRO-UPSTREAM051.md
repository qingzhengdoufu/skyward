# Miro Upstream 0.5.1 Integration

2026-11-24. Private test40 development build, not device accepted.

## Source Lineage

- Preserved device baseline: eaa5f7fb736408826cd4908234d68e6af085f01d, test39b.
- Original upstream: kirklin/skyward, main at
  43ec6da19402cf344b966bbb8c85c04fd2ffdcad (source version 0.5.1).
- Integration branch: integration/miro-upstream051. Upstream functional changes
  are integrated as individual commits with local compatibility retained.
- The fork package version remains 1.0.0. Upstream 0.5.0/0.5.1 release-only
  version edits do not lower the fork's version or promote its online updater.
- Rust 1.100.0, NDK r29 and cargo-ndk 4.1.2 were already configured; this
  integration adds the upstream unused-dependency checks to local packages.

## Integrated Upstream Changes

- Payload progress counts each payload block once, starting before the OTA
  engine starts. The local script activity counter remains independent.
- Unencrypted data opens without a key service; its reset path follows the
  installed filesystem table. No reset or formatting was used for validation.
- Fifteen-language catalog, locale selection, plural/case rules, wrapping,
  and preview text-fit reporting. Local-only features retain simplified
  Chinese, traditional Chinese and English, with English fallback elsewhere.
  The local Chinese IME and custom page layout are retained.
- Follow the installed system's timezone, with 62 selectable zones grouped
  by region. Local control-center dates, image backups and file search use
  the same timezone rules.
- Native raw/sparse super, erofs/ext4/f2fs logical images, filename matching,
  mount/read-only handling, logical resize and full written-range readback.
  Pending snapshot protections remain. These replace only the direct IMG path.
- Backup format 2, format 1 reading, separate internal-storage selection,
  and restore into already re-created data with its existing keys preserved.
  Factory-reset/fresh-key restoration is not tested on the user's phone.
- K60 Pro configuration, optional firmware, sample-scaled vibration support.
  Miro still uses its tested waveform and strength range, not sample scaling.
- NXP secure-element driver discovery and capability-bearing service loader
  dependencies. Miro retains configured_keys=true and its explicit tested
  service chain, module and device-node permission setup.
- Rust 1.100 refactors and unused dependency linting.

## Local Behavior Preserved

- ZIP install script execution, its existing extraction tools and arguments,
  modem preflight, warnings, bounded process/pipe cleanup and original package
  name. No full 7zz or ZIP queue is introduced by this integration.
- Official OTA still uses update_engine with local lifecycle handling.
  Recovery retention and optional post-install AVB handling remain.
- Miro stock303 configuration, black-frame touch compatibility, software
  double-tap, Peach WiFi and saved passwords, Qualcomm waveform 0.
- Weaver/secdis fallback, locksettings WAL handling, credential error types,
  safe ADSP bootstrap and prohibition on SSR/shutdown writes.
- AERA runtime, GPU/browser prerequisites, profile/cache placement, Chinese
  input, navigation, control center and full-screen exit behavior.
- Current-slot standalone raw image export/restore, shared-super labeling,
  SHA-256 verification and exclusive backup-directory creation.
- Post-install detached-system notice and test39b stale-input guards.

## Layout

Existing page groups and navigation remain. Internal storage is one additional
backup row, not a home shortcut. Timezone regions are inside Settings. No new
duplicate image-flash entry or browser mode is added.

## Limits And Acceptance

ZIP scripts can continue extracting zstd-compressed images with 7za/unzip and
zstd. Native direct-image support is raw or Android sparse, not an independent
zstd decompressor. Cloud-Genshin ICE and audio remain unresolved; the browser
runtime has not been updated or claimed fixed here.

Host tests, AERA UI tests, host/Android Clippy, native helper tests, packing and
independent image/source comparison are recorded in the local test40 logs.
Passing offline checks is not proof of decryption, WiFi, touch, ROM flashing,
logical resize or restoration on a real phone. Device acceptance is pending.

The test39b image and source branch remain available. No phone operation,
GitHub push, public binary distribution or PC power operation is authorized
as part of this offline integration. Browser runtime distribution restrictions
in crates/skyward-browser/NOTICE.md still apply.

## Offline Results

- 1722 passing test invocations, 3 existing ignored invocations.
- 485 UI tests in both workspace and AERA-feature runs.
- Host and Android all-target Clippy passed with warnings denied.
- Native frame/resource/task tests and complete pinned runtime extraction passed.
- 240 preview images cover ten pages, two resolutions, both themes and the
  three local languages. Chinese contact sheets were visually inspected.
- 414 source/config/resource files match the Linux build mirror byte for byte.
- The image is 104857600 bytes. Its AVB hash descriptor verifies; it remains
  an unsigned unlocked-bootloader development image, as the baseline was.
- Android ELF is AArch64 PIE with dynamic Bionic and no external C++ runtime.
- Compared with test39b, only system/bin/recovery and skyward/system-init
  change in the ramdisk. The latter includes upstream's never-type refactor.
- Device configuration, browser runtime/jail, fonts, extraction commands,
  OTA lifecycle helpers and ZIP script execution were checked against baseline.
- The existing static system-init linker warning about unused -no-pie remains.

Image: skyward-v1.0.0-miro-upstream051-stock303-test40-aera.img
IMG SHA256: c670e72b1522861bf5b3aeacdc33d444764e5114212fc3bf482af35a2cd107cd
ELF SHA256: 6eef3d88dc3bedc6e6f6c8d818addfea6db5bc26b2c5d766448b08e725415f9a

Validation reports and full build output are under D:/k80ptwrp/out/test40-*
and D:/k80ptwrp/out/build-test40-native.log. No phone has been touched in this
integration. User acceptance should first check boot, decryption, WiFi,
vibration, Chinese input, page navigation and backup discovery without writes.
