# Skyward 1.0.0 Miro Test37

Development checkpoint for REDMI K80 Pro (miro), based on the stock
OS3.0.303.0.WOMCNXM recovery ramdisk and the accepted test36 code.

## UI Cleanup

- Group Advanced tools and remove duplicate Terminal/Control center entries.
- Put the update mirror preference on the Skyward update page.
- Shorten repeated installation/update/settings text while retaining risks.
- Deduplicate browser menus; retain toolbar history/bookmarks/downloads,
  tab management and context-specific mode controls.
- Preserve the existing installer, decryption, touch, AVB and browser engine.

1630 test invocations pass, 3 existing ignored; host/Android Clippy and
independent image/source checks pass. 192 offline UI previews generated.
See docs/MIRO-TEST37.md for exact scope and limitations.

## Unpublished Draft

This image is NOT a public release. The browser runtime's full corresponding
third-party source/license and security-update bundle is incomplete. Keep
the draft unpublished until crates/skyward-browser/NOTICE.md requirements
are resolved. This is not an automatically offered online update.

The phone was disconnected during this offline checkpoint: test37 is not
device-accepted yet. No official Android 17 OTA was installed as validation.
Cloud Genshin ICE remains unresolved; sound output is not implemented.

Do not format data or install a ROM to test these UI changes.

Image SHA256:
ef5cc82a68787d022026cee423c5391d8f17bb43965fafdcd07b92c7345880ad

## Subsequent Device Result

The user later reconnected the phone. Test37 was flashed only to recovery_a,
with full partition and running ELF readback verification. Main-user data
decryption and WiFi connectivity are observed in the recovery logs; the
crash buffer is empty. Android then booted with sys.boot_completed=1 in
slot A. IMG backup in Download has the same SHA256. No ROM install, data
format or AVB/slot-setting change occurred. Full UI/browser acceptance is
still pending; this result does not remove the unpublished-runtime gate.
