# Miro Test43: Navigation Inset and Recent-Task Clipping

UI-only repair over test42b. No Root, install, decryption, device or browser
runtime changes are included. Previous source and images remain archived.

## Reported Regressions

The navigation inset extended the final rendered scanline vertically. If
text or an icon crossed that line, it became a vertical streak below the
page. The inset now uses the explicit page background or shared keyboard
panel color. Full-browser content remains untouched; its address keyboard
uses the keyboard panel color. The pill still chooses contrasting ink.

Recent-task cards scrolled through the top status area and the bottom
clear-button area. Those areas now have fixed background masks using the
existing blurred capture. Touches hidden under the top mask are not card
clicks, and the bottom control retains its own hit area.

## Verification

Regression tests inject text-like bright pixels at the navigation boundary
and check the entire inset for repetition. Light and dark recent-task views
are rendered at several scroll positions; the status and bottom controls
must remain byte-identical, and hidden card areas must not navigate.

The local build log, independent image verification and 240 preview images
are stored in `out/build-test43-native.log`,
`out/test43-independent-report.json` and `out/test43-ui-preview/`.
The previews include home and recent tasks scrolled at two positions,
English/Chinese input, two resolutions and light/dark themes.

No phone flash or Root installation has been performed for test43.
Browser-enabled images remain private under
`crates/skyward-browser/NOTICE.md`.
