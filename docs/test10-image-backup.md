# test10 image-backup candidate (not flashed)

Built from the preserved test09 adaptations. Phone disconnected at the user's
request; no adb or fastboot commands are part of test10 development.

## Standalone image export

- Backup -> Partition image backup. Enumerates only existing independent boot
  partitions in slots A/B, and the shared physical super partition.
- Does not list system/vendor/odm/product logical partitions. Does not list data,
  userdata, metadata, persist, credentials or EFS partitions.
- Requires explicit single selection, readable destination and slide confirmation.
- Saves Skyward/images/<timestamp>-<partition>/<partition>.img plus .img.sha256.
  One raw file, uncompressed and unsplit. This is a byte copy, not simg/sparse.
- Opens source read-only. Does not call ensure_installed, change slots, format,
  resize, change AVB, unmount system partitions or create logical mappings.
- Refuses outstanding/unknown OTA state, concurrent UI partition workers,
  insufficient destination capacity and FAT32 for files of 4 GiB or more.
- Uses exclusive output creation, inherited destination ownership, fsync and
  full output read-back SHA-256 comparison. Cancel/error removes only newly
  created incomplete output files; an empty timestamp folder may remain.
- Existing multi-part backup and restore format remains unchanged. Exported
  images live separately and are not displayed as restorable backup manifests.
- No guarantee of coherence against an external root process writing source
  blocks concurrently. Tested with regular host files, not real phone partitions.

## Other pending test10 changes

- Explicit Flash partition image row in Install, using the existing image picker.
  Filesystem images still cannot identify system vs vendor from magic alone.
- WLAN retries ENODEV/ENXIO/EAGAIN during netdev readiness for a bounded 20 s.
  Does not change MAC, persist, firmware, reload modules or request SSR.
- User confirmed test09 Wi-Fi works after toggling: log scans 27/35/39 APs.
  This supports startup readiness as a fault; test10 cold startup is untested.
- Fast charge and project-specific online releases remain unimplemented.
