# Miro test16 progress, 2026-11-16

## Scope

Based on test15 commit 935977165c31fa70feaefa2fa624b4a58372b8ce.
No changes to 7za/7zz, zstd, super writing, decryption, touch wake,
slot selection or post-install AVB behavior.

- Decode HTML entities without slicing through a UTF-8 code point. The old
  34-byte string slice panicked on Chinese search responses; release aborts
  then caused init to restart recovery. A synthetic response reproduced the
  old panic, and the fixed code also parsed a captured Bing response into
  10 results / 20 blocks. There was no evidence of a kernel reboot or OOM.
- Bottom navigation height: 44 to 36 dp; glyphs: 8 to 6 dp. Touch-down gives
  one haptic pulse when the vibration preference is enabled.
- Remove only the Control Center header settings icon. Keep grid settings.
- Before running a confirmed script ZIP containing an explicit modem.img,
  NON-HLOS.bin, or a .zst version, release the selected A/B modem mount
  aliases with normal unmount and clear BLKRO as needed. Check all target
  filesystem holders before changing mounts. Refuse busy devices without
  killing processes, resetting DSPs or using forced/lazy unmount.
- Restore the previous block read-only limits when the script ends. Leave
  old firmware mounts detached after writing, since their cached metadata
  must not be reused. Reboot recovery to reload firmware afterwards.

## Boundaries

The ZIP directory provides a narrowly scoped preparation hint, not proof
that the script will write a modem or that the firmware is compatible.
Renamed images, nested archives and firmware downloaded by scripts are not
detected. The original package script still selects partitions and writes
bytes. There is no new standalone modem image flashing interface.

If normal unmount fails, the script does not start. Earlier aliases may
already be detached; reboot recovery before retrying. No automatic firmware
consumer termination or mount reconstruction is attempted on this path.

## Validation and phone state

Full workspace tests, strict host/Android Clippy, Android release build and
offline image inspection are required for this artifact. The captured-search
fixture regression is separately run because it needs a local response file.

A reversible shell probe on the connected miro test15 verified all three
modem_a mount aliases can be normally unmounted and BLKRO cleared. It wrote
zero image bytes. The initial restoration missed propagation flags due to a
toybox mount syntax mismatch; all aliases were subsequently restored to
shared:16, BLKRO=1, and the same full-partition SHA256. See the separate probe
and restoration records in the workspace out directory.

Test16 is not flashed or device-validated. The phone remains on test15.
No claim is made that an actual modem firmware installation has passed.
