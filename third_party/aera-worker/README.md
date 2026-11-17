# AERA Worker For Miro

Based on AERA-Recovery/android_bootable_recovery commit
e438f6de62090a79e81daf00b449b48da7377753, aeraui/features/browser.
worker.cpp, protocol.hpp and platform_sandbox.hpp retain Apache-2.0 notices.

Local ARP3 adds an initial bounded native viewport handshake, a 1440x3200
maximum double buffer, 4x mobile CSS scale, and worker termination diagnostics.
The worker is dynamically linked against the original pinned musl/WPE 2.52.6
runtime. Runtime dependencies and sandbox checks are not weakened.

Build helpers are in D:/k80ptwrp/audit-tools/prepare-native-worker.py and
compile-native-worker.sh. A newly packed runtime needs new compressed and
expanded digests in skyward-browser/src/runtime.rs before use.
