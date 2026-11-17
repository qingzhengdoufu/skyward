# Test22c Accepted Baseline

2026-11-17: both normal Chromium and independent AERA variants are preserved.
The user confirmed recovery, AERA browsing and Chinese input work on test22c.

- AERA image SHA256: eea6cd39c0e9bcb98e3ca6039c372c2a39b9b577cc6f89f997a5c38c33ee8de2.
- Standard image SHA256: 8100b6f2ad8a426b629a9ae3a50c459bb73ebf37936e82e6c83962dfe0d8cff0.
- Test22b failed before main: adding the NDK common library directory for
  static C++ archives also selected static Bionic in a dynamic PIE.
  Running the binary under a different name with --help reproduced SIGSEGV.
- Test22c stages only libc++_static.a and libc++abi.a in its search directory;
  libc.so, libm.so and libdl.so remain dynamic. Both CLIs passed on the phone.
- Only recovery_a flashed on serial 7129edbc. Readback matched the AERA image.
- recovery_b unchanged: 437cc1bf7ed8e9bf6812a80d06d6bf99be5790cbfaae5d90cf3ed06d7c3a7f48.
- No system, data format, AVB or slot changes.
- Workspace + second AERA UI test run: 1556 passed, 3 existing ignored.
- Host and Android strict Clippy, exact image/binary/AVB/config checks passed.
- Chromium source, original test20 and test21 rollback images remain intact.

Known limits: AERA WebProcess can exit on navigation, transport publishes
roughly 18 changed frames/sec during one busy interval, not 120 FPS. GPU
firmware loads through the device's native sources; ZAP fallback succeeds
from ODM. No kernel GPU fault or OOM was found in the captured failure logs.
Audio and AERA downloads/main task-list integration remain incomplete.
Captured logs are under out/test22c-browser-live-20261117-175022.
