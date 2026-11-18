# Test27 Offline Browser Repair Checkpoint

2026-11-18, based on test26-final (386a5e9). Not flashed or device-tested.
The user took the phone away for class; no further phone access is permitted
until the user explicitly says it is available again.

## Captured Failure

The saved logs at D:/k80ptwrp/out/test26-video-crash-20261118 show:

    browser supervisor: resource limit, processes=3/96,
    resident=1612283904/1610612736 bytes

This establishes that one recorded session exceeded the jail's 1536 MiB
aggregate RSS bound. It does not establish the source of every byte or prove
a specific leak. The previous worker log separately records WebProcess
termination reason 0. Recovery itself continued running. The automatic
two-retry policy then exhausted its budget and opened the text-only renderer.

## Changes

- Removed both media-volume sliders, the media setting, its persistence,
  key remapping, icons, worker command and CDP volume code. The volume.rs module
  is deleted. Old settings containing the field remain readable and do not
  serialize it again. Original hardware-key navigation and haptics remain.
- The existing clocked silent GStreamer sink is retained because it allows
  videos with audio tracks to play without a hardware soundcard. No new
  speaker or mixer adaptation; audio awaits upstream. This image is silent.
- Configure WebKit's public memory-pressure API with a 1024 MiB per-process
  base, conservative reclamation at 35%, strict reclamation at 65%, kill at
  100%, and one-second pressure polling. Use document-viewer caching and
  disable the page cache. WPE MSE buffers are bounded at video 64 MiB,
  audio 8 MiB and text 1 MiB, using the environment variable documented in
  the exact WPE 2.52.6 source. This is not Linux drop_caches or periodic killing.
- The privileged jail, its policy, launcher hash and aggregate 1536 MiB
  limit are unchanged. A pathological page can still be stopped by a bound.
- Two full-mode retries remain bounded. A session that has played media no
  longer silently falls back to the text renderer after the retries fail;
  it reports failure and offers an explicit full-mode retry. This is failure
  handling, not a cure for engine termination.
- Hidden video frames no longer invalidate the visible control-center frame
  or its hit regions. Background URL/media events also avoid unnecessary
  refreshes of other pages. Control-center WLAN and brightness now have tests
  that inject video frames between pointer down/move/up.
- Playing media is aggregated across frames with bounded, expiring document
  heartbeats. Pause/end/unload or heartbeat expiry releases keep-awake. Idle
  scheduling does not queue an expired dim deadline while video is playing.
  Manual screen-off is still respected.
- Protocol-B slot 1 supplies bounded incremental pinch in addition to the
  existing slot-0 recovery input. Enabled only on the visible full-browser
  page, not on decryption, control-center or other recovery pages. Starting
  a pinch cancels the browser's single touch instead of clicking it.
- Desktop mode's viewport injection now survives reload. Switching modes
  returns from the menu to the page and resets the zoom baseline consistently.
- Landscape actually swaps WPE backing dimensions, rotates the native pixels
  clockwise and inversely maps browser touches. This does not rotate DRM,
  recovery navigation, keyboard or control center. It is landscape web content,
  not a guarantee that a site's embedded video requests fullscreen itself.
- Corrected the old video-overlay stylesheet, which was mistakenly JavaScript
  passed to a CSS API. Only the WebKit center overlay on playing video is
  targeted; site-specific custom overlays are not globally removed.

## Verification And Limits

- Workspace: 1144 passed, 3 pre-existing ignored. A separate 445-test AERA UI
  invocation also passed, for 1589 passing invocations including repetition.
- Host and Android workspace/all-target Clippy passed with -D warnings.
- Native C++ worker and frame-layout bounds checks passed.
- Real 448-member runtime extraction and Chinese-font integration passed.
- Independent image verification passed: AVB digest, dynamic Android ELF,
  exact embedded binary/runtime, stock inventory, tools, OMAPI and jail.
- Visual previews confirm both volume bars are absent and control-center
  brightness remains. Rotation/channel-order, protocol bounds, media retry,
  keep-awake/pause/manual power and hidden-video/control-center tests passed.
- Input helper tests compile on Android; the real two-finger device sequence,
  landscape output and live touch mapping still require on-device acceptance.
- No sustained WebKit/GPU video memory test was possible after disconnection.
  Do not claim the leak, all playback crashes, a custom site's pause icon,
  120 FPS or audible output is fixed based only on these offline checks.
- Installer, decryption, backup and devices/miro.toml are unchanged. No phone
  reboot, flash, format, DSP reset, slot change, system/data/metadata write
  during this offline checkpoint. Phone remains on the last flashed test26.

## Artifact Identity

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test27-aera.img

- Image SHA256: 705d5de10a98f56b0b6eb0ede77765ae57602d83833a141c7057241048e5a3fb
- ELF SHA256: f16cd6936df2816a603dd603e4b150a7116150716638e36e82b7cd2764fa1204
- Runtime SHA256: 2493f7bdcc2330d7c528cac22586c94f2f018b1c573c4e4ce203c089f06030f7
- Worker SHA256: 55bf7c6bfa8acee6586bc458e3d8307d975bf962f039c2b02a251c6a6cea3381
- Jail SHA256: a149479f2b952f6762a1721e3979715b7501c8a2e8afc15d30571f6162e0da17

Build/check/verification recipes are in D:/k80ptwrp/audit-tools; the exact
runtime, sources/history, logs and recipes are saved to
E:/K80Pro-Skyward-Backups/miro-test27-native. Public distribution gates in
crates/skyward-browser/NOTICE.md remain in force. Previous checkpoints remain.
