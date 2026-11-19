# Skyward 1.0.0 Miro Test39b

Development validation build for REDMI K80 Pro (miro), based on the stock
OS3.0.303.0.WOMCNXM recovery ramdisk. Supersedes the unreleased test38 build.

## Changes

- Retains test38 standalone image-backup discovery/restoration, shared
  Chinese/English input and advisory script warnings. Unknown commands are
  not skipped, and script exit status is not rewritten.
- Marks the installed-system environment as detached before installation.
  After installation, Home and the result page prompt for a recovery reboot
  before using system-backed services. No fixed waiting timer or forced
  reboot is added. Already-decrypted data is not relocked by this marker.
- Ignores stale keyboard/pattern releases after their field disappears or
  automatic unlocking completes. Repeated manual decryption requests and
  worker-start errors no longer terminate the recovery loop.
- Makes selected optional diagnostics nonfatal. Adds a bounded panic
  location log without credentials, input text or panic payloads.

No changes to crypto algorithms, partition-writing/ZIP/OTA algorithms, 7zz,
AVB implementation, recovery retention, or browser runtime in this revision.

## Validation And Limitations

1655 passing test invocations, 3 existing ignored. Host/Android all-target
Clippy, native/runtime checks and independent image/source verification pass.
72 multilingual/theme/density previews were checked. Full recovery_a and
running ELF hashes match. Owner data decrypts, WLAN connects, and 12 repeated
Chinese search/candidate/navigation rounds kept the same recovery PID with
no panic log. Existing standalone backups appear in Restore.

The original intermittent exit has no surviving pre-crash log, and its root
cause remains unconfirmed. These tests do not prove it cannot recur. The
post-ROM detach notice was tested offline, without reinstalling a ROM or
restoring real partitions. Cloud Genshin ICE remains unresolved; hardware
audio is not implemented. See docs/MIRO-TEST39B.md for the validation scope.

## Unpublished Draft

Keep this draft unpublished. The browser runtime's complete third-party
source/license and security-update bundle remains incomplete; requirements
in crates/skyward-browser/NOTICE.md still apply. This is not a public stable
release or an automatically offered online update.

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test39b-aera.img
Image SHA256:
b83359e67bf5aa59fb7f0a39bbd131a2833c65761e68aad044c531b807d06b77
