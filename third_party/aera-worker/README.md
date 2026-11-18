# AERA Worker For Miro

Based on AERA-Recovery/android_bootable_recovery commit
e438f6de62090a79e81daf00b449b48da7377753, aeraui/features/browser.
worker.cpp, protocol.hpp and platform_sandbox.hpp retain Apache-2.0 notices.

Local ARP3 adds an initial bounded native viewport handshake, a 1440x3200
maximum double buffer, 4x mobile CSS scale, and worker termination diagnostics.
The worker is dynamically linked against the original pinned musl/WPE 2.52.6
runtime. Runtime dependencies and sandbox checks are not weakened.

Build helpers are in D:/k80ptwrp/audit-tools/prepare-native-worker.py and
compile-native-worker.sh. A newly packed runtime needs new compressed and
expanded digests in skyward-browser/src/runtime.rs before use.

Test25 keeps screenshot row bounds strict while accepting a last row without
tail padding. Valid CSS-size fallback images with matching aspect ratio are
scaled by Cairo; rejected snapshots are logged but do not stop WebKit.
The worker handles video fullscreen inside the native viewport.
The GstChildProxy-compatible audio bin uses
clocked silent output until hardware soundcard/routing support is validated.
The actual speaker-output draft is saved separately under out/test25-audio-addon-draft.

scaletempo, subenc and soup plugins are built from official GStreamer 1.26.9
sources. Original source notices and LGPL terms remain applicable; keep the
source archives and recipes with the private runtime. No DRM capability or
device video-decoder access is added. Public distribution gates in
crates/skyward-browser/NOTICE.md still apply.

Test27 removes media-volume UI, persistence and commands. Command 17 is retired;
mode commands 18/19 and cancel-touch command 20 retain their explicit numbers.
WPE uses a 1024 MiB process memory budget with earlier conservative/strict
reclamation, document-viewer caching, no page cache, and bounded MSE buffers.
The unchanged jail still limits aggregate resident memory to 1536 MiB.
This is pressure management, not evidence that every playback leak is fixed.
Landscape swaps the native backing dimensions, while recovery rotates pixels
and inversely maps touches; the DRM configuration is unchanged.
