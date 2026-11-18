# Test32b Initial Device Verification

2026-11-19 02:01 Asia/Shanghai. User authorized flashing test32b.
Device: miro / 7129edbc. Before flash: test31, slot A, battery 39%.
No installer operation was running. The old recovery startup log was saved.

Bootloader checks confirmed miro, current-slot a, unlocked yes,
is-userspace no and recovery_a size 0x6400000. Only recovery_a was flashed;
then fastboot reboot recovery returned OKAY. No system, data, AVB or slot
operation was requested or performed.

ADB returned in recovery. Full 100 MiB recovery_a SHA256 matches the candidate:
554914bbc6b2d5df4c37ac7939f2779e0b789bce0f0c4ed6c5c99bbdf7cb3e95.
Both /system/bin/recovery and /proc/409/exe match the built ELF:
cdbef2f3e4fcc4d265dc39d5c2ef84058f0dd53c38379edf8e7991c71259cbf6.
The runtime and jail match test31. Live slot remains _a.

Startup evidence: user 0 decrypts, leaving only secondary user 999 closed;
three saved networks load, wlan0 associates and gets DHCP at about 22.2 seconds;
internet reachability and network time sync succeed. Crash buffer is empty.
This verifies one boot's credential reuse, not changed-password/forget failure
or repeated radio cycles. Fullscreen gestures and UI feel still await user
acceptance. Cloud-game testing was not performed and remains unresolved.

Evidence: D:/k80ptwrp/out/test32b-flash-20261119-020111.
Do not interpret generic stock-init/optional firmware warnings as a clean
kernel log; this checkpoint makes no claim that every driver warning is fixed.
