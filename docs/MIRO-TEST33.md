# Test33 Browser Transport And Control Center

2026-11-19, private development build based on test32b / 4ec005e.
The user requested building/flashing, followed by PC hibernation.

## Browser Evidence

Android Chrome 157.0.8103.78 successfully played Cloud Genshin under the
same Xiaomi 12S hotspot. Read-only CDP queries captured a selected, nominated
IPv4/UDP pair: local prflx, remote host, controlled role, DTLS connected.
iceServers was empty; the selected pair was not TURN relay. Remote offer
was not ICE-lite. BUNDLE covers audio/video/application, with one distinct
credential pair and active answer DTLS setup. H264 2144x966 frames and Opus
packets increased; short samples reported up to 60 decoded FPS, not panel FPS.
No credentials, complete SDP, cookies or account tokens were exported by CDP.

Recovery test32b sent IPv4 UDP/TCP STUN checks and received matching replies
with valid fingerprints. Temporary bounded uretprobe checks returned
STUN_VALIDATION_SUCCESS. Further candidate-processing probes recorded no
events and are inconclusive. They were removed before Android testing.
Do not infer an authentication failure or claim the empty probes identify
the failing code path. A prior WiFi change left old-address sockets in the
browser; restarting only the browser restored outbound STUN, but not ICE.

## Changes And Offline Validation

The WPE 2.52.6 GStreamer endpoint now initializes GLib's default TLS backend
before creating the WebRTC pipeline. glib-networking defers GnuTLS global
initialization, which libnice also requires for STUN HMAC. Before this change
the real WebKit local-peer test asserted in stun_sha1 with HMAC result -402
(GNUTLS_E_LIB_IN_ERROR_STATE). Explicit initialization removed the error,
without bypassing authentication or printing keys. This offline failure is
not proof the phone's earlier STUN checks had the same defect.

GStreamer 1.26.3 rejects BALANCED and keeps NONE. WebKit's balanced mapping
now uses MAX_COMPAT, preserving separate candidate gathering while accepting
remote BUNDLE. The default balanced/balanced local test previously failed
to negotiate BUNDLE; a max-bundle offer with balanced answer also failed.
Patched engine tests pass both shapes: three media sections, BUNDLE, both
ICE connected/completed and DataChannel ping/pong, no diagnostic preload.
The exact stripped shipping stage also passes max-bundle -> balanced.

Media regression: 71 frames, 70 content changes, stable paused position,
seek to 2000 ms, resume/end-of-stream. AAC decode remains silent. No hardware
sound, volume controls, WebGL, UA, IP protocol, sandbox policy or transport
authentication checks were changed.

Control-center tiles accept 400 ms holds. WiFi opens WLAN; haptics/MTP open
Settings; tap-to-wake opens ScreenOff; other tiles open their existing pages.
One haptic follows the vibration preference. Release is consumed and does
not also toggle. Horizontal movement cancels the hold; short taps retain
their prior actions. Both direct and animated control-center routing tested.

Rust build: 1614 passing invocations, 3 existing ignored. UI 454 repeated
with aera-browser; host/Android all-target Clippy, native frame/resource/task
tests and full runtime/font extraction pass. The existing static system-init
NDK unused -no-pie link warning remains. Engine compiles and the exported
ten-file patch reproduces its source against the pinned official archive.

Independent verification: 100 MiB image, AVB digest (algorithm NONE, unsigned),
ELF, stock inventory and 380 Linux source/resource files match. Relative
to test32b only recovery and runtime.xz change. Within the runtime only
usr/lib/libWPEWebKit-2.0.so.1 changes; 474 members, worker/GPU/jail/dependencies
are preserved. Installer/super/7zz/AVB/decryption/backup/device config unchanged.

## Device Result And Remaining Work

Only recovery_a flashed after identity/slot/unlocked/partition-size checks.
Full partition readback, on-disk ELF, /proc/826/exe, runtime and jail hashes
match. Slot remains A. User 0 decrypts and WiFi reaches the internet.
No system, data-format, AVB or slot-setting operation was performed.

Important: recovery PID 445 aborted once at uptime ~27 s, then init restarted
it as PID 826. No Rust assertion/backtrace was recovered; cause is unresolved.
PID 826 remained running in subsequent observations and the browser published
frames, but that is not a guarantee of crash-free startup. Preserve crash.log,
dmesg and full logcat; do not label the first abort harmless.

The phone's new browser still logs ICE checking -> failed with zero received
media frames. Cloud Genshin is NOT fixed on device, despite the corrected
offline defects. Next steps: capture sanitized local/remote description and
candidate metadata in recovery, compare with Chrome, and fix the actual
remaining interop issue. Ordinary intermittent webpage clicks are deferred
as requested. No public upload: browser distribution gates still apply.

## Identity And Paths

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test33-aera.img
- Image bytes: 104857600; compressed ramdisk: 97566133.
- Image SHA256: 0a0bb21ac1d2f2f3dd019a6841a2e910312e9267cc6f9a8e71963af84faad8ea
- ELF SHA256: b44e2d0e95a96412d24707476add6d371d4740b47776bd6651f06b8ae6c12505
- Runtime bytes: 45837028; expanded: 223837301; members: 475.
- Runtime SHA256: 894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5
- Expanded SHA256: 65db1393edefff0a0c919ecd1675dc02ac217861171842fe048669ae82c9bd8f
- Engine patch SHA256: 45bbfa153c3596a1c3a86cfba5548eec845b4a1ef914e08deea8e5e205bf33dd

Build/capture/verification tools: D:/k80ptwrp/audit-tools.
Flash evidence: D:/k80ptwrp/out/test33-flash-20261119-042849.
Chrome evidence: out/chrome-cloud-20261119032545 and 20261119033321.
Android screenshots/logs: out/android-cloud-baseline-20261119-030153.
Recovery pre-Android baseline: out/cloud-recovery-baseline-before-android-20261119.
E backup target: E:/K80Pro-Skyward-Backups/miro-test33-browser-native.
Prior full engine/sysroot: ../miro-test29-cloud-native/engine-worktree.tar.
