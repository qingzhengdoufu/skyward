# Test28 Immersive Browser Checkpoint

2026-11-18, based on test27 (60c45d7). Private offline development image.
Not flashed or device-tested. The phone is away; no ADB or fastboot access
was attempted during this checkpoint.

## Behavior

- Full browser menu: Immersive fullscreen / 沉浸全屏. A visible webpage's
  fullscreen request also enters this mode; leaving webpage fullscreen restores
  ordinary browser controls. A background page cannot steal recovery focus.
- Hide the recovery status bar, address/tools dock and Android-style bottom
  navigation. Their hidden hit regions no longer consume webpage touches.
- Resize the actual WPE native backing from the normal browser content area
  to 1440 x 3200, or 3200 x 1440 in the existing landscape content mode. CSS
  layout remains at the engine's 4x device scale; this is not 1440 CSS pixels.
  Recovery's DRM mode, keyboard and control center stay portrait.
- Reserve the top 12 dp as an escape gesture area. A downward drag exceeding
  40 dp, with vertical movement greater than horizontal movement, exits
  immersion and opens the control center. Short taps in that strip are consumed.
  The physical edge is the phone's portrait top edge, even with landscape content.
- A hardware Back event exits immersion before navigating away. K80 Pro has no
  dedicated Back key, so the top-edge gesture is its normal on-device exit.
- Webpage input focus temporarily restores recovery navigation and overlays the
  keyboard without resizing the browser backing. Closing it restores immersion.
- In-flight old-size frames are acknowledged but not painted into the resized
  viewport. Snapshot generations also reject asynchronous results requested
  before a resize. Two bounded pixel slots and the privileged jail are unchanged.
- Browser stop/basic mode and final failure restore ordinary recovery navigation.
  Existing bounded retry behavior and test27 media-memory changes are retained.

## Verification

- Workspace: 1149 passed, 3 pre-existing ignored. Separate AERA UI invocation:
  449 passed. Total passing invocations including repetition: 1598.
- Host and Android workspace/all-target Clippy passed with -D warnings.
- Native worker compiled with -Wall -Wextra -Werror. Frame-layout bounds passed.
- Real 448-member runtime extraction and Chinese-font integration passed.
- Independent image checks passed: AVB digest (not an OEM signature), dynamic
  Android ELF, exact embedded binary/runtime, stock inventory, tools, OMAPI and
  unchanged jail. Runtime member comparison against test27 changes only
  usr/bin/aera-browser-worker; all dependency files are identical.
- Native 1440 x 3200 previews reviewed in Chinese/English and light/dark themes,
  including keyboard overlay and rotated 3200 x 1440 web-content fixtures.
  UI regression tests also use 720 x 1600, check every composed pixel for absent
  chrome, hidden-control touch routing, top-pull/Back exits, keyboard viewport
  stability, stale frame rejection and navigation recovery on failure.
- Installer, crypto, VAB, backup and devices/miro.toml remain unchanged from
  test27. No system/data/metadata writes, formatting, DSP reset, slot change,
  phone reboot or flash. The phone's last flashed build remains test26-final.

## Limits And Device Acceptance

This is an immersive UI implementation, not validation that Cloud Genshin works.
Cloud-game transport, site compatibility, WebRTC and WebGL support remain
unverified. WebGL is currently disabled and permission requests are denied by
the existing worker policy. Input still has a single browser touch plus the
existing pinch gesture, not independent multi-finger game controls. Audio stays
silent; no volume controls or audio adaptation were restored.

The full WPE/GPU path, DOM fullscreen transition, real resize, rotation/touch
alignment, sustained video memory use and top-edge exit require phone acceptance.
No claim of 120 FPS, playable cloud games or cured video crashes follows from
these offline checks. Test27 itself was not device-tested after disconnection.

When the user reconnects the phone, first confirm it is available for testing.
Flash only the explicitly selected recovery partition, retain readback/hash
verification, and do not format or alter system/data/metadata/AVB/slots.

## Artifact Identity

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test28-aera.img (100 MiB)

- Image SHA256: 52d3a91fb36f3aeeb54dfe4c5d01bcd4db83d396696214d5f475e90ba56f5b5d
- ELF SHA256: e0a66f78223eb59887ab14c5ba374d7681a760e39eebcb5c0009e7fe5b2c5887
- Runtime SHA256: 1c70823d9a70bc6e7a2ea9da08c156aa03505755aeb3fb7d8a5db553ef6996cb
- Expanded runtime SHA256: 2113fc9c3d3dd3c6b2d499c33732c0ce22c5bab9c52a00b0dee68cd8839d8f5c
- Worker SHA256: af2aecf34c84685bc005dd962f49d6d25f4876d4e1ed556e9faec4d6e321ab0d
- Jail SHA256: a149479f2b952f6762a1721e3979715b7501c8a2e8afc15d30571f6162e0da17

Build/check/verification recipes are in D:/k80ptwrp/audit-tools. Save this image,
source/history, runtime, previews and logs to
E:/K80Pro-Skyward-Backups/miro-test28-native without overwriting test27.
The public-distribution gates in crates/skyward-browser/NOTICE.md still apply.
