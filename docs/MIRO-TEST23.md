# Test23 AERA Mainline Checkpoint

2026-11-17: the user chose the current AERA implementation as the only
maintained build. Chromium is no longer a default feature or build output.
Historical source and accepted test22c images remain available for rollback.

## Changes

- Webpage typing and Pinyin candidates go directly to the focused engine
  field. Backspace and Enter use the engine's real key events, not edits of
  an empty local string. Address-bar editing stays local.
- Browser and keyboard frames are fixed. Swipes begun on keys cancel the
  press without overscrolling recovery; page touches still reach WebKit.
- The webpage backing covers the full display. Status, tools and Android
  navigation overlay it, with keyboard/navigation hit regions excluded from
  webpage touches. Showing the keyboard does not resize the webpage.
- Removed the browser-home wordmark and visible author name 澪洛依. Legal
  notices and the Kirk Lin / 红烧豆腐 credits remain.
- Browser command sending retries bounded backpressure for up to 250 ms;
  EOF is distinguished from malformed messages. Cancelled load/snapshot
  events no longer kill the session. Real worker termination still fails.
- Sandbox supervision logs exit signals and resource limits without changing
  sandbox policy, memory/process limits or device access rules.
- Browser pixel conversion now uses release opt-level 3. CPU-only comparison
  measured 26.25 to 11.34 ms at 1080x2100, 41.05 to 14.00 ms at 1440x2436,
  and 54.44 to 19.30 ms at 1440x3200. These are not webpage or panel FPS.

## Verification

- Workspace: 1129 passed, 3 existing ignored tests.
- Focused pre-build run: 435 UI and 13 browser tests passed.
- Strict host and Android all-target workspace Clippy passed.
- Real runtime extraction and Chinese font integration verified.
- Image inventory, AVB digest, exact ELF, dynamic Bionic, OMAPI, builtin
  tools, runtime and launcher pins checked independently.
- Renamed CLI --help and --version returned 0 on miro; recovery PID stayed
  435. No second display client was started.
- SHA256 image: 99365ab75a2108fd3f2abc6f71d32a706d659cd5fdbc1233cf0ab76cb9bb3a64.
- SHA256 ELF: 00e1b5e8b7692ec59b08f1bb797d7efa69d45380db8698d29ab2afed6d4b8422.
- SHA256 jail: a149479f2b952f6762a1721e3979715b7501c8a2e8afc15d30571f6162e0da17.

No changes to skyward-install, skyward-crypto, skyward-backup or miro.toml
relative to accepted eeaeb64. No partition writes, reboot or flashing in
this checkpoint. Test23 is built but has NOT been booted or UI-tested on
the phone; accepted test22c is still installed.

## Remaining Limits

The old logs contain WPE process termination and intermittent DNS failure.
Their exact termination cause is not yet established. GPU participation is
observed, but 120 FPS, audio and full Android-browser equivalence are not
claimed. AERA downloads are still private to its own data tree. Arbitrary
webpage caret movement/selection is not added to the ARP2 key protocol.

Live evidence: ../out/test22c-keyboard-live-20261117-181222 (outside repo).
Local build: ../audit-tools/build-test23-aera.sh. Independent validation:
../audit-tools/verify-test23-aera.py. The historical upstream CI pack jobs
are not an AERA runtime distribution pipeline; do not publish their plain
packer output as this tested image.
