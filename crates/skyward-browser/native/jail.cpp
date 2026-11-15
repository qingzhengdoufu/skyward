// SPDX-License-Identifier: Apache-2.0
// Uses AERA's syscall policy; only the browser and RAM storage are exposed.
#include "jail_policy.hpp"
#include <libminijail.h>
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <dirent.h>
#include <fcntl.h>
#include <signal.h>
#include <string>
#include <sys/mount.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/statfs.h>
#include <sys/wait.h>
#include <syslog.h>
#include <unistd.h>

static constexpr uid_t kUid = 99090;
static void Die(const char *what) { perror(what); _exit(78); }
static void Check(int result, const char *what) { if (result < 0) Die(what); }
static volatile sig_atomic_t stopping;
static void Stop(int) { stopping = 1; }
struct Device { const char *path; uid_t uid; gid_t gid; mode_t mode; };
static Device devices[] = {{"/dev/kgsl-3d0", 0, 0, 0}, {"/dev/dma_heap/system", 0, 0, 0}};
static void GiveDevices() {
  for (auto &device : devices) {
    struct stat info{};
    if (lstat(device.path, &info) || !S_ISCHR(info.st_mode)) Die("browser GPU device");
    device.uid = info.st_uid; device.gid = info.st_gid; device.mode = info.st_mode & 0777;
    Check(chown(device.path, 0, 3003), "browser GPU group");
    Check(chmod(device.path, 0660), "browser GPU access");
  }
}
static void RestoreDevices() {
  for (auto &device : devices) {
    chown(device.path, device.uid, device.gid);
    chmod(device.path, device.mode);
  }
}

