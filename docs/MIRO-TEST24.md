# Test24 Accepted Native Browser Baseline

2026-11-17: the user confirmed webpage display and automatic network time
are normal on the flashed test24 image. Preserve this checkpoint before
further browser performance experiments.

## Implemented

- ARP3 startup viewport handshake, bounded by 1440x3200, with two sealed
  maximum-size pixel slots. Each frame validates its own dimensions/length.
- WPE generates pixels at native content-area dimensions, 4x mobile CSS
  scale. On the current miro density the live backing/output are both
  1440x2440. Status/tools/navigation occupy the remaining display area;
  webpages are not stretched from 1080x2100 or hidden behind those strips.
- Keyboard appearance does not resize the engine viewport. Webpage text
  editing/Pinyin behavior from test23 is retained.
- Native-size conversion is a BGRA/RGBA channel swap instead of bilinear
  interpolation. This remains a CPU copy, not zero-copy display scanout.
- New worker built from pinned AERA source, retaining the original WPE,
  Mesa and runtime dependencies. Jail policy and launcher hash unchanged.
- WLAN online probing starts a background SNTP sample. Two provider
  responses must agree within three seconds. Only CLOCK_REALTIME changes,
  never the hardware RTC or Android's persist offset. Offline/failure does
  not fail recovery or WLAN. SNTP is not cryptographically authenticated.
- Starting full mode before data opens now explains the decryption requirement.

## Verification

- Workspace tests: 1134 passed, 3 existing ignored.
- Strict host/Android workspace Clippy passed.
- Real runtime extraction, Chinese font integration and independent image
  inventory/AVB digest/ELF/OMAPI/tool checks passed.
- Renamed CLI --help and --version returned zero on-device before flashing.
- Isolated native worker smoke test returned nonblank 1440x2492 pixels.
- Only recovery_a flashed; readback equals image. recovery_b unchanged.
- Recovery PID 423 stayed alive; data owner decrypted, WLAN connected,
  network clock corrected from Nov 12 to Nov 17. User confirmed display/time.
- GPU participation observed, KGSL fault counters all zero. This is not
  evidence of 120 FPS; transport publication and display FPS are distinct.

## Live Performance Capture

The latest continuous-scroll session in
../out/test24-fps-scroll-20261117-195336 publishes 599 changed frames across
20.147 seconds: 29.73 frames/sec. Excluding the startup interval gives
480 frames / 15.460 seconds: 31.05 frames/sec. Individual sampled intervals
are 23.58-39.74 frames/sec. Earlier sessions peaked around 48-51, but are
not substituted for this requested capture. No GPU faults or recovery
restart occurred in the sampled window; the browser was reopened during
its first seconds, and the later 20-second PID-stable window is separate.

These logs count frame publication to the recovery UI, not final display
presentations. Present timing and a controlled animation are still needed
to attribute the remaining latency or claim an exact displayed FPS.

For the stable 20.31-second PID window, CLK_TCK=100 was verified on-device.
Aggregate process CPU time was 73.46% of one core for recovery, 8.37% for
the isolated worker, and 30.82% for WPEWebProcess. GPU busy snapshots were
17.64-55.81%, with all fault counters zero. This suggests CPU display work
deserves profiling; it does not prove that the GPU or WebKit cannot limit
other pages. No clock forcing or panel mode change was performed.

Image SHA256: 84c01f7bc79f2f384ded8b91eecb28b8bc55c8597fd601e340d0c631aeaf0287.
ELF SHA256: d949f9019abdec73f38d0f207f48240edf187a9affa2a1fa21236b70478785f7.
Runtime SHA256: b2c2f3aa81a3c401471de00e3e12747af22df1f8a455dc419edd645162cce37a.
Worker SHA256: bc9477c63b9e3662886cec1d62d51af9ffb87284dc2d752b191ab96bc3ef9377.
Jail SHA256: a149479f2b952f6762a1721e3979715b7501c8a2e8afc15d30571f6162e0da17.
recovery_b SHA256: 437cc1bf7ed8e9bf6812a80d06d6bf99be5790cbfaae5d90cf3ed06d7c3a7f48.

## Remaining Risks

- Intermittent WPE WebProcess stops and peer-reset errors remain in the
  captured navigation history. Recovery did not restart; later sessions
  reopened normally. This is not a fully stable Android-browser equivalent.
- WPE headless frame pacing is 60 Hz in upstream source. Recovery's device
  config does not request the fastest panel mode. No 120 FPS claim is made.
- GPU readback, pixel copies, CPU composition and fallback snapshots remain.
- Snapshot fallback is sparse on idle pages; idle transport rate is not
  an animation or scrolling benchmark.
- Audio/download integration and Android-browser parity remain incomplete.
- The vendored worker SDK generation/build and native runtime packaging
  helpers are under ../audit-tools outside this Git checkout; E-drive backup
  includes them and the exact payload. Historical upstream CI pack jobs are
  not the distribution pipeline for this locally verified image.

The installer, decryption, backup and devices/miro.toml were unchanged
relative to accepted test23. No data format, ROM/AVB write or slot switch.
