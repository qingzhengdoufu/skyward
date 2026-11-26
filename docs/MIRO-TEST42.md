# Miro Test42: Immersive Pill and Recent Tasks

Final image: test42b. It also covers the browser keyboard bottom surface.
UI-only revision over test41. The previous image and working-source archive
remain in `E:/K80Pro-Skyward-Backups/miro-test41-gestures-root/`.

## Changes

- The gesture pill continues the surface directly above it, including the
  keyboard or modal backdrop, instead of filling a separate navigation band.
  Its ink follows the underlying brightness. Bottom control spacing stays
  reserved so keys and destructive confirmations do not overlap the gesture.
- New-folder dialogs accept left and right edge back swipes despite their
  fixed layout. A cancelled dialog creates nothing.
- Full-browser pages and their address sheet arbitrate edge gestures before
  handing a touch to the web engine. A horizontal inward swipe returns via
  the existing navigation-back action; vertical drags and taps still reach
  the page. The existing fully immersive edge-exit behavior is unchanged.
- Recent tasks use two columns, external icons and names, rounded full-aspect
  previews, and a blurred capture of the previous page. No page title or
  header back button is drawn. The circular close control clears only recent
  cards and returns home; it does not stop installation, downloads, Root
  operations, or the browser process.
- Thumbnails are 360 pixels wide, twelve task groups at most, shared through
  Arc instead of cloned for every render. Images now render inside the page's
  drawing bands and transitions, fixing preview/header overlap on scrolling.

## Preserved

No changes to ROM/OTA installation, ZIP extraction, snapshots, AVB, slot
handling, recovery preservation, device configuration, decryption or browser
engine/runtime. KernelSU and module-install code is identical to test41.
No phone flashing or Root installation is part of the offline build.

## Validation

The local workspace holds `out/build-test42-native.log`,
`out/test42-independent-report.json` and `out/test42-ui-preview/`.
Tests include both-edge modal/browser back, vertical web touch forwarding,
pill surface continuity, rounded preview clipping, band-vs-whole rendering,
and clearing recent cards while an installer remains active.

Browser-enabled images remain private under
`crates/skyward-browser/NOTICE.md`. Hardware gestures need user acceptance
after flashing; offline tests do not replace that check.
