# Miro 1.0

Based on d730dc0, the minimal haotian adaptation that successfully installed
the user's ROM and booted Android. The original full-feature branch remains
preserved at 986b9fa; this branch is miro-v1-peripherals.

## Restored

- Stock Peach v2 WLAN configuration and transient interface readiness retries.
- Qualcomm HV waveform 0, with tested 0.1 through 0.4 strength scaling.
- Startup animation, USB manager and insertion/removal notifications.
- Standalone physical-partition image backups with checksum read-back.
- Signed releases from qingzhengdoufu/skyward; download and write require
  separate confirmations. Version 1.x uses ordinary semantic version ordering.
- Mainland stable system-update identification and green download progress.
- Plugin manifest browsing, not execution.
- Both adaptation authors in the About page.

## Deliberately unchanged

The entire skyward-install, skyward-block, skyward-pack and skyward-crypto
trees match d730dc0. The successful built-in 7za implementation, zstd helper,
script execution, partition flashing and AVB actions were not replaced.
The ramdisk command list remains the same five commands. No external 7zz,
7z alias, simg2img or modem preflight was added. No ZIP queue or additional
logical/sparse flashing extension was ported, because these require changing
the installation path that this release deliberately preserves.

The proven black-frame touch compatibility mode and offline-only ADSP start
remain. Hardware gesture wake is not enabled. Settings and remembered
credentials retain the successful baseline's skyward-minimal namespace.
EFS backup stays unavailable on miro until its partition list is verified.

## Verification

Build with audit-tools/build-miro-v1.sh in the parent workspace, using the
same stock 303 recovery, Rust 1.99.1 and NDK r29 as the successful build.
The script runs workspace tests, host/Android clippy and Android release
builds before packing. It does not access or flash a phone.

Hardware Wi-Fi, vibration, USB and ROM installation still need device tests.
Host tests and an intact installer source are not proof of device behavior.

The targeted --miro-v1 preview renders restored pages in Chinese/English,
light/dark themes and intro frames. The generic preview run still stops at
its held-settings-row scenario: its helper injects page coordinates without
scrolling the longer homepage's settings row onto the physical screen. This
hits a crop assertion and is not a completed generic preview pass. No canvas
or rendering change was made to hide the assertion.
