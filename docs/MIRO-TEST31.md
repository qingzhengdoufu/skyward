# Test31 RTC Diagnostic Checkpoint

2026-11-19, private diagnostic based on test30 (f187918).
Only the worker and runtime identity pins change. Engine, GPU/dependencies,
launcher and partition operations remain test30. No compatibility fix claimed.

The cloud-site-only observer logs bounded RTC state, error names/codes and
aggregate packet/frame counters. No SDP, candidate addresses, credentials,
cookies or page text is logged. It observes native methods without changing
arguments/results. JavaScript semantic/redaction/bounds tests and strict worker
compilation pass; Rust tests have 1600 passing invocations, 3 existing ignored.
Independent image verification confirms that all runtime dependencies are
unchanged and only the worker differs from test30.

Flashed only recovery_a and verified full partition/running ELF/runtime/launcher
hashes. Slot remains A. No data formatting, system/AVB/slot changes.

## Findings

Logs: D:/k80ptwrp/out/test31-cloud-repro-20261119-002259.
Remote description, remote ICE candidate and local description calls succeed;
audio/video track events arrive. ICE moves checking -> failed, with zero
succeeded candidate pairs, received RTP packets and decoded frames. A track
event is not proof of network/media reception. Recovery PID remains 426.

A passive 60-second handshake summary observed outbound UDP/TCP STUN requests
and inbound success-class STUN responses, plus TCP connection establishment.
This does not prove transaction matching, integrity validation, candidate
nomination, DTLS or stream reception. Further validation captures recorded
zero records and provide no additional evidence. Never bypass STUN integrity
or disable the jail as a workaround. The user confirms Android's browser can
play under the same network. No cloud gameplay works in recovery yet.

The user now prioritizes WiFi credential retention, usable fullscreen exits
and existing-function cleanup. Pause cloud-game testing until those are done.

## Identity

- Image: skyward-v1.0.0-miro-upstream031-native-stock303-test31-aera.img
- Image bytes: 104857600; compressed ramdisk bytes: 97575192.
- Image SHA256: 0b4212124c6749e10fcd593539a3eed5a6b528b6b98e882802ba8a7b69ac9c87
- ELF SHA256: 897aeabdbc3aeb85791bc0dec9ca5824a52fdf165a4c7013e1f7faac5b9c2348
- Runtime SHA256: 6cc70c7bb216a9a0462db85c3577b5fadf3a74b6d1e33cf27d626d47a5c9c1d3
- Expanded SHA256: 64ca42c47bce58512a7a2eb7f90b4055f4c69ad2e8e3c0c915d6efb73c2762ea
- Worker SHA256: 591e9fe8b49ba492fcfcf26d379a1924ab12297f2379728ac3b744ce4c52131b
- Launcher SHA256: 881a1d81f43edb5d8013fa194788ffb02f2eb18eb71bb5894541a210ae80accb

Flash/readback: D:/k80ptwrp/out/test31-flash-20261119-002153.
Private-distribution restrictions in crates/skyward-browser/NOTICE.md apply.
