# Miro upstream 0.3.1 integration experiment

## Identity and rollback

- Original upstream: kirklin/skyward release commit bd34c05 (v0.3.1).
- Local parent: 12b9ba0, retained unchanged in miro-haotian-minimal.
- Experimental branch: experiment/miro-upstream031, separate checkout.
- Device build version stays 1.0.0. Image filename identifies the experiment.
- This is a Git merge with deliberate local conflict resolutions, not a
  byte-for-byte upstream build and not a completed phone validation.

## Integrated

- Rust 1.100.0, Edition 2024, resolver 3, Clippy/rustfmt and Android ARM64.
  Rust 1.99.1 remains installed for the successful original build.
- Upstream Binder serving, parcel/wire changes, skyward-omapi and
  skyward-programs service/manifest/init parsing and their fixture tests.
- Automatic key-service discovery is built and available to devices that
  do not opt into configured_keys. No two key chains run together.
- Upstream dual-system boot-set fingerprint protection (990abf5), including
  UI warnings for Rewritten/Unrecorded and explicit switching confirmation.
- Shared worker infrastructure, adopted by the installer, system switching,
  wipe, file work, partition work and snapshot merge.
- MTP improvements, host-test support, filesystem-space helpers and hex
  encoding changes from upstream. Isolated DNS/log/readings fixes already
  backported in test4 remain in place.
- Packer gains upstream stock-description extraction, while retaining the
  miro-specific ramdisk safety modifications and standalone OMAPI packing.

## Deliberately not made identical to upstream

1. UI state and runtime architecture: retain local app.rs/ui.rs instead of
   replacing them with upstream's split app directory and Pages runtime.
   This preserves intro, USB notifications/manager, image export, signed
   own updates, current-slot backup, themes, revised corners, password
   messages, remembered credentials and the working touch policy.
2. Miro key startup: configured_keys=true retains the explicit tested
   KeyMint/Gatekeeper/eSE/Weaver chain, socket credentials and standalone
   se_omapi. Upstream discovery/in-process OMAPI is present but not used
   for this device until separately tested. The service supervisor itself
   is upstream's Started model, with configured sockets converted and a
   five-second restart backoff retained. This supervisor change needs
   device validation even though the service configuration stays the same.
3. Crypto compatibility: retain the local locksettings reader, sibling
   spblob/secdiscardable selection, made-up users, typed credential
   rejection and bounded Weaver wait. Metadata encryption helpers move
   to upstream encryption.rs. Bounded Binder calls are ported onto the
   new callback-capable Binder implementation.
4. Installation: preserve raw super recognition, 4 MiB writes/window
   verification, local-file/FUSE behavior, write flushing, script name/path
   in /skyward/run, stall reporting/timeouts, AVB GUI/backup and post-install
   option. Builtin sevenzip/unzip/zstd and AVB implementations remain local.
   The installer now uses upstream Worker; dual-system ensure_installed()
   can reject an operation before writes, so this is NOT the unchanged
   successful installation path despite retaining its super enhancements.
5. Haptics: retain the tested Qualcomm waveform 0 ABI, strength 0.1..0.4 and
   screen-off ticks. Upstream's alternative sample waveform helper is
   retained for its tests but not enabled on miro.
6. Touch/power/charging: retain black-frame keep_panel_on, software double
   tap, no SSR/restart of a running ADSP and stock-init SSR removal. ADSP
   discovery/wait is moved into the existing remoteproc helper. No hardware
   gesture experiment or stock kernel change is introduced.
7. WLAN: retain Peach v2 only, readiness retries, remembered networks and
   automatic reconnect. Do not adopt the untested two-driver/MAC-file
   configuration from upstream's miro device file.
8. Readings, backup and update workers retain their local implementation
   rather than all being rewritten onto Worker in this experiment.
9. Local release trust stays pinned to qingzhengdoufu/skyward and its signed
   manifest. Upstream automatic multi-device/ZIP publishing is NOT enabled.
   Imported upstream and inherited haotian workflows are manual-only here;
   upstream's publisher is additionally gated to its own repository.
   No push, tag, signed release or online update publication was done.
10. Plugins stay manifest browsing only. Original upstream 0.3.1 has no
    executable plugin/browser implementation; no haotian experiment or
    AERA WebKit runtime was bundled.

Main's post-release cc68936 and c5e09f6 are not part of this v0.3.1 merge.
Version number remains local 1.0.0, not upstream 0.3.1.

## Verification and remaining work

Workspace tests (including 401 UI tests and all upstream fixture tests),
host Clippy -D warnings, Android workspace/all-targets Clippy -D warnings,
Android release, static system-init and host packer build all passed.
The static helper's linker prints a nonfatal unused -no-pie warning.
Independent unpack verified image size, AVB hash descriptor, stock inventory,
device configuration, embedded recovery binary and builtin command links.
The image is unsigned for an unlocked bootloader, as the prior test images.
Chinese/English light/dark targeted page previews were rendered.

No phone access, flash, reboot, AVB/slot change or data operation was done.
Boot, credential retry/Weaver timeout, WLAN, haptics, touch after dark, USB,
MTP, ZIP/super install and post-install recovery preservation require device
validation. Passing host tests is not evidence that these work on the phone.

Build script: parent audit-tools/build-upstream031.sh.
Image: out/skyward-v1.0.0-miro-upstream031-stock303-test1.img.
SHA256: cec26ec6bdb672704437e3ab07a2da57aec8ca0212be1a5ea73aae1e4d01cd19.
