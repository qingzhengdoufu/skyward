# Test30 Cloud Connection Checkpoint

2026-11-19. Private development image, based on test29 (59a4689).

## Device Evidence

Test29 repro logs: D:/k80ptwrp/out/test29-cloud-repro-20261118-234232.
They repeatedly report missing fakevideosink and pthread creation EAGAIN,
including a fatal GLib error creating a webkit-webrtcbin ICE thread. The
recovery PID stays 428 and the WLAN default route remains present.

Test30 adds matched libgstdebugutilsbad.so, which provides fakevideosink and
fakeaudiosink. All 474 previous runtime members are byte-identical. No engine,
GPU driver, speaker routing or worker behavior changed.

The launcher now distinguishes the existing 96 process bound from a 256 task
bound. Linux RLIMIT_NPROC includes threads; the old limit allowed only 95
additional threads in a dedicated-UID reproduction and then returned EAGAIN.
The new limit allowed all 128 test threads. The supervisor also counts tasks;
the 1536 MiB aggregate resident-memory bound, capabilities, UID, seccomp,
device access and mounts are unchanged. This is bounded headroom, not a fix
for potential peer/thread leaks in a site or engine.

Remove the saved-network management list, its forget action, view model and
worker report channel. Keep nearby scanning, manual connection, remembering
passwords/automatic reconnection and forgetting the network from its details.
No stored passwords were erased by this change.

## Verification

1600 passing Rust test invocations (includes AERA UI repetition), 3 existing
ignored. Linux and Android strict Clippy passed. C++ resource bounds and Linux
RLIMIT_NPROC reproduction tests passed. ARM64 minimal-runtime loader checks,
required fake sink factories, H.264/Opus SDP, local DataChannel ping/pong and
video pause/seek/resume/EOS passed. Complete 475-member extraction with Chinese
font injection and independent image/ELF/stock/AVB digest checks passed.

Flashed only recovery_a, retained slot A, rebooted recovery. Full partition,
running ELF, runtime and launcher SHA256 match. No formatting, system/AVB/slot
modification. Data owner 0 decrypted; WiFi connected and NTP synchronized.
Flash logs: D:/k80ptwrp/out/test30-flash-20261119-000335.

Test30 repro: D:/k80ptwrp/out/test30-cloud-repro-20261119-000445.
The user still sees the game's connection-interrupted/check-network dialog.
The observed session no longer logs missing fakevideosink, thread EAGAIN or
WebProcess termination. Task limit 256 and inherited seccomp are confirmed;
the game WebProcess reaches 81 threads in one snapshot. The recovery PID is
454 and WiFi/default route remain present. These observations do not establish
ICE/DTLS success or reception/decoding of a remote stream. More targeted RTC
telemetry is needed. Do not describe cloud games as working.

## Identity

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test30-aera.img
- Image bytes: 104857600; ramdisk bytes: 97561938.
- Image SHA256: 432c79f29ec824a129867298e2d5ce876d588330dbae2e755646a4c8b35a17e4
- ELF SHA256: 5ce40853f8df813792e4794697af76d765690c37aca76173bbf0e90f98b0b922
- Runtime SHA256: b6ce328a4eaa43c353da9c6a4518dad4dc8c494699eaab6b06a7d7dc9905d4e6
- Expanded runtime SHA256: c806092c39c0c170c9eabbe12deabb256bfcb0354648f947381b9c65441aa64d
- Launcher SHA256: 881a1d81f43edb5d8013fa194788ffb02f2eb18eb71bb5894541a210ae80accb

Private-only distribution restrictions and unresolved test29 limitations apply.
