#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$root"
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to the verified Linux NDK r29}"
stock=${1:?Usage: bash build-miro.sh /path/to/stock/recovery.img [output-directory]}
out=${2:-out}
mkdir -p "$out"
stock_sum=87bd663d74d8a0473f26e0ceac7ab588f4d3cf6bf2d2a50c2ecaff45ca99ff21
printf '%s  %s\n' "$stock_sum" "$stock" | sha256sum --check

cargo test --locked -p skyward-config -p skyward-pack -p skyward-bootimg -p skyward-crypto -p skyward-net -p skyward-sys -p skyward-ui -p skyward-install -p skyward-backup -p skyward-files
cargo test --locked -p skyward --test qcom_haptics
cargo test --locked -p skyward --test commands
cargo ndk -t arm64-v8a --platform 30 build --locked --release -p skyward
RUSTFLAGS='-C target-feature=+crt-static' cargo ndk -t arm64-v8a --platform 30 build --locked --release -p skyward-system-init --target-dir target/system-init
cargo build --locked --release -p skyward-pack

name=skyward-v0.2.4-miro-OS3.0.303.0.WOMCNXM
target/release/skyward-pack recovery \
    --stock "$stock" \
    --binary target/aarch64-linux-android/release/skyward \
    --system-init target/system-init/aarch64-linux-android/release/system-init \
    --config devices/miro.toml \
    --font resources/fonts/MiSansVF.ttf \
    --mono resources/fonts/KirkMono-Regular.ttf \
    --mono-license resources/fonts/KirkMono-OFL.txt \
    --omapi resources/omapi \
    --out "$out/$name.img"
sha256sum "$out/$name.img"
