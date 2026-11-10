# test09 candidate

Based on the preserved test08 working source. Device configuration, credential
handling, panel-on touch compatibility, ADSP startup, WLAN and haptics unchanged.

Adds filesystem image identification (erofs/ext4/f2fs), Android sparse decoding,
existing logical-partition mapping and shared super selection. No partition
resizing, formatting or automatic slot switch. Direct super writes fail closed
when device-mapper holders remain. Logical/super writes remain experimental and
are not authorized for this phone test. Bounded read/write parallelism is not
implemented in this candidate.

Install scripts keep the Android update-binary protocol. Adds simg2img for one
file input, output-capacity checks for block devices, and non-overwriting /sbin
aliases. simg2img does not take stdin or multiple resparse inputs. 7za remains a
ZIP extraction subset, not a full 7-Zip binary; resetprop still uses init and
cannot override already-set read-only properties. Scripts have root access and
are not sandboxed. A compatible updater can still erase data; inspect each ROM.

The saved automatic AVB option defaults off. After an explicitly requested
successful OTA, image or script install, recovery disables verification and
dm-verity on the selected/reported target slot without a ROM declaration.
Scripts use boot-control's next slot, not write tracking; arbitrary scripts may
write other slots too. Never treat this as proof of every script's targets.
Dedicated vbmeta only, unlocked required, backup under /tmp before mutation and
read-back afterward. Backups in /tmp are lost on reboot. No startup AVB writes.

GitHub self-update currently still uses the existing upstream fork endpoint;
no project-specific release source has been configured. Fast charge is unchanged
and not verified. These pending features are not claimed fixed by this build.

Authorized device test: verify identity/unlocked/slot/partition size, flash only
recovery_a, boot recovery and read logs/status. No install package, no AVB flag
mutation, no erase/format, no other partition flash. Existing recovery startup
mounts/settings behavior is retained; this is not a guarantee of zero automatic
data/metadata writes by recovery itself.
