#!/bin/sh
set -eu
export HOME=/root
export PATH=/bin:/usr/bin:/usr/sbin
export LD_LIBRARY_PATH=/usr/lib/chromium:/usr/lib/aarch64-linux-gnu
export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
export SSL_CERT_DIR=/etc/ssl/certs
if [ -f /usr/lib/chromium/vk_swiftshader_icd.json ] && [ -f /usr/lib/chromium/libvk_swiftshader.so ]; then
  export VK_DRIVER_FILES=/usr/lib/chromium/vk_swiftshader_icd.json
  export VK_ICD_FILENAMES="$VK_DRIVER_FILES"
fi
mkdir -p /tmp /root
exec /usr/lib/chromium/chromium \
  --headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage \
  --no-first-run --no-default-browser-check --disable-background-networking \
  --disable-quic --no-proxy-server --hide-scrollbars --window-size=412,915 \
  --remote-debugging-address=127.0.0.1 --remote-debugging-port=9222 \
  --user-data-dir=/root/chromium-profile \
  --disable-crash-reporter --disable-breakpad \
  "$1" >> /tmp/chromium.log 2>&1
