# Original Skyward 0.3.1 audit and selective backports

Historical record for parent test4. The later integration experiment and
its explicit exceptions are documented in MIRO-UPSTREAM031-MERGE.md.
The later native integration supersedes those exceptions as described in
MIRO-UPSTREAM-NATIVE.md; the remaining text is historical, not current status.

Checked on 2026-11-15 against kirklin/skyward, not the haotian fork.
The GitHub latest release is v0.3.1, published 2026-11-13T23:45:50Z.
Its release commit is bd34c05; main additionally contains cc68936 and c5e09f6.
Local parent ec5f49d is preserved as preserved/v1-test3-before-upstream.

## Backported

- 34dab67: try the next DNS server on SERVFAIL/REFUSED. The original DNS
  implementation and tests were brought over without replacing mirror,
  signed-release verification or the device's working WLAN configuration.
- cca526b: split long UTF-8/multiline kernel logs into bounded records and
  preserve full text in recovery.log first. Covered by focused tests.
- 2ea0065: an unreadable battery or installed system's build properties does
  not exit the UI. Keep previous periodic readings, and deduplicate failures.
  Diagnostic logging here is best-effort and cannot end the UI.
- b696542's Linux inotify dependency fix is already satisfied: zerocopy is
  an unconditional dependency on this baseline, including Linux and Android.

## Not claimed as merged

This is a selective functional alignment, not a full merge of upstream 0.3.1.
Local version remains 1.0.0 as requested. Rust 1.99.1/edition 2021 remain.

- Automatic service discovery, skyward-programs and skyward-omapi change the
  credential startup model; the verified miro static services and standalone
  OMAPI are retained, along with the working decrypt path.
- 990abf5 changes dual-system boot-set restoration and ensure_installed(),
  called from the install path. It is deliberately deferred, not represented
  as already fixed, while successful installation behavior stays frozen.
- Rust 1.100/edition 2024, shared worker and large UI restructuring are not
  mixed with these device fixes. Device configurations for other phones and
  their generated fixture inventory are not imported.
- Main's two post-release device/node-readiness fixes were reviewed, but
  are not part of the v0.3.1 release or this backport set.

The full skyward-install, skyward-block, skyward-pack and skyward-crypto trees,
plus input/decrypt and remoteproc source, still match successful d730dc0.
No phone is accessed by this change/build.

## Backup scope

The original standard backup already reads the running slot's boot and
logical system partitions. Image export now enumerates only that slot's
independent boot partitions and validates the slot again when exporting.
Shared super is retained at the user's explicit request and labelled as
containing logical partitions from both slots. It is not current-slot-only.

Original upstream implements EFS image backup from device.backup.efs, not
an automatic device-independent EFS discovery or a modem firmware backup.
Miro has no verified list in either configuration. The unavailable EFS row
is hidden, its selection cleared at startup, and the backend empty-list
guard remains. Old manifest layout stays readable; no guessed IMEI/NV
partition list or identity restore behavior is introduced.

## Plugins and browser research

The original v0.3.1 tree has no plugin/browser runtime. Haotian experimental
branches do have executable plugin frame/input and DNS forwarding, so those
are substantive features, not part of original v0.3.1 or its stable release.
This build retains local manifest browsing only, not experimental execution.

AERA-Recovery/android_bootable_recovery, branch aera-16.0, has an on-demand
WPE WebKit browser under aeraui/features/browser. Reviewed README.md,
jail_main.cpp and generated aera_browser_payload.hpp, not executed binaries.
The payload reports 40966036 compressed and 210785213 expanded bytes.
Its launcher uses libminijail, UID 99090, namespace/chroot isolation, no
capabilities and seccomp. It shares the existing WLAN route, not recovery
block devices or the framebuffer. WPE SHM frames are presented by LVGL.

This architecture is feasible to adapt but not a drop-in Rust UI widget.
It needs a browser worker/protocol, display and input integration, bounded
resource use, TLS/DNS, explicit download access and lifecycle tests. AERA's
README also states its license/source bundle and portable build/security
update process are incomplete for public distribution. No prebuilt runtime
is downloaded, trusted or bundled here. Browser remains a separate proposal.

Sources:
- https://github.com/kirklin/skyward/releases/tag/v0.3.1
- https://github.com/kirklin/skyward/commit/34dab6778ef8c0681467bc315ad2db512232fc05
- https://github.com/kirklin/skyward/commit/cca526b6a178b4496a89b5a927aa613fd07ce520
- https://github.com/kirklin/skyward/commit/2ea006551a4c535f855571eeff1c5ef53a89f216
- https://github.com/AERA-Recovery/android_bootable_recovery/tree/aera-16.0/aeraui/features/browser
