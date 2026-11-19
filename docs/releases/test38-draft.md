# Skyward 1.0.0 Miro Test38

Accepted development build for REDMI K80 Pro (miro), based on test37 and
the stock OS3.0.303.0.WOMCNXM recovery ramdisk.

## Changes

- Restore scans both Skyward/backups and Skyward/images in internal storage
  and USB. Existing raw IMG/SHA256 pairs are recognized without migration.
- Image restore names the original target partition, verifies size and full
  source SHA256 before writing, then flushes and verifies full target readback.
  Shared super is explicitly both-slot content.
- Chinese/English candidates are available in native text fields: feature
  search, file search, rename, WLAN fields, letter unlock, terminal, editor
  and basic-browser forms. Numeric PIN/pattern controls remain unchanged.
- Script-output warnings distinguish missing commands/files, permissions,
  capacity, I/O and selected unpack diagnostics. Nothing is skipped and
  original script exit codes are preserved. Exit-0 scripts with diagnostics
  show a completed-with-warnings result.

## Validation

1644 test invocations pass, 3 existing ignored. Host/Android all-target Clippy,
native/runtime checks and independent image/source verification pass.
216 multilingual light/dark previews were generated. User reported tested
features working after flash. Full recovery_a and running ELF hashes match,
main-user data decrypts, WLAN reaches internet, and the captured crash buffer
is empty. The user's script-ROM test completed and retained test38 in A.
The IMG backup in phone Download is SHA256-verified. See MIRO-TEST38.md.

This does not establish complete ROM correctness, exhaustive partition
restore validation or cloud-game compatibility. Cloud Genshin ICE remains
unresolved; hardware sound is not implemented.

## Unpublished Draft

The browser runtime's full corresponding third-party source/license and
security-update bundle remains incomplete. Keep this draft unpublished
until crates/skyward-browser/NOTICE.md requirements are resolved.
This is not an automatically offered online update.

Image SHA256:
4ffc1ed63ec9391e509d3e0d4430d5b1ab56739fbb43b77d79be22f69b4cd58b
