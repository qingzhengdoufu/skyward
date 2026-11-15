# AERA Interoperability

Wire format and archive layout are implemented for compatibility with
AERA-Recovery/android_bootable_recovery at commit
e438f6de62090a79e81daf00b449b48da7377753, under
aeraui/features/browser/protocol.hpp and runtime.cpp. Those reference
sources carry SPDX-License-Identifier: Apache-2.0.

Private development images may include the pinned AERA runtime, with WebKit,
Mesa, GStreamer and fonts. Their complete source/license and security-update
requirements must be resolved before public distribution. Do not publish these
browser-enabled development images as release artifacts.
The pinned digest is byte identity, not a security audit or signature.

native/jail_policy.hpp is AERA's Apache-2.0 syscall policy, at the revision
above. native/jail.cpp is Skyward's browser-only RAM launcher using that policy
and Google minijail c46d08600b8b6548809d37be1964396efde1c3ac (BSD-style
license in the minijail source). It was locally built and tested on miro.
MiSans is copied from the existing recovery resource into the private RAM
runtime for CJK text. Existing font licensing remains applicable.

Test9 binds only /data/skyward-browser's profile/cache/download subdirectories
into the private runtime. The runtime itself remains in RAM. See
docs/MIRO-BROWSER-TEST9.md for device evidence and unresolved limitations.
