# Test32b WiFi And Fullscreen Cleanup

2026-11-19, private candidate based on test31 (315557f). Not flashed or
device-accepted. Browser/cloud-game investigation remains paused.

## Changes

- Nearby WiFi entries reuse credentials by exact SSID bytes and security;
  passwords remain in the network worker, never in UI state/debug output.
- Reconcile an already-completed supplicant connection and start DHCP even
  when the connected event predates the event socket attachment.
- Keep the existing multi-network /metadata/skyward/wlan format (maximum 16).
  Validate/bound reads, reject symlinks, atomically replace private 0600
  records and fsync the file/directory. Supplicant RAM config is created
  privately and published root:wifi 0640, not world-readable.
- Persist the remember-network switch. Disconnect and forget are distinct;
  forgetting requires confirmation and waits for the worker's real result.
  Errors leave network details usable. Rejected credentials are invalidated.
- Immersive exit accepts inward swipes from all four edges, permits diagonals,
  and restores recovery navigation. Top swipe also opens control center;
  VolumeUp exits without webpage history navigation. Edge taps pass through,
  and unsuccessful edge drags do not become accidental clicks.
- Remove the generic plugin manifest-only page, state and parsers. AERA
  browser/runtime/download controls remain intact; no user folders removed.

Pudding reference: mariusarva/skyward-pudding at 9c2bec6771f6cc10b77e9d4bcdb0867c60ffc6cd.
Its remember/auto-connect and private persistence behavior informed the review;
we did not apply its whole device patch or replace our multi-network format.
Haotian main at 7b893d5 has no generic manager; its separate webview-plugin
branch (99383b07) does have a real experimental host/download path. Removing
our placeholder does not mean no haotian branch implements plugins.

## Verification

Build recipe: D:/k80ptwrp/audit-tools/build-test32b-native.sh.
1610 passing Rust test invocations (UI 452, repeated with aera-browser);
3 existing ignored tests. Host and Android all-targets Clippy pass.
Native frame/resource/task tests and complete pinned runtime/font extraction
pass. The static system-init link retains an NDK unused -no-pie warning.

Independent image/AVB digest/ELF/stock checks pass. AVB algorithm is NONE,
not an authenticity signature. Only system/bin/recovery changes relative
to test31's ramdisk. All 380 compiled source/resource files match the Linux
mirror. Install/super/7zz/AVB/decryption/backup/device config and browser engine,
worker/runtime/jail remain unchanged. Offline UI preview is supplemental,
not phone gesture or radio validation.

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test32b-aera.img
- Bytes: 104857600; compressed ramdisk: 97570021.
- SHA256: 554914bbc6b2d5df4c37ac7939f2779e0b789bce0f0c4ed6c5c99bbdf7cb3e95
- ELF SHA256: cdbef2f3e4fcc4d265dc39d5c2ef84058f0dd53c38379edf8e7991c71259cbf6
- Runtime SHA256: 6cc70c7bb216a9a0462db85c3577b5fadf3a74b6d1e33cf27d626d47a5c9c1d3
- Jail SHA256: 881a1d81f43edb5d8013fa194788ffb02f2eb18eb71bb5894541a210ae80accb

The earlier test32 artifact predates late fixes and is not the acceptance
candidate. Preserve it as history, but use test32b for the next requested flash.

## Remaining Acceptance

Confirm password retention across recovery restarts, radio off/on, saved
network reuse after disconnection, changed-password retry and forget failure.
Verify edge taps/swipes, landscape, VolumeUp and control-center exit on phone.
No data formatting, system/AVB/slot operation, flash or reboot was performed for
this checkpoint. Audio stays removed; cloud-game ICE is still unresolved.
Credential records remain plaintext on unencrypted metadata despite restricted
permissions. Legacy 0644 records become 0600 on the next successful write,
not merely by booting this image. Persistence errors are reported, not a
guarantee of deletion when metadata cannot be written.