static void PersistentStorage() {
  int data = open("/data",O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
  struct statfs filesystem{};
  if (data<0 || fstatfs(data,&filesystem) ||
      (filesystem.f_type!=0xef53 && filesystem.f_type!=0xf2f52010)) Die("decrypt data before opening browser");
  if (mkdirat(data,"skyward-browser",0700) && errno!=EEXIST) Die("browser data directory");
  int parent=openat(data,"skyward-browser",O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
  close(data);
  struct stat info{};
  if (parent<0 || fstat(parent,&info) || info.st_uid || !S_ISDIR(info.st_mode) || (info.st_mode&0777)!=0700)
    Die("unsafe browser data directory");
  for (const char *name : {"profile","cache","downloads"}) {
    if (mkdirat(parent,name,0700) && errno!=EEXIST) Die("browser persistent directory");
    int child=openat(parent,name,O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
    if (child<0 || fstat(child,&info) || !S_ISDIR(info.st_mode) || (info.st_uid!=0 && info.st_uid!=kUid))
      Die("unsafe browser persistent directory");
    Check(fchown(child,kUid,kUid),"browser data owner");
    Check(fchmod(child,0700),"browser data permissions");
    close(child);
  }
  close(parent);
}

// Observe only the private process group. Terminate the whole tree when it
// exceeds its aggregate resident-memory or process bound on cgroup-v2 phones.
static bool WithinLimits(pid_t group) {
  DIR *entries = opendir("/proc");
  if (!entries) return false;
  unsigned count = 0;
  uint64_t pages = 0;
  while (dirent *entry = readdir(entries)) {
    char *end = nullptr;
    long pid = strtol(entry->d_name, &end, 10);
    if (pid <= 0 || !end || *end || getpgid(pid) != group) continue;
    ++count;
    const std::string path = std::string("/proc/") + entry->d_name + "/statm";
    FILE *file = fopen(path.c_str(), "re");
    unsigned long unused = 0, resident = 0;
    if (file) { if (fscanf(file, "%lu %lu", &unused, &resident) == 2) pages += resident; fclose(file); }
  }
  closedir(entries);
  return count <= 96 && pages * static_cast<uint64_t>(sysconf(_SC_PAGESIZE)) <= (1536ULL << 20);
}

static void Supervise() {
  const pid_t parent = getppid();
  struct sigaction action{}; action.sa_handler = Stop; sigemptyset(&action.sa_mask);
  for (int signal : {SIGTERM, SIGINT, SIGHUP}) Check(sigaction(signal, &action, nullptr), "supervisor signal");
  Check(prctl(PR_SET_PDEATHSIG, SIGTERM), "supervisor parent lifetime");
  if (getppid() != parent) Die("recovery exited");
  const pid_t child = fork();
  if (child < 0) Die("browser fork");
  if (!child) {
    for (int signal : {SIGTERM, SIGINT, SIGHUP}) ::signal(signal, SIG_DFL);
    Check(setsid(), "private browser group");
    return;
  }
  close(3); close(4);
  int status = 0;
  bool exited = false;
  while (!stopping) {
    const pid_t ended = waitpid(child, &status, WNOHANG);
    if (ended == child) { exited = true; break; }
    if ((ended < 0 && errno != EINTR) || !WithinLimits(child)) break;
    usleep(100000);
  }
  kill(-child, SIGKILL);
  if (!exited) { kill(child, SIGKILL); while (waitpid(child, &status, 0) < 0 && errno == EINTR) {} }
  RestoreDevices();
  _exit(exited && WIFEXITED(status) ? WEXITSTATUS(status) : 78);
}

int main(int argc, char **argv) {
  minijail_log_to_fd(2, LOG_DEBUG);
  if (argc != 2 || getuid() || geteuid()) return 78;
  const std::string root = argv[1];
  const std::string prefix = "/tmp/skyward-web-";
  if (root.size() != prefix.size() + 6 || root.compare(0, prefix.size(), prefix) ||
      !std::all_of(root.begin() + prefix.size(), root.end(), [](unsigned char c) { return std::isalnum(c); })) return 78;
  int directory = open(root.c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
  struct stat info{}; struct statfs fs{};
  if (directory < 0 || fstat(directory, &info) || !S_ISDIR(info.st_mode) || info.st_uid ||
      (info.st_mode & 0777) != 0700 || fstatfs(directory, &fs) ||
      (fs.f_type != 0x01021994 && fs.f_type != 0x858458f6)) Die("private RAM runtime");
  for (const char *name : {"proc", "tmp", "run", "dev", "downloads", "profile", "cache"}) {
    if (mkdirat(directory, name, 0755) && errno != EEXIST) Die("browser directory");
    struct stat child{};
    if (fstatat(directory, name, &child, AT_SYMLINK_NOFOLLOW) || !S_ISDIR(child.st_mode) || child.st_uid) Die("unsafe runtime directory");
  }
  Check(fchmod(directory, 0755), "runtime access"); close(directory);
  PersistentStorage();
  GiveDevices();
  Supervise();
  rlimit files{512, 512}, processes{96, 96}, core{0, 0}, size{64ULL << 20, 64ULL << 20};
  Check(setrlimit(RLIMIT_NOFILE, &files), "browser file limit");
  Check(setrlimit(RLIMIT_NPROC, &processes), "browser process limit");
  Check(setrlimit(RLIMIT_CORE, &core), "browser core limit");
  Check(setrlimit(RLIMIT_FSIZE, &size), "browser download limit");
  Check(syscall(SYS_close_range, 5U, ~0U, 0), "close inherited recovery descriptors");
  auto *jail = minijail_new(); if (!jail) Die("minijail");
  minijail_namespace_vfs(jail); minijail_namespace_uts(jail);
  Check(minijail_namespace_set_hostname(jail, "skyward-browser"), "private hostname");
  Check(minijail_enter_chroot(jail, root.c_str()), "browser chroot");
  Check(minijail_bind(jail, root.c_str(), "/", 0), "read-only runtime");
  const unsigned long flags = MS_NOSUID | MS_NODEV | MS_NOEXEC;
  Check(minijail_mount_with_data(jail, "proc", "/proc", "proc", flags | MS_RDONLY, "hidepid=2,subset=pid"), "browser proc");
  Check(minijail_mount_with_data(jail, "tmpfs", "/tmp", "tmpfs", flags, "size=384M,mode=1777"), "browser temporary files");
  for (const char *name : {"/run"})
    Check(minijail_mount_with_data(jail, "tmpfs", name, "tmpfs", flags, "size=64M,mode=0700,uid=99090,gid=99090"), "browser private storage");
  for (const char *name : {"profile","cache","downloads"}) {
    const std::string from=std::string("/data/skyward-browser/")+name;
    const std::string to=std::string("/")+name;
    Check(minijail_bind(jail,from.c_str(),to.c_str(),1),"browser persistent bind");
  }
  Check(minijail_mount_with_data(jail, "tmpfs", "/dev", "tmpfs", MS_NOSUID | MS_NOEXEC, "size=1M,mode=0755"), "browser devices");
  Check(minijail_mount_with_data(jail, "tmpfs", "/dev/dma_heap", "tmpfs", MS_NOSUID | MS_NOEXEC, "size=64K,mode=0755"), "browser DMA directory");
  for (const char *device : {"/dev/null", "/dev/zero", "/dev/urandom", "/dev/random", "/dev/kgsl-3d0", "/dev/dma_heap/system"})
    Check(minijail_bind(jail, device, device, 1), "browser device bind");
  minijail_change_uid(jail, kUid); minijail_change_gid(jail, kUid);
  const gid_t inet = 3003; minijail_set_supplementary_gids(jail, 1, &inet);
  minijail_use_caps(jail, 0); minijail_no_new_privs(jail);
  auto policy = aera_jail::Policy(); sock_fprog filter{static_cast<unsigned short>(policy.size()), policy.data()};
  minijail_set_seccomp_filters(jail, &filter); minijail_use_seccomp_filter(jail);
  minijail_enter(jail);
  Check(prctl(PR_SET_DUMPABLE, 0), "browser dumpability");
  char *const environment[] = {
    const_cast<char *>("PATH=/usr/bin"), const_cast<char *>("HOME=/tmp"), const_cast<char *>("TMPDIR=/tmp"),
    const_cast<char *>("LANG=zh_CN.UTF-8"), const_cast<char *>("AERA_LOCALE=zh-CN"),
    const_cast<char *>("XDG_RUNTIME_DIR=/run"), const_cast<char *>("XDG_CACHE_HOME=/cache"),
    const_cast<char *>("XDG_DATA_HOME=/tmp/data"), const_cast<char *>("XDG_CONFIG_HOME=/tmp/config"),
    const_cast<char *>("XDG_DATA_DIRS=/usr/share"), const_cast<char *>("GSETTINGS_BACKEND=memory"),
    const_cast<char *>("GIO_MODULE_DIR=/usr/lib/gio/modules"), const_cast<char *>("GIO_USE_PROXY_RESOLVER=dummy"),
    const_cast<char *>("GIO_USE_NETWORK_MONITOR=base"), const_cast<char *>("VK_DRIVER_FILES=/usr/share/vulkan/icd.d/freedreno_icd.json"),
    const_cast<char *>("MESA_LOADER_DRIVER_OVERRIDE=zink"), const_cast<char *>("LIBGL_DRIVERS_PATH=/usr/lib/dri"),
    const_cast<char *>("EGL_PLATFORM=surfaceless"), const_cast<char *>("ZINK_DESCRIPTORS=lazy"),
    const_cast<char *>("ZINK_DEBUG=sync,flushsync"), const_cast<char *>("MESA_SHADER_CACHE_DISABLE=true"),
    const_cast<char *>("GST_PLUGIN_SYSTEM_PATH=/usr/lib/gstreamer-1.0"),
    const_cast<char *>("GST_PLUGIN_SCANNER=/usr/libexec/gstreamer-1.0/gst-plugin-scanner"),
    const_cast<char *>("GST_REGISTRY=/cache/gstreamer-registry.bin"), const_cast<char *>("GST_REGISTRY_REUSE_PLUGIN_SCANNER=no"),
    const_cast<char *>("WEBKIT_CAIRO_PAINTING_THREADS=4"), const_cast<char *>("ICU_DATA=/usr/share/icu/76.1"),
    const_cast<char *>("FONTCONFIG_PATH=/etc/fonts"), const_cast<char *>("XKB_CONFIG_ROOT=/usr/share/X11/xkb"), nullptr};
  char *const args[] = {const_cast<char *>("/usr/bin/aera-browser-worker"), const_cast<char *>("--isolated-ipc-v1"), nullptr};
  execve(args[0], args, environment); Die("exec browser worker");
}
