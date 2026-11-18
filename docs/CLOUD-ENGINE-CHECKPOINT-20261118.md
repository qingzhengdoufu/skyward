# Cloud Engine Checkpoint Before GPU Switch

Saved 2026-11-18 Asia/Shanghai at the user's request. Stop all work after saving;
the user needs to shut down the computer to switch the graphics card.

## Accepted Recovery Baseline

- Repository: D:/k80ptwrp/miro-upstream031-native
- Branch: experiment/miro-upstream031-native
- Recovery baseline commit: 4578ba1ad2c98e47377dc3c17c2d06350399f311 (test28).
- Image: D:/k80ptwrp/out/skyward-v1.0.0-miro-upstream031-native-stock303-test28-aera.img
- Image SHA256: 52d3a91fb36f3aeeb54dfe4c5d01bcd4db83d396696214d5f475e90ba56f5b5d
- It was flashed only to recovery_a and verified by exact partition readback.
  Slot A was not changed. Runtime and running ELF hashes matched the artifacts.
- E:/K80Pro-Skyward-Backups/miro-test28-native retains source/history/image.
- The user took the phone away to use it. Do not access, reboot or flash it until
  the user explicitly makes it available again. Last observed state was Android.
- Never format data/metadata, modify AVB/slots/system, reset DSP or write persist
  calibration. Never run a second recovery/DRM client for diagnostics.

## Diagnosis And Evidence

The Cloud Genshin screenshot says this browser cannot run the game. Its public
SDK's CompatibilityChecker checks RTCPeerConnection, RTCDataChannel, WebSocket,
MediaStreamTrack and an SDP offer containing H264/90000. WebGL is not part of
this specific test. Browser identity alone cannot supply missing engine APIs.

The exact old WPE library (SHA256
ea0d0b0df141f10ae9a17218ef63eacf15a114b8b9bbe4f04f62c40783a2a0f1)
has default WebRTC and MediaStream settings off. Its compiled feature list omits
PeerConnectionEnabled and MediaDevicesEnabled, whose source definitions are
gated on WEB_RTC and MEDIA_STREAM. Explicit runtime setters change preference
values but do not add the missing compiled features. See
D:/k80ptwrp/out/test28-cloud-capability-probe.log.

An attempted offline DOM probe could not start the original Adreno-only Mesa
display under QEMU. It is not proof of actual browser SDP success or failure.
The retained settings-only probe avoids display/GPU access.

Captured test28 logs are under D:/k80ptwrp/out/test28-{logs,live,immersive,cloud-start}-*.
There are WebProcess termination reason 0, channel-close and broken-pipe errors.
Recovery PID remained 430. No conclusive OOM/GPU fault was found in those captures.
Viewport switches to 1440x3200 and landscape 3200x1440 were logged. The cause of
all browser exits is still unresolved; do not call the browser stable.

## Independent Engine Build

- WSL distribution: SkywardBuild
- Work: /build/skyward-cloud-engine
- Source: /build/skyward-cloud-engine/wpewebkit-2.52.6
- Build: /build/skyward-cloud-engine/build
- ARM64 sysroot: /build/skyward-cloud-engine/alpine-host/target
- Clang 21 / NDK r29 at /opt/android/android-ndk-r29.
- Alpine 3.22.8 host minirootfs and ARM64 signing keys verified by pinned SHA256.
  ARM64 packages installed with signature checks, not --allow-untrusted.
- Host CMake/Ninja/Ruby/Bison/Gperf/GLib generators installed in WSL.
- configure-cloud-engine.sh completed successfully with ENABLE_WEB_RTC,
  ENABLE_MEDIA_STREAM, USE_GSTREAMER_WEBRTC, video and WebAudio enabled.
  WebAudio is an engine dependency, not restored speaker/volume adaptation.
  WebGL and encrypted media remain off; camera/microphone permission policy
  remains deny. No public distribution is permitted until licensing gates close.
- Custom source edits retain the existing AERA userspace-sandbox validation at
  WTF and Web/Network helper entry points, musl intl linking, GPU compositing
  and headless 120 Hz scheduling. They are independent of the recovery baseline.
- ENABLE_UNIFIED_BUILDS changed from OFF to ON to reduce duplicate compilation.
  Build concurrency is 2; WSL has about 7.6 GiB RAM and 2 GiB swap.
- Compilation was interrupted with SIGINT to the verified Ninja PID at the user's
  request. Last completed progress line was 299/2067 (not a meaningful overall
  completion percentage). No finished WebKit library or new recovery image yet.
- Existing objects, .ninja_log, .ninja_deps, generated sources and CMakeCache are
  preserved. Resume at the same absolute paths for incremental reuse.

## Tests Already Passed

audit-tools/test-cloud-rtc-pipeline.sh cross-compiled and ran under ARM64 QEMU.
Required webrtcbin/nice/DTLS/SRTP/SCTP/RTP/H264/OpenH264/Opus factories loaded.
The local receive-only offer reported:

    RTC_OFFER h264=1 opus=1 datachannel=1

This only verifies the GStreamer backend and SDP generation. It does not verify
WebKit JS APIs, actual ICE connectivity, a running game, video latency, independent
game multitouch or audio output. No account or remote game was used in the probe.

## Resume

Read this file and confirm the user is ready before resuming work. The phone is
not needed for compilation. First inspect logs and disk/memory; then run:

    wsl -d SkywardBuild -- bash /mnt/d/k80ptwrp/audit-tools/build-cloud-engine.sh

This resumes Ninja incrementally. Do not rerun prepare-native-worker.py, which
overwrites the original worker SDK runtime. Do not clean the new build directory.
Full recipes are in audit-tools/{prepare,configure,build}-cloud-engine.sh,
cloud-engine-toolchain.cmake, cloud-capability-probe.c,
cloud-rtc-pipeline-probe.c and test-cloud-rtc-pipeline.sh.

Remaining: finish compilation and fix errors; stage only needed runtime ELF
dependencies/plugins, preserving the existing GPU stack and silent sink; verify
actual WebKit capabilities and H264 offer within the jail; check image capacity;
then build a separate private test image and ask for device availability. The
planned worker capability diagnostics have NOT been implemented yet.

Networking: Windows proxy at 127.0.0.1:7890 worked. WSL used a process-local proxy
at 172.27.192.1:7890; this gateway may change after reboot. Recheck /proc/net/route
and connectivity before downloads. No persistent Windows/Clash changes were made.

## Storage And Backup

D drive cleanup reclaimed about 39.2 GiB physically by removing old Cargo/debug
caches and compacting the stopped WSL VHD. Latest source/image/logs were retained.
Do not delete D:/k80ptwrp/toolchains/wsl/ext4.vhdx: it is the whole build system.
At checkpoint D has about 65 GiB free; E about 31 GiB free.

New progress backup:
E:/K80Pro-Skyward-Backups/cloud-engine-20261118-before-gpu-switch

It contains the complete independent engine work tree (including incremental
objects, target dependencies and source modifications), recipes, baseline source
and history, probe/build logs and this handoff document. Verify manifest hashes
before any restore. The active build remains in WSL; no restore is normally needed.
