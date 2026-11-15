# Miro browser test9

Private development build, 2026-11-16. Version remains 1.0.0. Original upstream
through c5e09f6 remains merged. This supersedes test3's key/browser status.

## Phone fixes and evidence

- test3 failed before data decryption: the configured NFC service exited, and
  upstream startup returned before any discovered KeyMint service was started.
- Restored the earlier miro service order, accounts, sockets, descriptions,
  compatibility libraries and standalone OMAPI. Discovery remains available to
  other configurations but is not run alongside the configured miro chain.
- test4 and test6 logs show the owner CE files opened, with only user999 still
  closed. No owner credential was supplied by the agent. No format, slot switch,
  ROM install, super write or credential reset was performed.
- AVB unlock detection now reads kernel device_state, flash.locked and verified
  boot state before properties. Unknown is not considered unlocked and explicit
  locked evidence wins within each source. Fastboot reported unlocked=yes; this
  phone omits device_state, has property flash.locked=0 and kernel orange state.
  test6 info reported unlocked=Some(true). Both AVB checks were read as disabled;
  the agent did not toggle or write vbmeta to test the GUI.
- Removed the duplicate advanced-page USB manager entry. The home entry remains.

## Browser implementation

AERA e438f6de62090a79e81daf00b449b48da7377753 runtime is pinned by compressed and
expanded hashes. Its 445-member archive is bounded, extracted into a fresh
private tmpfs directory and deleted when the browser closes. No symlinks, device
nodes, path traversal or unresolved aliases are accepted by the parser.

Skyward's launcher uses AERA's seccomp policy and pinned Google minijail. The
WebKit processes run as UID/GID99090, no capabilities, no_new_privs=1, seccomp=2,
inside a read-only runtime. It exposes only GPU/DMA devices, narrow browser
storage and network access. The supervisor monitors memory/process count and
terminates the dedicated process group on exit. It is not a full security audit.

Miro's GPU modules are loaded on demand. The original modem overmount hid
gen80000_sqe.fw/gen80000_aqe.fw/gen80000_gmu.bin; these are read from the existing
vendor partition through a nonrecursive bind and copied only into RAM firmware
storage. GPU access permissions are restored when the supervisor exits.

The host receives sealed double-buffer pixels and validates/copies them before
acknowledgement. Only navigation, coordinates and key codes go back. Real test
and external HTTPS pages generated nonblank changing frames. On test6 the user
confirmed page display; a screenshot shows Bing loaded in Skyward. Web input
focus opens the native keyboard. Keyboard characters are sent as Unicode values.

## Test9 changes

- Web page width fills the available width whether the keyboard is open or
  closed. Keyboard opening clips the visible height without shrinking text.
  The visible slice moves to reveal the last touched field; display and touch
  use the same uniform scale and source offset. This is host-side clipping of
  the fixed AERA viewport, not dynamic WebKit viewport resizing.
- Pixel-only updates repaint the cached browser viewport, rather than rebuilding
  the whole toolbar and keyboard. Repeated unchanged status reports do not
  invalidate the UI. No measured smoothness improvement is claimed yet.
- MiSans is copied from the existing recovery resource into the private browser
  runtime for Chinese glyphs. No additional font is packed in the image.
- Profile, XDG cache and downloads live under /data/skyward-browser/{profile,
  cache,downloads}. The data filesystem must be open; no broad /data bind is
  given to the browser. These directories persist after the browser closes.
  WebKit's separately hardcoded /tmp/cache remains temporary in this build.
- Corrected the sandbox DMA parent-directory permissions. test6's root-only
  parent prevented unprivileged DMA access despite the device node permissions.

## Validation and limitations

Full workspace tests passed, including 406 UI and nine browser tests; host and
Android all-target Clippy -D warnings, release binary, system-init and packer
builds passed. Static system-init retains the nonfatal unused -no-pie warning.
Image was independently unpacked: config, OMAPI, runtime and launcher hashes,
recovery binary identity, size and AVB hash descriptor match. It is unsigned.
Host page previews cover light/dark Chinese/English, keyboard open/closed.

test9 was flashed only to recovery_a and recovery restarted. Partition readback
matches the image. test9's data-backed browser directory, CJK glyphs, keyboard
focus crop and DMA path still require a browser session after user decryption.

Not finished:
- Audio: no ALSA soundcard in recovery, even after loading the stock audio
  machine/codec/amplifier modules. No fake silent bridge is called audio support.
- AERA runtime intentionally refuses new-window actions. Some target=_blank and
  script-created links cannot navigate; arbitrary same-tab replacement needs a
  worker change and rebuilt/hash-verified runtime. This remains unresolved.
- WebKit HTTP cache is temporary; only XDG cache and profile/download data are
  persistent. No completed-download UI/export or volume control is implemented.
- Rich pages/video remain expensive. No hardware video/audio claim, no complete
  browser compatibility claim, and no end-to-end latency benchmark yet.
- The AERA runtime's full third-party source/license/update bundle is incomplete
  for public distribution. Do not upload the browser-enabled IMG to releases.

Image: skyward-v1.0.0-miro-upstream031-native-stock303-test9.img
SHA256: ece0b2e0e5c6cd454107d949ec5b0194dc7bbf43464bad5599b0b1633205d6d6
Rollback: recovery-a-before-native031-test3-20261115.img, SHA256
c47e1b7a5c551f90bd43cdd33772c67fca2dc1f2ee54943d56c52c00a3ae70c7.
