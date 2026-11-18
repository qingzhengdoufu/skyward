# Test36 Preserve Recovery Across Official OTA

2026-11-19, private offline build based on test35 / a21000d.

## Haotian Parity

The OTA route is shared with Rinrroy/skyward-for-haotian at 7b893d5:
stored payload.bin -> stock update_engine_sideload -> partition verification
and the other A/B slot. Test35 ports the engine's real progress and bounded
pipe lifecycle. Test36 ports its recovery-preservation compatibility from
5789696 and 20b450b, including the matching online-recovery-upgrade handling.
No alternate installer, 7zip path or snapshot rewrite was introduced.

KeepRecovery already defaults on and is exposed as an install option. Before
writing, Kept holds the current slot's Skyward recovery image and SHA256 of
both full recovery partitions. After a successful full OTA, next is the
other slot, as in haotian. Script packages query BootControl::next_slot after
the script, not the unchanged ro.boot.slot_suffix. Both paths use Kept::wanted:
the next slot receives this exact build unless already identical; another
slot is restored only if the install overwrote it with non-Skyward content.
Writing uses image::flash with readback verification for these small images.
This code does not set the active slot itself; update_engine/package does.

Some original recoveries carry ramdisks this parser cannot decode. Those are
now logged and treated as non-Skyward for preservation/upgrade selection,
instead of aborting the whole install. Open/read failures of the partition
still propagate. Two added tests cover both OTA directions with unfamiliar
stock ramdisks and a missing partition read error; haotian's regression
tests are retained.

Reflash failure remains a distinct visible result and is not claimed as
successful preservation. Like haotian, successful ROM installation itself is
not relabeled failed merely because preserving recovery failed. A user who
turns KeepRecovery off explicitly disables preservation.

## Real Package Offline Acceptance

The Rust package::identify/check implementation accepts the user's 9.26 GB
ZIP64 full OTA for miro, with offset 4931 and payload bytes 9260034330.
No timestamp from an installed phone was supplied to this host-only check,
so downgrade checks against the actual installed build are still required
on device. The package was not executed. Full payload/metadata SHA256 and
payload CRC pass; see MIRO-TEST35.md and official-ota-OS4.0.3.0-inspection.json.

Only the 100 MiB recovery was extracted from the payload, with manifest
hash verification: 2e490b0ceff2cb0a1e17333c58eb01d1220ec787db761a2a84296266842aefd3.
An external host-only Rust harness uses the same Kept/image APIs and the
actual test36/official recovery images in temporary regular files. A->B and
B->A both restore exactly this build and pass reread/equality checks.
Evidence: out/official-ota-real-preservation-test36.log. No phone, block-device
or BootControl operation occurs in that harness.

## Build And Preservation

1624 passing test invocations, 3 existing ignored. Host/Android all-target
Clippy, native frame/resource/task tests and full runtime/font extraction
pass. Independent verification matches 382 compiled source/resource files.
Relative to test35 only system/bin/recovery changes in the image. The browser
runtime, jail, GPU worker, device config, crypto, super, 7zz and AVB handling
are unchanged. Browser work is paused at test33 / b01760b; cloud-game ICE is
unresolved and must not be described as fixed.

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test36-aera.img
- Bytes: 104857600; compressed ramdisk: 97561916.
- Image SHA256: 2aa50f329ec3fa2607c1e2c15607b58af791ff890dec2bffc778e244de6d6b99
- ELF SHA256: 6eacbfe59b486a6691cd40bb1fd0f6c45211ffca53ac517b87e2ceae3cfecff6
- Runtime SHA256: 894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5

The phone was disconnected by the user. Test36 is NOT flashed or device
accepted, and no official ROM has been installed. Last verified phone image
is test34 (WiFi long-hold navigation fixed). The original planned Android
reboot and PC shutdown sequence was not performed while disconnected.

Actual Android 17 OTA signature acceptance, update_engine/VABC compatibility,
postinstall, target boot/vendor kernel compatibility with retained Skyward
and Android boot remain device-test gaps. Do not format data to test them.

Source/history/images/runtime/logs/build recipes and current engine source/
build snapshot are backed up privately under:
E:/K80Pro-Skyward-Backups/miro-test36-official-ota-native.
The earlier full dependency/sysroot snapshot remains in miro-test29-cloud-native.
No public release/push is authorized for this browser-enabled build.

## Device Acceptance After Reconnection

The user reconnected the phone and explicitly authorized flashing test36,
booting Android and then shutting down the PC. They subsequently requested
both test36 IMG and the official ZIP copied to phone storage. This section
supersedes the earlier disconnected/offline-only status above.

Only recovery_a was flashed after product miro, slot A, unlocked bootloader,
bootloader-mode and partition-size checks. Full recovery_a SHA256, on-disk
ELF and running /proc/389/exe match the independently verified test36 image.
Runtime and jail digests match too. No slot setting or vbmeta was changed.
WiFi auto-connects, obtains DHCP, reaches the internet and synchronizes time.
An actual 800 ms control-center WiFi hold opens WLAN and scans successfully;
PID stays 389 and the final crash buffer is empty. No decryption password was
entered on this boot; no new decryption acceptance claim is made.

Both phone Download files were SHA256-verified before reboot:
- skyward-v1.0.0-miro-test36-aera.img:
  2aa50f329ec3fa2607c1e2c15607b58af791ff890dec2bffc778e244de6d6b99
- miro-ota_full-OS4.0.3.0.XOMCNXM-user-17.0-526d60ce11.zip:
  451ed9001b4ec5bcb801173068faa44c48b316e0a1b0b1fb29ee286e0fa1bb44

ADB reboot returned to Android, sys.boot_completed=1, product miro, slot _a.
The official ZIP has NOT been installed; actual OTA compatibility remains
untested. No data format, system flash or AVB operation occurred. Browser
work remains paused/unresolved. Source tree is committed and private.

Evidence: D:/k80ptwrp/out/test36-flash-20261119-053316.
Updated private source/history and acceptance evidence are saved under the
E backup's device-acceptance folder. PC shutdown follows only after that
backup is verified; this document itself does not claim shutdown occurred.
