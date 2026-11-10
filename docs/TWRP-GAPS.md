# TWRP comparison and startup animation worktree

Scope: static audit of the locally supplied K80 Pro TWRP ramdisk (SHA-256
437cc1bf7ed8e9bf6812a80d06d6bf99be5790cbfaae5d90cf3ed06d7c3a7f48),
its twres/portrait.xml and current Skyward miro source. A TWRP theme entry can
be gated by build/runtime flags: it is evidence of an interface, not proof the
particular binary successfully implements every feature. No phone testing or
partition writes were performed for this comparison.

## First Repair Batch (miro.15)

Startup intro now starts at the first UI loop frame (not before worker setup).
The r15 device log records 48 frames from 10.084s to 10.773s; WLAN scan and
password decryption subsequently succeeded. Frame pacing is log evidence, not
a substitute for subjective visual inspection.

EFS is shown unavailable and cannot be selected without a configured verified
partition list. Backend refuses empty EFS selection. No EFS partitions are read
or guessed. Backup directories now use exclusive creation and only newly-owned
directories can be removed on failure. Backup manifests validate names, file
paths, duplicates and digests before restore. simg2img accepts stdin using a
bounded 256 MiB temporary spool, validated before opening the target. Device
tests used synthetic /tmp files: file input, stdin, unzip and 7za output hashes
matched; malformed input left an existing target intact.

The original comparison below is retained as the pre-repair audit. Resolved
items are those explicitly listed above; remaining gaps are not implicitly fixed.

## Startup Implementation

skyward-ui/src/intro.rs implements the approved 680 ms transition: 280 ms
wordmark fade-in, 120 ms hold, 110 ms fade-to-black, 170 ms page fade-in. Mark
and page never overlap. The renderer uses the project's existing font, sprite
and canvas and makes no per-frame allocation. No video player or decoder.

skyward/src/ui.rs enables it only on miro's ordinary Menu startup, without a
frame-dump request. The timer starts at the first UI loop frame. It does not
hide kernel/service startup before DRM opens or speed boot. It makes
the ordinary UI event loop redraw while active, with no separate sleep loop.
All inputs remain serviced; first input dismisses the intro without activating
an invisible control. Non-menu install/wipe/rescue/fastboot requests bypass it.
No display power, touch gesture, AOD, data, AVB or partition code is changed.
Next hardware test still required for smoothness and startup overhead.

## Material gaps

| Area | Skyward miro status |
| --- | --- |
| ZIP install queue | One package per install. No TWRP-style queue/reorder/remove of multiple ZIPs (supplied theme offers up to 10). |
| Script compatibility | Executes package-provided update-binary using API 3. No built-in general Edify/updater-script interpreter; a package's own updater may implement it. No guarantee for scripts using TWRP CLI/ORS or private paths. |
| Script tools | 7za is ZIP extraction subset only, no .7z/RAR. zstd focuses on decompression. resetprop cannot override already-set ro properties. simg2img accepts one file, not stdin/multi-input resparse. |
| Image installation | Raw/sparse boot-family, filesystem and super paths exist, but actual ROM/super/AVB writes remain untested. No parallel read/write pipeline, logical resize/group editing, or GUI for releasing all super mappings. Live super holders cause refusal. |
| Backup granularity | Existing grouped boot/system/data/EFS backup plus standalone boot-family/super .img export. No per-logical-partition selection in image-export page, no backup password encryption, no TWRP .win/manifest compatibility. |
| EFS on miro | Category exists, but devices/miro.toml has no backup.efs list. Selecting it currently plans no EFS partitions. Must identify/test exact device partitions before claiming coverage. Do not invent a list from another device. |
| Data restore | File-based with internal storage, lock credentials and secure service data excluded; data-key reset protection. Not a complete device clone and not guaranteed to migrate after Format Data. Actual restore regression coverage is still missing. |
| Filesystem tools | ext4/f2fs repair and filesystem recreation exist. No general GUI filesystem resize (distinct from resizing logical partitions), no TWRP SD-card repartition workflow. |
| Ramdisk/kernel repacking | Host packer exists. No on-device GUI to inject recovery ramdisk/kernel into an existing boot image as the supplied TWRP repack pages offer. |
| Automation/desktop backup | adb shell, sideload, fastbootd and BCB/rescue support exist. No TWRP OpenRecoveryScript compatibility or TWRP adb backup/restore stream protocol. |
| Backup management | Dedicated backup format, restore/delete flow and integrity checks exist. No full TWRP-like encrypted-backup/custom-name/rename/options parity. |
| UI/portability | Three languages, light/dark UI. No TWRP XML-theme import or broad language set; this is a miro adaptation, not a generic device tree. |
| Hardware | Password decryption, compatible dark/wake touch, USB charging and WLAN scans observed. True panel-off touch resume and wall-charger fast-charge not fixed/verified. Do not assume TWRP succeeds at them merely from its theme. |

## Already present, not missing wholesale

- Basic file operations, permissions, text editor, terminal, log export and
  SELinux-context relabel tools.
- Password-based data opening on the tested system; not a proven full NXP
  Weaver transport implementation.
- ZIP/script installation entry, raw/sparse handling, adb sideload, MTP and
  fastbootd. Each real ROM must still be inspected and tested.
- AVB graphical tool and optional recovery-owned post-install disable path;
  actual writes are not validated merely by host tests.
- Wi-Fi and a pinned-signature self-update implementation, with manual write
  confirmation. Phone network/download/write end-to-end testing is incomplete.
- Backup, restore, wipes, mounts, slot controls and optional second-system code.
  Existence in source does not make untested destructive workflows safe.

## Priority recommendation (no automatic implementation)

First validate read-only image export and a known ROM's updater dependencies;
then fix misleading EFS coverage and test backup restoration using expendable
data. Next consider complete tools/ZIP queue and carefully scoped super-holder
handling. Logical metadata editing and boot repacking should be later features
because their blast radius is larger. Encrypted backup and TWRP backup import
are separate compatibility projects, not filename changes.
