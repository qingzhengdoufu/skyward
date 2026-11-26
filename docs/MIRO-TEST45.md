# Miro Test45: Full-Height Page With Overlay Gesture Pill

UI-only correction over test44. The previous builds matched the adjacent
surface color but still reduced the ordinary page viewport by 24 dp. That
was not a true overlay and left a separate navigation area.

## Layout

All native page themes now use the complete physical screen height. The
gesture pill painter draws only the pill and never copies or fills its
background. Page content can therefore remain visible underneath it.

The 24 dp inset is a control-safe spacing value, not a shortened canvas.
Keyboards, keypads and bottom toolbars include the spacing in their own
surface and draw through to the screen edge. Their keys and buttons remain
above the gesture start region. Browser keyboards no longer apply the inset
twice. Full-browser viewport size and fully immersive exit behavior are
unchanged. Scrollable pages can bring their final controls above the pill.

Individual recent-card close buttons, clear-all, side-back repairs and
fixed recent-task top/bottom masks from test44 are retained.

## Verification

Pixel tests draw a nonuniform surface in the entire bottom inset and assert
that the overlay changes no pixels outside the pill. Native text keyboards
are checked for full-height backgrounds and key bounds above the safe area.
Light/dark page, keyboard, task dismissal and scroll previews are generated
at two resolutions in three languages.

See the local out/build-test45-native.log, preview-test45.log,
test45-independent-report.json and test45-ui-preview folder. No phone
operations are performed by the offline build itself.

## Device Acceptance And Delivery

On 2026-11-26 (Asia/Shanghai), test45 was flashed to the current A-slot
recovery partition on the K80 Pro. The complete partition readback matched
the local image, and the running recovery binary matched the built ELF.
The recovery log confirmed owner storage decryption, WiFi internet access
and network time synchronization. No recovery panic log was present.
The user accepted the result before requesting source publication.

The image was delivered to the phone as
Download/skyward-v1.0.0-miro-test45-aera.img and its SHA-256 was checked:

    33d4f3c67b4c72d26328e6891528ef9e8d5503242f73967de01dd126a499b952

Built ELF SHA-256:

    21043576a200b8696b0a8a81ff47a6537d628d35ea1288f4d95ad325aad1e87b

Offline verification: 1757 passing test invocations, 3 ignored invocations,
host/Android all-target Clippy, native helper checks and 252 UI preview
images. Only system/bin/recovery differs from test40 in the image inventory.

KernelSU installation, module installation, destructive restore/formatting
and ROM flashing were not exercised during this acceptance. No Root was
installed, no system partition was flashed, no data was formatted and no
slot was switched for this test. Historical test41-test44 notes describe
their build-time state, not the final test45 acceptance.

ROM installation, extraction, AVB, slot handling, recovery retention,
snapshot merging, device configuration, decryption and browser runtime
remain unchanged. KernelSU/module installation remains the test41 code.
Browser-enabled images remain private under
crates/skyward-browser/NOTICE.md.
