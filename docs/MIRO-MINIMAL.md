# Independent miro comparison build

Base: Rinrroy/skyward-for-haotian f7748b576ce5f641daf761d4408d2156f9657956.
This snapshot contains its super streaming fixes and script-location fix,
before the later runtime/key-service discovery rewrite. It is not latest main.
Rust 1.99.1 is pinned for the locally available toolchain.

The full feature branch remains at D:/k80ptwrp/miro-publish, commit 986b9fa,
with an additional preserved/full-feature-986b9fa branch and source tar backup.
Nothing in that checkout was removed to create this one.

Only miro device description, FocalTech input configuration, black-frame
scanout compatibility, static key services/OMAPI, and their offline-only ADSP
startup dependency are added here. Stock init's SSR restart is removed to
avoid the previously observed shutdown hang. Automatic ART deletion is disabled.
Settings and optional saved credentials use /metadata/skyward-minimal, leaving
the full feature build's settings and saved credential untouched.

Installer, Rust 7za/zstd, block layer and crypto source match the haotian base.
There is no local ZIP queue, official CN update override, signed miro updater,
image backup extension, custom intro, USB manager, full 7zz, modem preflight,
or old quiesce experiment. Haotian's own features remain as its baseline.
WLAN and haptics are not configured for this experiment; neither is promised.

This is a comparison image, not proof the flashing problem is fixed. Modem
read-only state is deliberately not bypassed; arbitrary ROM scripts can still
erase FRP or ignore command failures. No ROM script should be automatically
re-run on failure. No formatting, slot changes or publication are part of build.

First validate recovery startup, touch dark/wake and owner data decryption.
Then compare the actual /sbin/7za resolution and decode the exact ROM's super
to a hash, not to a partition. Host pipeline tests do not establish real-device
partition writes. Do not clear data/metadata or reset credential hardware.
