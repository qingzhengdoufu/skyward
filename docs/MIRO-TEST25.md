# Test25 Browser And Media Checkpoint

2026-11-17. Private development image, not yet flashed or tested on-device.
Accepted test24 remains at preserved/miro-test24-accepted (e88b3d5).

## Changes

- Entering Browser defaults to full WPE mode. An active session is reused.
- After interruption, retry full mode twice, then fall back to the lightweight
  renderer. Redirects cannot reset the failure limit. A hidden browser never
  forces navigation; reopening or explicit reload can try full mode again.
- Removed the experimental wording from browser entry/full-mode labels.
- Fixed snapshot row bounds: accept a final row without unused tail padding,
  but reject truncated rows and invalid extents. Valid CSS-size compatibility
  snapshots keep aspect ratio. Bad snapshots are nonfatal and logs are limited.
- Keep the previous worker log across an automatic restart and record IPC and
  process-termination diagnostics. This does not prove all WebProcess crashes
  are resolved; the original termination reason still needs device evidence.
- Added a status-bar media volume slider and hardware volume-key handling.
  Values persist with a quiet 15% default. They control HTML media volume, not
  an Android system mixer. The silent output path is still silent at 100%.
- Enabled video fullscreen within the existing isolated native viewport.
  Added official GStreamer 1.26.9 scaletempo, subenc and soup plugins.
- Replaced the incompatible autoaudiosink implementation with a GstBin that
  implements GstChildProxy and clocks silent audio, keeping video playback
  independent of unavailable hardware speaker output.

## Offline Verification

- Full workspace: 1139 tests passed, 3 existing ignored in the final recheck.
  A further settings compatibility test and the three-size slider check were
  added after packaging. The final release rebuild is byte-identical to the
  packaged ELF; no runtime code changed after image creation.
- Strict host/Android workspace Clippy passed; C++ frame-layout test passed.
- Independent image inspection passed: 100 MiB, AVB digest, Android dynamic
  ELF, stock-entry whitelist, exact recovery/runtime/jail, tools and OMAPI.
- Real runtime extraction and recovery Chinese font integration passed.
- Screenshot previews checked at native 1440x3200, including keyboard overlay.
  Slider touch checks cover 720x1600 and both native-density variants.
- Actual bundled ARM64 GStreamer, under QEMU, plays a four-second H.264/AAC
  MP4 via localhost HTTP byte ranges. Playing, paused position, seek to two
  seconds, resumed EOS and changed decoded video pixels all passed:
  70 video frames, 69 changes, HTTP Range 3624-160944, EOS true.
- Audio decode: 139 PCM buffers, all 139 nonzero. This is decoded audio data,
  NOT audible speaker output. The sink is intentionally clocked and silent.
- These tests do not exercise WebKit's on-device GPU video composition,
  cross-origin player behavior, hardware video decoding or thermal performance.
  HLS/DASH and DRM/Widevine were not added or validated; no all-sites claim.
- No new FPS measurement. No 120 FPS claim.

## Audio Investigation

Earlier live checks found no ALSA soundcard. ADSP was running; the offline
remoteproc0 is SPSS, not ADSP. Exact stock audio modules were loaded in RAM,
and TFA986x devices at I2C 0x34/0x35 probed, but the machine soundcard did not
bind. SPF/LPASS dependency failures need another live read-only capture.

The saved test24 kernel log independently records "No soundcards found" and
"spf_core_add_child_devices: apm is not up". A running ADSP alone therefore
does not establish an operational audio graph.

Local stock ODM audio/sku_sun/mixer_paths_overlay_static.xml replaces the
speaker path with TFA Profile=stereo (left/right for separate speakers). It
must not be confused with the generic WSA speaker route in mixer_paths_sun_mtp.
Neither route is applied here. Do not hardcode PCM card/device zero or mixer
gain before identifying the registered soundcard and safe playback route.

Next on-device checks, only when the user reconnects the phone: read cards,
PCM devices, deferred probes, LPASS clock links and machine-driver binding;
then compare stock module dependencies. Only after a card exists should a
short low-level playback test and isolated browser PCM bridge be considered.
No DSP reset/SSR, boot_adsp/boot=0, persist writes or calibration changes.

The abandoned hardware-output draft is separately preserved under
D:/k80ptwrp/out/test25-audio-addon-draft, not compiled into this image.

## Identity And Preservation

- Image SHA256: 83fb72e4afdf6265538a09ab9e99ca2ee12387c48862cde7d01b3571daa511a1
- ELF SHA256: 8e98f104ae0f18d36764a47d460e9d36d022fc3597fdce3382d44be84e253f4a
- Runtime SHA256: 74cb01865d7cc291b904bea82c6f4025fb22d4e17aa8c9427b7e2dc551cb7460
- Worker SHA256: 4bb5ef2cee793c15334cc10c0d283ca7dcf96336e7908c702f3d6995dca04697
- Jail SHA256: a149479f2b952f6762a1721e3979715b7501c8a2e8afc15d30571f6162e0da17

Installer, decryption, backup and devices/miro.toml unchanged from test24.
No phone commands, partition writes, slot changes or formatting during this
offline checkpoint. Flashing still requires renamed CLI preflight, recovery_a
only and readback verification after reconnection.

Exact runtime, sources/history, build recipes, SDK source archives and test
logs are backed up at E:/K80Pro-Skyward-Backups/miro-test25-native.
Runtime distribution restrictions in crates/skyward-browser/NOTICE.md apply.
