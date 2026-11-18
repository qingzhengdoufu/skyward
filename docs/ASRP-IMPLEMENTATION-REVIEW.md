# ASRP Implementation Review

Read-only source inspection, 2026-11-19. Reference revision:
[27848acc86831d1523061e3483d9ba5ce47bb466](https://github.com/ASRP-Team/android_bootable_recovery/tree/27848acc86831d1523061e3483d9ba5ce47bb466).
No ASRP build scripts or binaries were executed and no ASRP code was merged.

## Browser

The [browser README](https://github.com/ASRP-Team/android_bootable_recovery/blob/27848acc86831d1523061e3483d9ba5ce47bb466/aeraui/features/browser/README.md)
describes the same AERA-derived WPE WebKit 2.52.6 approach used as our
interoperability base, not Chromium or an Android WebView. Its archive contains
musl, WebKit, GStreamer, TLS/ICU/fonts, Mesa and a separate ARM64 worker.
The root launcher validates the payload, starts an unprivileged isolated
process tree and exposes bounded shared-memory frames over SOCK_SEQPACKET.

Page composition follows EGL -> Zink -> Turnip -> KGSL. Video decode is
software, not Android MediaCodec. The documented browser frame is bounded
to 1080x2100. GPU composition does not establish 120 FPS or hardware video
decode. Our larger native viewport and Chinese input have been tested on miro,
but there is no measured cross-project performance comparison.

The browser supports history/navigation, pinch zoom, downloads and a live
session while its view is hidden. It does not give webpages recovery's
partition or general decrypted-storage access. The README explicitly disables
WebRTC, WebGL, encrypted media and PDF.js. Our private engine enables WebRTC,
but cloud-game ICE still fails: having the component is not working gameplay.
Replacing it with the inspected ASRP browser is not a cloud-game fix.

The older [BUILD_RESULT.md](https://github.com/ASRP-Team/android_bootable_recovery/blob/27848acc86831d1523061e3483d9ba5ce47bb466/aeraui/features/browser/BUILD_RESULT.md)
records a 2026-09-06 offline build and synthetic/sanitizer tests, explicitly
not on-device rendering or performance. It is historical evidence, not proof
of the current project's hardware success or failure.

## Media And Sound

The [media scene](https://github.com/ASRP-Team/android_bootable_recovery/blob/27848acc86831d1523061e3483d9ba5ce47bb466/aeraui/scenes/media_scene.cpp)
and media session implement a separate local player, with file browsing,
play/pause, relative seek, timeline, thumbnails and fullscreen presentation.
The extension filter includes mp4/mkv/webm/mov and mp3/flac/wav/m4a/aac/ogg/opus.
Actual decoding still depends on the packaged codec/runtime, not extensions.
Its isolated worker is /usr/bin/aera-media, UID 99092; the media library is
bound read-only. This is separate from browser HTML video.

The streams scene adds service searches, extraction, quality choices and HLS
playback through an extractor/media runtime. It is not arbitrary website
compatibility supplied by the browser. DRM and site-specific restrictions
still apply.

[StartBridge](https://github.com/ASRP-Team/android_bootable_recovery/blob/27848acc86831d1523061e3483d9ba5ce47bb466/aeraui/features/audio/bridge.hpp)
selects a device-specific /system/bin/aera-audio-bridge when available,
otherwise /system/bin/aera-audio-null-bridge. The fallback authenticates
known worker UIDs and drains PCM silently. Thus the source's player/audio
protocol is not proof that K80 Pro speakers work. Our removed sound/volume
adaptation remains removed, as requested.

## Other Features And Trust

The project README and source include a PC HTTPS workbench, signed update
and plugin packages, screen mirroring, a recorder/gallery, root management,
network storage and SSH/SFTP. Some features depend on device/build options
and additional runtimes. They have not been validated by us on miro.

[Host API 2](https://github.com/ASRP-Team/android_bootable_recovery/blob/27848acc86831d1523061e3483d9ba5ce47bb466/aeraui/features/plugin_api/README.md)
is a functional general plugin interface, unlike our retired manifest-only
page. Its workers run as root with recovery's capabilities, mounted partitions
and user data, without the browser's jail/seccomp/resource boundary. Signed
packages do not remove that privilege. Do not move webpages into this host
as a sandbox workaround.

## Recommendation

Retain our tested miro device/decryption/touch and installer baseline. Finish
WiFi/UI acceptance first, then diagnose our existing ICE failure. Reuse
architectural ideas selectively: independent media workers, bounded frame
back-pressure, explicit lifecycle and background-task handling. Do not import
the whole LVGL/AOSP UI stack, root plugin host or silent audio fallback as if
they solve our browser/audio defects. Preserve attribution and source/license
requirements before any distribution; browser-enabled images remain private.
