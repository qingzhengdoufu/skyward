set -eu
D=$SKYWARD_CHROMIUM_ROOT
mkdir -p "$D/dev" "$D/proc" "$D/sys" "$D/etc"
for node in sys proc dev; do
    /system/bin/umount "$D/$node" 2>/dev/null || true
done
DNS=$(getprop net.dns1 2>/dev/null || true)
[ -n "$DNS" ] || DNS=223.5.5.5
printf 'nameserver %s\nnameserver 1.1.1.1\n' "$DNS" > "$D/etc/resolv.conf"
/system/bin/mount -o bind /dev "$D/dev"
/system/bin/mount -t proc proc "$D/proc"
/system/bin/mount -t sysfs sysfs "$D/sys"
# /dev is a bind mount: never mount over the recovery's own /dev/shm.
exec /system/bin/chroot "$D" /bin/sh /run.sh "$SKYWARD_URL" serve
