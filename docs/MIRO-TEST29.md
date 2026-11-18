# Test29 WebRTC Engine Checkpoint

2026-11-18. Private development image based on test28 (4578ba1).
No phone access, flash, reboot or partition writes during this checkpoint.
Test28 remains the rollback image; the phone was last observed in Android.

## Scope

- Replace WPE WebKit 2.52.6 with a separately compiled ARM64/musl build with
  WEB_RTC, MEDIA_STREAM and GStreamer WebRTC enabled. Real DOM APIs and H.264
  offer generation are present, not user-agent or preference-only emulation.
- Use matched Alpine GStreamer 1.26.3 libraries/plugins with ICE/libnice,
  DTLS, SRTP, SCTP, RTP, Opus and OpenH264. audiofx supplies scaletempo in
  place of the previous standalone scaletempo plugin.
- Enable WebRTC in the worker. Once per session, the first completed HTTPS
  page logs only interface booleans from an isolated script world. It does
  not create a connection or read the page's account/content state.
- Preserve the existing Adreno/Mesa GPU stack and its dependency closure,
  silent audio sink, external minijail and resource limits. No microphone or
  camera permission is granted. Audio/volume adaptation remains removed.
- Recovery UI, immersion/escape gestures, Chinese input, downloader, installer,
  decryption, VAB, backup and devices/miro.toml are unchanged from test28.

## Evidence

- Workspace tests: 1149 passed, 3 existing ignored. Separate AERA UI test run:
  449 passed. Total passing invocations including repetition: 1598.
- Linux and Android workspace/all-target Clippy passed with -D warnings.
  Worker compiled with -Wall -Wextra -Werror; native frame bounds passed.
- JavaScript capability probe: eight success/failure/timeout/cleanup cases
  passed. Normal interface probe creates no peer connection.
- Minimal shipping runtime: ARM64 dynamic loader and plugin dependency checks
  passed. GStreamer generated H.264/Opus/DataChannel SDP. Two local peers
  completed ICE/DTLS/SCTP and exchanged a DataChannel ping/pong.
- A real WebKit page under ARM64 QEMU reported secure, WebSocket,
  RTCPeerConnection, RTCDataChannel, MediaStreamTrack, H.264 and offer=true,
  timeout=false. It loaded local HTML with an HTTPS base, not a real account.
- The DOM test starts from a copy of the minimal shipping runtime, adding only
  a separately inventoried test-only llvmpipe EGL stack for QEMU and the same
  recovery font injected on device. No developer sysroot is copied wholesale.
  It runs without the diagnostic abort preload. Test-only drivers/probes/QEMU
  are not included in the image.
- Minimal runtime video regression: 70 decoded video frames, 69 changes;
  pause position stable, seek to 2000 ms, resume and end-of-stream passed.
  AAC decoded into the existing silent sink; no speaker output was attempted.
- Complete 474-member runtime extraction and Chinese font integration passed.
- Independent image verification: 100 MiB capacity, Android ELF, AVB hash
  descriptor (not an OEM signature), exact binary/runtime, stock inventory,
  OMAPI, existing command aliases and unchanged jail passed. All 125 runtime
  changes match the engine/dependency staging manifest; eleven protected
  GPU/dependency/silent-sink paths match test28, including alias targets.
- The exported eight-file WebKit patch applies to the pinned official source
  and reproduces the modified source files byte-for-byte.

## Limits

The QEMU DOM test uses software EGL, root in a disposable test chroot and the
test-only sandbox opt-out. Production rejects that opt-out and still requires
the external jail. Physical Adreno and minijail acceptance are not proven by
these offline tests. No production security check was disabled.

Cloud Genshin's published initial compatibility check requires the APIs and
H.264 offer now tested. This does NOT establish service acceptance, successful
remote ICE/TURN connectivity, game streaming, frame rate, latency or stability.
WebGL is still off. Input still lacks independent multi-finger game controls.
Audio remains silent; Opus/WebAudio support is not Android audio routing.

Test28 WebProcess termination/channel-close errors remain an unresolved device
issue, not a proven OOM or GPU failure. The new engine is not claimed to fix all
browser exits. A 120 Hz headless scheduling interval does not prove 120 FPS.

## Device Acceptance

Ask for the phone to be made available before accessing it. Flash only the
explicitly selected recovery partition after checking identity/current slot,
and verify exact readback hashes. Never format data/metadata or alter system,
AVB, slots, DSP state or persist calibration for this browser experiment.
Never run a second recovery/DRM client for diagnostics.

First verify recovery startup, data decryption, WiFi, GPU and inherited jail.
Then test ordinary browsing/video, immersion and the cloud compatibility page;
do not automate account login or claim gameplay before an observed result.
Retain the complete test28 rollback image and its hashes.

## Artifact Identity

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test29-aera.img

- Image bytes: 104857600; compressed ramdisk bytes: 97594805.
- Image SHA256: 60b6988b2a5ce749fb451d3acd885ab71ba25e7bc24b806f583ed98cab60a0cd
- ELF SHA256: c0f2f0637f7fae64825cab864fbafd6856f4cdc9d1ad9858351aad3f42b43088
- Runtime compressed bytes: 45859860; expanded bytes: 223699149; members: 474.
- Runtime SHA256: 1060a5724ff7709cf961fab1edf1ce3be7a3d970ed3ed90d09406e78a3bf421d
- Expanded SHA256: 2d8777e89e65ffdcd60c3e4aa0a217c70a26d6202e5f34503e99e150af48fe7a
- Worker SHA256: 6d35b16252742d9508fe5da52d568f244c749c3c0de3079b2084c60cc7ecaec3
- Jail SHA256: a149479f2b952f6762a1721e3979715b7501c8a2e8afc15d30571f6162e0da17
- Official WPE source SHA256: b2bafef2751625b7fdf530f230ff0f542ff0eeba3590c3a989d931b2a55c858e
- Local engine patch SHA256: acb2e457efa8bdeed95b9449fb476ad43d362317d5110c8595ae48091f0b2275

The engine patch is in third_party/aera-worker/engine. Build/test/pack/verify
recipes and logs are under D:/k80ptwrp/audit-tools and out. The independent
engine source, sysroot and incremental build are in WSL SkywardBuild under
/build/skyward-cloud-engine; the recovery mirror is /build/miro-native031.
Use two build jobs on the current host. Do not rerun prepare-native-worker.py,
which replaces the accepted old worker SDK runtime.

Save source/history, engine work tree/patch/recipes, image/runtime and logs to
E:/K80Pro-Skyward-Backups/miro-test29-cloud-native without overwriting older
backups. The public distribution gates in crates/skyward-browser/NOTICE.md
remain open: do not upload this browser-enabled image as a public release.

## Correction To The Earlier Checkpoint

The earlier checkpoint called public WebKit feature identifiers
PeerConnectionEnabled and MediaDevicesEnabled. The actual public identifiers
are PeerConnection and MediaDevices. The corrected probe reports both as
compiled/enabled in this build, and the real DOM test independently verifies
the APIs. Historical checkpoint files otherwise remain unchanged.
