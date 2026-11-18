# Test35 Official Full OTA Lifecycle

2026-11-19, private offline build based on test34 / e46967f.

## Source Comparison

Compared with Rinrroy/skyward-for-haotian main at
7b893d5e8b6decae17ad670c0088a0b3bb79202a (fetched this session).
Both recoveries identify full A/B ZIPs by payload.bin and hand the stored
payload offset and properties to the stock update_engine_sideload. This is
not a script/7zip/super.img install. Package identification and preflight
checks were already identical. Incremental packages and older timestamps
remain refused; no verification or downgrade bypass was added.

Ported the fork's ota.rs progress parsing and engine.rs child/pipe lifecycle
(8e72808 / 2b2b27f). Progress now comes from update_engine, not how far a FUSE
reader sought in the ZIP. Snapshot helpers may keep inherited pipes open
after the engine exits; waiting now observes the engine independently and
releases the readers without killing snapshot helpers. Existing worker
abstraction, modem-script handling, super, 7zz, AVB, crypto and UI are kept.

## User Package Inspection

Read-only inspection of:
E:/k80p/miro-ota_full-OS4.0.3.0.XOMCNXM-user-17.0-526d60ce11.zip

- ZIP bytes: 9260042933; 7 entries.
- Stored payload v2: 9260034330 bytes; minor version 0; not incremental.
- pre-device=miro, ota-type=AB, SDK 37 / Android 17.
- Internal version: 17OS4.0.261106.181832371.QCPECN.S (differs from filename).
- post-timestamp=1793965700, security patch 2026-10-01.
- No ota-wipe=yes or POWERWASH=1. This is not a guarantee of data compatibility.
- Full payload SHA256 and metadata SHA256 match payload_properties.txt;
  ZIP payload CRC also verified. This is integrity, not OEM authentication.
- Payload operations: REPLACE=559, REPLACE_BZ=222, REPLACE_XZ=4992.
- Dynamic partition group: 11800674304 bytes; snapshot and VABC enabled,
  compression lz4, COW v3. 48 partitions total 12046012416 image bytes.
- Includes a 104857600-byte recovery image; keeping Skyward matters.

No archive script ran and no ROM/phone partition was written for this check.
The stock Android 16 update_engine's real Android 17 installation, signature
trust, VABC handling and boot result remain untested.

## Verification And Identity

1620 passing test invocations, 3 existing ignored; host/Android all-target
Clippy and native/runtime/font checks pass. Independent verification matches
382 source/resource files with the Linux mirror. Browser/runtime unchanged
from test33/34; only recovery changes in the ramdisk relative to test34.

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test35-aera.img
- Bytes: 104857600; compressed ramdisk: 97561397.
- SHA256: bf7ab146e7c6da675e581620388017ae2f7f70b09896792118c36dfbb9e6d017
- ELF SHA256: 96c294a6cf733e360a5ec96d3f1882798a7aa0d5aa84103ed60aaf669876c72f
- Runtime SHA256: 894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5

Not flashed: the user disconnected the phone. Last verified phone image is
test34, which passes actual WiFi long-hold navigation. Browser work is paused
at test33; cloud-game ICE remains unresolved. No public distribution.

Next scoped work: match haotian's recovery-preservation compatibility when
the other slot contains a ramdisk this recovery cannot parse. KeepRecovery
is already enabled by default, holds before installing, restores next slot
and overwritten non-Skyward slots, and reads written recovery images back.
