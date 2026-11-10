#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")" && pwd)
: "${ANDROID_NDK_HOME:?Set Android NDK r29}"
archive="$root/resources/7zip/7z2606-src.tar.xz"
printf '%s  %s\n' a4e374c8f864f51df48c2cfd0e1413ec3b2ea030deae852d6a655402ebb61ac4 "$archive" | sha256sum --check
work="$root/target/7zip-26.06"
mkdir -p "$work"
tar -xf "$archive" -C "$work"
tool="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
make -C "$work/CPP/7zip/Bundles/Alone2" -f ../../cmpl_clang_arm64.mak -j4 \
    USE_ASM= CC="$tool/aarch64-linux-android30-clang" \
    CXX="$tool/aarch64-linux-android30-clang++" \
    LDFLAGS='-static-libstdc++ -static' LIB2=-ldl
cp "$work/CPP/7zip/Bundles/Alone2/b/c_arm64/7zz" "$root/target/7zz-android"
