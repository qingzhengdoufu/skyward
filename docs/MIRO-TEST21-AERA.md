# Miro Test21 AERA Experiment

Private device-test build, 2026-11-17. Based on test20 (ccc0b220).

## Build

- Feature: skyward/aera-browser; normal builds retain Chromium.
- Shared UI: bottom browser address and plain tool icons, grouped partition
  rows, cached Control Center backdrop blur.
- AERA ARP2 uses sealed double-buffered pixels rather than PNG/CDP capture.
- Pinned runtime: e438f6de62090a79e81daf00b449b48da7377753, 445 members.
- Runtime SHA256: 4e3941bd2bcbcc53e4d1bdc7641e7d7c7b16b848b8ba21bb934119118b37f197.
- Jail SHA256: 12ec8bee8d9165becba466bcff657ace99646624a7cc35a5c2aea54f7a36c7c7.
- Dedicated profile/cache/downloads: /data/skyward-aera, not Chromium's tree.
- KGSL is loaded on browser demand, not on recovery startup. No display DRM
  acquisition, frequency override, ADSP reset or blanket device chmod.
- Existing installer, 7za, decryption and touch paths are preserved.

## Verification And Flash

- Workspace tests passed with 3 existing ignored tests; 425 UI tests also
  passed with the AERA feature enabled.
- Host workspace and Android AERA strict Clippy passed.
- Android release built and pinned runtime fully verified.
- Independent image checks passed: partition size, AVB digest, stock-entry
  preservation, configuration, tools, OMAPI and exact recovery binary.
- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test21-aera.img.
- Image SHA256: 4357990a1b926c3d53cbd5c611d799b06c327b43507b7c548b6d7045ccbd8ebc.
- Serial 7129edbc, product miro, unlocked, current slot A.
- Only recovery_a flashed; fastboot returned OKAY, recovery rebooted.
- Readback matched image; recovery_b stayed at
  437cc1bf7ed8e9bf6812a80d06d6bf99be5790cbfaae5d90cf3ed06d7c3a7f48.
- Previous recovery_a was test20 and its full backup matched
  cfe2314d20797cc6a5b3fd52171eb846fc72a226b13d8906a0c85d79fe73cc12.
- First boot: recovery PID 422, user 0 storage opened, WLAN scan found 32 APs,
  crash buffer empty. No ROM flash, slot switch, AVB change or format.

## Still Experimental

- No device proof of AERA page rendering or Adreno 830 acceleration yet.
- Bundled Mesa 26.2.2 Turnip contains an Adreno 830 entry; this is not a
  successful device enumeration or render test.
- Fixed 360x700 CSS viewport is letterboxed; keyboard changes display extent.
- Raw pixels still require CPU conversion/copy; this is not zero-copy scanout.
- AERA downloads stay in its private tree and are not yet wired into the main
  download task list. Audio output is not implemented by this experiment.
- Runtime redistribution requires its complete third-party license/source
  bundle and update procedure. Do not publish as a production release.
