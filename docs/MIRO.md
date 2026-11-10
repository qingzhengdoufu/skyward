# REDMI K80 Pro / miro

## Attribution

- Original project: https://github.com/kirklin/skyward
- Adaptation base: https://github.com/Rinrroy/skyward-for-haotian
- Base commit: d6e7b7aefcba123bf0ef15252e954e45a6de747f (0.2.3).
- Local adaptations and testing: this repository's miro branch; build credits
  retain 澪洛依 and 红烧豆腐. Original authors and GPL-3.0 remain unchanged.
- OMAPI prebuilt provenance: resources/omapi/README.md. This inherited vendor
  binary is not claimed to have been compiled from Rust source here. Stock
  kernel, init, HALs and libraries are supplied locally by the builder.

## Build

Use Linux or WSL, Rust toolchain from rust-toolchain.toml, Android NDK r29,
and cargo-ndk 4.1.2. The build requires the user's own full stock recovery.img
from OS3.0.303.0.WOMCNXM. No stock package is downloaded by this script.

Stock SHA-256:
87bd663d74d8a0473f26e0ceac7ab588f4d3cf6bf2d2a50c2ecaff45ca99ff21

```bash
export ANDROID_NDK_HOME=/path/to/android-ndk-r29
bash build-miro.sh /path/to/stock/recovery.img /path/to/output
```

The script runs selected host tests and builds ARM64 release binaries, then
packs on the stock ramdisk. The 100 MiB image has an unsigned AVB footer and
requires an unlocked bootloader. No flashing is performed by the build script.

test11 SHA-256:
6c2146d203f1b7766bffc40fbed87ef382335370942d318f3fc4625ea73d9c20

No release is created automatically. Review proprietary binary redistribution
rights and all files before publishing a packed recovery image. Source is not
the same as authorization to redistribute every stock binary in the ramdisk.

## Observed on this device

- Android 16 / HyperOS OS3.0.305; stock base is local 303 recovery.
- Password opens CE storage using the companion secdis protector path. This
  does not prove the NXP Weaver transport independently works.
- Touch: panel kept on behind black frames, software double-tap wake. True
  hardware panel-off wake lost touch after resume and remains disabled.
- USB charging after offline-only ADSP startup, never SSR or boot=0.
- Native Qualcomm waveform 0 vibration, user-tested strength mapping.
- WLAN Peach v2 scans; initial netdev registration race has bounded retries.
  test11 startup scan observed, not long-term or all-network validation.
- SoC display: 骁龙8至尊版移动平台.

## Experimental / not yet device-validated

- Raw/sparse image flashing, logical-partition mapping and full super writing.
  No automatic logical resizing. Super refuses live holders. Parallel image
  read/write pipelining is not implemented.
- Recovery-owned automatic AVB disable after successful user-requested install;
  option defaults off. No ROM declaration required. Dedicated vbmeta only,
  unlocked required, pre-change /tmp backup and read-back. Scripts' target is
  inferred from boot-control's next slot, not from tracking arbitrary writes.
- Single raw image backup to Skyward/images: independent boot partitions and
  shared super only; no data, metadata, persist or EFS in this new entry.
- CLI tools: 7za supports ZIP extraction only, not general .7z/RAR; zstd is
  decompression-focused; unzip -p streams; simg2img takes one file input only.
  resetprop uses init and does not replace Magisk's full implementation.
- Arbitrary update-binary scripts run as root and can erase any partition.
  Snapshot guardrails are not a sandbox. Inspect each ROM before using it.
- No fast-charge fix. Version 0.2.4 enables this repository's signed self-update
  channel. Install it manually once from test11, whose old endpoint is unchanged.
  Read releases/v0.2.4.md for bootstrap and online-update limitations.

## Signed self-update from 0.2.4

Only published, non-prerelease GitHub releases in wmin85099-cmd/skyward are
accepted. The release body embeds a skyward-manifest block signed with P-256
ECDSA/SHA-256. The public key is pinned in skyward-net/src/github.rs; the private
key is kept outside this repository and is never embedded in an image.

The signature covers tag, device, asset name, size and SHA-256. API metadata and
asset repository URLs must match. Downloads use HTTPS and SHA-256, are rechecked
before writing, and must carry miro's embedded device configuration. After the
download, a separate slide confirms the exact current recovery slot. Only that
slot is updated. Bootloader unlocked state is read from kernel boot parameters;
low/unknown battery, pending/unknown OTA state, concurrent partition workers or
an image not exactly filling the target recovery partition refuse the write.
No AVB flag changes, slot switching or automatic reboot accompany self-update.

This protects authenticity/integrity, not bug-freedom or vendor verified boot.
GitHub APIs may be rate limited or unreachable on some networks. A download
lost on reboot restarts because temporary recovery storage is volatile. Signature
key loss requires a deliberate manual channel/key migration; do not publish it.

Existing recovery starts root adb and may mount data read-write or persist
settings. Do not interpret these notes as a zero-write guarantee.

## Keeping upstream linked

GitHub identifies this repository as a fork of kirklin/skyward. The miro
branch also retains the Rinrroy adaptation history. Keep main for upstream
tracking; merge reviewed upstream changes into miro and rebuild/regression-test
before releasing. Never force-push over main or assume an upstream image fits.

Historical local notes: miro-build.md and test09/test10/test11 documents. These
contain build-time observations, not a claim of universal device safety.
