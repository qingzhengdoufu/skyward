# AERA Interoperability

Wire format and archive layout are implemented for compatibility with
AERA-Recovery/android_bootable_recovery at commit
e438f6de62090a79e81daf00b449b48da7377753, under
aeraui/features/browser/protocol.hpp and runtime.cpp. Those reference
sources carry SPDX-License-Identifier: Apache-2.0.

This crate does not bundle or execute an AERA prebuilt, WebKit, Mesa,
GStreamer, fonts or the AERA Android jail. Their source/license and
security-update requirements must be resolved before distribution.
The pinned digest is byte identity, not a security audit or signature.

The AERA sources were read, not executed. A separate local NDK build of
its jail and minijail was attempted; see the integration report for results.
