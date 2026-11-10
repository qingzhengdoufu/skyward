# Full 7-Zip 26.06

Official source: https://github.com/ip7z/7zip/releases/tag/26.06
Archive: 7z2606-src.tar.xz, SHA256
a4e374c8f864f51df48c2cfd0e1413ec3b2ea030deae852d6a655402ebb61ac4

build-7zip.sh builds the complete Alone2 (7zz) console variant using Android
NDK r29, ARM64 API 30, static libc/C++ linkage, no assembly. Android's pthread
implementation is in libc, so LIB2=-ldl replaces the Unix -lpthread setting.
No source changes; build flags are in the script. No desktop Windows binary
is used. system/bin/7z and 7za alias 7zz. The old Rust ZIP subset remains only
as a host-test/fallback path for images not built with --sevenzip.

Includes upstream codecs/handlers and commands, including 7z/LZMA2 compression,
encrypted archives, ZIP, tar, xz, zstd and RAR decoding (not RAR compression).
This is a root CLI, not a sandbox; scripts remain responsible for destination
paths and passwords. Do not extract untrusted archives over system paths.

Copyright Igor Pavlov and contributors. See License.txt and unRarLicense.txt,
also installed under /skyward/licenses/7zip in the recovery image. The full
source archive is supplied here alongside the build instructions. It remains
a separate executable, not linked into the GPL Rust binary. unRAR sources may
not be used to develop a RAR/WinRAR-compatible archiver.
