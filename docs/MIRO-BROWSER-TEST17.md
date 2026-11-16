# Miro test17 browser fixes, 2026-11-16

Baseline: test16 / 6453ff441f63a724c1b17fd1cefc5fa88b2f78b4.
Installer, super/7za/zstd, modem preparation, decryption, touch and device
configuration are unchanged. No phone operation was performed after the
user chose to boot Android independently.

## Downloads

Browser and plugin downloads retain destination.part plus .part.json on
pause, connection failure or recovery exit. Retry requests Range and If-Range
only for the same resource with an ETag or Last-Modified validator. A valid
206 must match the offset, total, response length and validator; a 200
restarts instead of appending. Missing validators restart from zero rather
than concatenate potentially different firmware. Completed files retain
no-clobber behavior. A per-file lock prevents two writers; symbolic links
and nonregular part files are refused.

The download list exposes Resume / Retry and reuses the same list item.
State survives on disk, not in the in-memory list: after restarting recovery,
open the same file link again to continue. GitHub asset signature renewal
can reuse the immutable asset path, subject to validator verification.
An expired signed link needs reopening through its release page. No unknown
digest is invented and no unverified download is described as hash-verified.

## Chromium

The previous outer launcher used a printf format with literal backslash-n
characters in resolv.conf, and consulted Android net.dns1 although the
recovery keeps DHCP DNS in its own state. The new launcher uses the network
lease DNS and real newlines. Full-mode searches use a normal HTML Bing URL,
not the native reader's RSS feed. CDP now checks Page.navigate errorText and
records main-document loading failures rather than assuming success.

Existing plugin roots receive an atomic launcher update on the next start;
the pinned package and its checksum are unchanged. Certificate verification
remains enabled. Direct networking is explicit and inherited process proxy
variables are cleared, consistent with recovery's native network stack.
This does not tunnel a phone through the PC's VPN or bypass a blocked site.

## Vulkan scope

The pinned Aurora package already contains AArch64/glibc libvulkan.so.1,
libvk_swiftshader.so and vk_swiftshader_icd.json. Its SwiftShader dependencies
(libm, libgcc_s, libc and the AArch64 loader) are present in the same root.
VK_DRIVER_FILES and VK_ICD_FILENAMES now explicitly name that ICD.

This selects a CPU software Vulkan implementation; it is NOT an Adreno 830
hardware driver, GPU acceleration, confirmed WebGPU support or a TLS fix.
The conservative headless software rendering path remains. No vendor GPU
libraries, kernel module changes, DRM device acquisition or unsafe WebGL
enablement was added. Runtime loading on ARM64 remains unverified until the
next recovery test; static library presence is not a runtime success claim.

## Validation

- Nine local HTTP resume tests: connection truncation, ranges, file changes,
  pause, range refusal, immutable GitHub identities, expired links, locks and
  symlink refusal.
- UI regression tests: pause/resume/retry and engine-specific search URLs.
- CDP tests: DHCP DNS formatting and navigation errors.
- Full workspace tests, strict host and Android Clippy, Android release build.
- Host Chromium navigation, input, screenshot and attachment detection test.
- Pinned Aurora install and executable permissions test.
- Offline image inspection: AVB digest, ramdisk layout and binary identity.

Test17 was built but not flashed. A successful website load, transfer resume
and Vulkan ICD load still require phone validation. Audio is not fixed.

The user-operated ROM flash captured during test16 is archived separately:
out/test16-rom-session-20261116-231710.zip. The script ended 0 and recovery A
was restored, but per-partition write/readback and Android boot were not
verified. Its undefined checkDevice and unbounded FRP zeroing are properties
of that ROM script, not commands issued by the assistant.
