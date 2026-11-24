#!/usr/bin/env bash
# Private, offline Miro build. Never connects to or writes a phone.
set -euo pipefail
if [[ $# != 4 ]]; then
  echo "usage: bash scripts/build-miro-local.sh STOCK_RECOVERY RUNTIME_XZ BROWSER_JAIL OUTPUT_DIRECTORY" >&2
  exit 64
fi
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
stock=$(realpath -- "$1")
runtime=$(realpath -- "$2")
jail=$(realpath -- "$3")
output=$(realpath -m -- "$4")
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to Android NDK r29}"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$root/target}
cd -- "$root"
echo "87bd663d74d8a0473f26e0ceac7ab588f4d3cf6bf2d2a50c2ecaff45ca99ff21  $stock" | sha256sum -c
echo "894178cb432c30dcd90ed607a280f8da4b0e552dcb3098648fad3edec0bc80c5  $runtime" | sha256sum -c
echo "881a1d81f43edb5d8013fa194788ffb02f2eb18eb71bb5894541a210ae80accb  $jail" | sha256sum -c
test ! -e "$output" || { echo "output already exists: $output" >&2; exit 73; }
cargo test --locked --workspace
cargo test --locked -p skyward-ui --features aera-browser
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo ndk -t arm64-v8a --platform 30 clippy --locked --workspace --all-targets -- -D warnings
cargo run --locked --quiet -p skyward-browser --example verify-runtime -- "$runtime" resources/fonts/MiSansVF.ttf
RUSTFLAGS='-C target-feature=+crt-static' cargo ndk -t arm64-v8a --platform 30 build --locked --release -p skyward-system-init --target-dir "$CARGO_TARGET_DIR/system-init"
cargo build --locked --release -p skyward-pack
cargo ndk -t arm64-v8a --platform 30 build --locked --release -p skyward
mkdir -p -- "$output/browser-stage"
cp -- "$runtime" "$output/browser-stage/runtime.xz"
cp -- "$jail" "$output/browser-stage/skyward-browser-jail"
cp -- "$CARGO_TARGET_DIR/aarch64-linux-android/release/skyward" "$output/skyward.elf"
"$CARGO_TARGET_DIR/release/skyward-pack" recovery --stock "$stock" --binary "$output/skyward.elf" \
  --system-init "$CARGO_TARGET_DIR/system-init/aarch64-linux-android/release/system-init" \
  --config devices/miro.toml --omapi resources/omapi --browser-stage "$output/browser-stage" \
  --font resources/fonts/MiSansVF.ttf --mono resources/fonts/KirkMono-Regular.ttf \
  --mono-license resources/fonts/KirkMono-OFL.txt --out "$output/recovery.img"
(cd -- "$output" && sha256sum recovery.img skyward.elf > SHA256SUMS)
echo "Private build complete: $output"
