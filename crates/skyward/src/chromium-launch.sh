set -eu
D=$SKYWARD_CHROMIUM_ROOT
mkdir -p "$D/dev" "$D/proc" "$D/sys" "$D/etc"
for node in sys proc dev; do
    /system/bin/umount "$D/$node" 2>/dev/null || true
done
[ -n "${SKYWARD_DNS:-}" ]
printf '%s' "$SKYWARD_DNS" > "$D/etc/resolv.conf"
/system/bin/mount -o bind /dev "$D/dev"
/system/bin/mount -t proc proc "$D/proc"
/system/bin/mount -t sysfs sysfs "$D/sys"
# /dev is a bind mount: never mount over the recovery's own /dev/shm.
exec /system/bin/chroot "$D" /bin/sh /run.sh "$SKYWARD_URL" serve
