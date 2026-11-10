# miro diagnostic build

## Provenance

- Source base: Rinrroy/skyward-for-haotian at
  `d6e7b7aefcba123bf0ef15252e954e45a6de747f` (0.2.3), plus the local diff.
- This is a new source build, not a repack of skyward-miro-v2b.img.
- Packaging base: the user's local stock OS3.0.303.0.WOMCNXM recovery.
- Stock SHA-256:
  `87bd663d74d8a0473f26e0ceac7ab588f4d3cf6bf2d2a50c2ecaff45ca99ff21`.
- Target test system: OS3.0.305.0.WOMCNXM, Android 16. The user reports
  recovery equivalence; binary identity with a 305 stock image is not verified.
- No stock OTA package download is needed or performed for this build.
- Kernel, init, HALs and libraries remain vendor binaries. Recovery and
  system-init are built from Rust source. The generic se_omapi prebuilt is
  the separately disclosed YARP component in resources/omapi/README.md;
  its SHA-256 is
  `dc9ab7085a958ce673030fec6fc185f41bb15b56ce0706bed1132f4a55bd7d2f`.

## Stock evidence

Read-only EROFS mounts of odm_a and vendor_a from the local 303 super:

- odm/etc/init/android.hardware.weaver-service.nxp.rc: NXP Weaver,
  vendor_nxp_weaver (2911), WAKE_ALARM.
- odm/lib64/ese_weaver.so: imports the OmapiTransport vtable and Binder death
  callback; requires android.se.omapi-V1-ndk.so and
  libjc_keymint_transport_nxp.so. This is not merely an inference from AOSP.
- odm/etc/vintf/manifest/manifest_nxp_nfc_ese.xml declares NFC and eSE1,
  but the NFC binary does not implement the secure-element AIDL interface.
- vendor/bin/hw/android.hardware.secure_element-service.qti references
  /eSE1 and loads libEseUtils/libGPQeSE; libEseUtils references /dev/nq-nci.
  The exact hardware-dependent registration path still needs a phone test.
- vendor/etc/init/ssgtzd.rc supplies the actual sockets, credentials and
  capabilities used in the new configuration.

## Changes and limits

- Start ssgtzd, the separate QTI secure-element HAL and generic OMAPI service
  before NXP Weaver. Wait for eSE1, not just NFC. Use stock service identities.
- Keep the latest socket descriptor handover and locksettings WAL fixes.
- No Thales AID, APDU bridge, Weaver slot reset or credential deletion.
- Remove automatic ART cache clearing from recovery startup.
- Remove only the stock init wait/write for boot_adsp/ssr. Keep the existing
  state-aware boot=1 recovery path; never force a DSP shutdown.
- Explicit touch compatibility mode keeps scanout active behind a black
  framebuffer and prevents suspend. This is NOT true panel-off gesture wake.
  It may consume more power and is intended to make the first data test usable.
- No AOD command numbers, guessed gesture sysfs writes, Wi-Fi tuning or haptic
  firmware from a different phone are added.
- Decryption, touch wake and library compatibility remain unverified on device.
- Upstream enables unauthenticated root ADB in recovery. Use a trusted USB host
  and do not leave this diagnostic recovery unattended.

## First phone test

Do not format data, erase metadata, remove locksettings, or reset Weaver slots.
Keep a stock recovery and a working bootloader recovery route available.
This build is only for an unlocked miro; do not use it on other models.

Before entering a credential, collect read-only evidence with platform-tools:

```text
adb devices
adb shell getprop > device-properties.txt
adb shell dmesg > dmesg-before.txt
adb logcat -b all -d > logcat-before.txt
adb shell service list > services.txt
adb shell /system/bin/recovery weaver > weaver-config.txt 2>&1
```

The `weaver` command only asks getConfig (slot count/key size/value size);
it does not read/write a slot or check a password. If it fails, do not keep
retrying a password. Return these logs so the eSE/OMAPI failure can be isolated.
Do not run DRM capture/inspection by launching a second recovery process.
Do not stop the UI or toggle MTP while collecting ADB logs.

After one normal UI unlock attempt, collect dmesg and logcat again and describe:
exact error, whether filenames under /data/media/0 become readable, elapsed
time, and whether touch survives a power-key dark/wake cycle. Do not send a
password, credential database, protector blobs, or encryption keys. Logs may
include identifiers; redact them before sharing.

True display-off/double-tap wake is a separate driver test: inspect FocalTech
gesture mode, IRQ wake and the resume sequence on this exact kernel. Enabling
power/wakeup alone does not establish that a powered-down controller reports
gestures, and software double-tap requires ordinary touch events to arrive.

## test02 Wi-Fi adaptation

- Keep test01 intact; package a new image directly from the same stock 303.
- Stock vendor/etc/init/hw/init.target.rc (early-boot) waits one second for
  /sys/kernel/cnss/fs_ready, then writes 1. Use that exact path, not haotian's
  platform-device path. Do not enable the stock SSR recovery write.
- The supplied 305 recovery.log confirms qca_cld3_peach_v2 and CNSS dependencies
  loaded. Load rfkill and that driver through modules.dep.
- Stock vendor/etc/wifi/peach_v2/WCNSS_qcom_cfg.ini exists (8027 bytes). Stage
  the running phone's copy under /etc/firmware before module loading; modem
  firmware is already exposed by bring_up. No persist file is fabricated.
- Run the installed vendor/bin/hw/wpa_supplicant with nl80211 and a volatile
  control configuration. Keep stock recovery libraries before vendor libraries,
  and publish the supplicant AIDL declaration before services start.
- Add nonfatal stage logs and retain stderr in recovery.log, without debug/key
  dump flags. Logs can still contain device/network identifiers; redact them.
- Read the MAC before spawning, so an interface-query failure cannot leak a
  supplicant child. Wi-Fi worker failures stay on the Wi-Fi page.
- Device validation is still required: scan, WPA2 authentication, DHCP, DNS,
  toggle off/on, and Wi-Fi use while data remains locked. Enterprise/EAP and
  certificate-backed networks are not promised in recovery.

## test03 charging startup candidate

- test02 phone evidence: password entry opened CE users 0 and 999 through the
  system's secdis-bound companion protectors. Keep that successful path intact.
- test02 scanned nearby Wi-Fi access points. It reported USB power offline and
  Discharging despite a configured USB data connection. ADSP stayed offline;
  qcom_subpmic repeatedly failed to read usb_online. Android charged normally
  after a normal reboot. Physical charging current has not been measured.
- The audio boot=1 loader logged "failed to read adsp state" and aborted before
  booting the DSP. Qualcomm's public adsp-loader source shows the same
  qcom,adsp-state gate (OnePlusOSS sm8750, d50b305f7da9e14715a25120a4ac7b1a4b8b97c3,
  vendor/qcom/opensource/audio-kernel/dsp/adsp-loader.c); MiCode uke-v-oss has
  the corresponding gate too. These are reference sources, not proof that
  every instruction matches the installed miro module.
- Opt miro into firmware.adsp_boot=remoteproc. Once modem firmware is mounted,
  validate the ADSP name, offline state, existing adsp.mdt selection and readable
  firmware file, recheck state, then request start through remoteproc. Leave a
  running processor alone. Do not change its firmware selection, stop/restart
  it, invoke SSR, or write boot=0. Other devices keep their previous loader path.
- No charge-current, voltage, thermal or battery-protection settings changed.
  Driver failure is nonfatal to recovery startup; diagnostics are nonfatal.
- This is a charging candidate, not an on-device verified fix. It does not fix
  the separate Weaver VINTF omission; do not interpret the working companion
  protector path as evidence that Weaver works. Preserve test02 for comparison.
- Host tests exercise offline-only startup, running/no-op, invalid states,
  wrong processor/firmware and missing firmware. Hardware boot, GLINK and
  charging still require phone testing; a sysfs write can block in the driver
  despite the userspace state checks.

## test03 measured follow-up

- The explicitly authorized recovery_a flash succeeded; recovery_b and all
  other partitions were left alone. ADSP reached running at about 7.97 seconds,
  and qcom_subpmic reported a successful cp-state send at about 9.12 seconds.
- PC USB reported ONLINE/PRESENT=1, approximately 5.10 V and 497 mA, battery
  Charging at 30.1 C. Charge counter rose from 2778000 to 2782000 uAh in two
  samples. This is software evidence, not an external power-meter measurement.
- Wi-Fi still scanned access points. Wall chargers, fast charging, wireless
  charging and test03 password-decryption regression remain unverified.
- Evidence: out/device-test03-charge-20261109-230815. The UI started at about
  29.87 seconds, including a 20-second Weaver registration wait. This historical
  result supersedes the pre-test status above, not its safety limits.

## test04 controls and startup candidate

- Keep the test03 charging path, Wi-Fi configuration and successful test02
  companion-protector decryption implementation. No credential logic changes.
- Use the existing explicit service.description mechanism for the stock file
  /odm/etc/vintf/manifest/android.hardware.weaver-service.nxp.xml. Its AIDL
  version-2 declaration separates interface IWeaver from instance default; the
  current auto-discovery looks for the combined string IWeaver/default and
  misses it. Publish this stock description before service startup. Do not
  reduce the global service timeout or skip a credential service. Removing the
  observed 20-second wait is an objective, not a measured test04 result.
- Stock init already probes qcom-hv-haptics. Opt only miro into FF_CONSTANT,
  selecting a device that advertises that capability; retain the old periodic
  default for other devices. Use 30-ms effects, slider range 0.1 to 0.4, and
  cap screen-off ticks at the configured strongest value for this backend.
  No guessed custom waveform IDs, foreign firmware, rebind or voltage tuning.
- Reference for FF_CONSTANT support: OnePlusOSS/android_kernel_oneplus_sm8750,
  branch oneplus/sm8750_b_16.0.0_oneplus_13,
  drivers/input/misc/qcom-hv-haptics.c. It supports FF_CONSTANT and periodic
  FF_CUSTOM, not the previous FF_SINE assumption. This is reference code,
  not verification of every instruction in miro's installed driver.
- Add a capability-gated double-tap-to-wake setting, saved through the existing
  /metadata/skyward/settings.json mechanism. Old settings retain enabled as
  their default. Disabled means taps do not wake; the power key still does.
  keep_panel_on remains true regardless of the preference: this is still a
  black-frame compatibility mode, not hardware gesture wake or real suspend.
  Hardware wake is explicitly deferred at the user's request.
- 545 host tests passed, including 360 UI tests. Host config/UI all-target
  Clippy and Android ARM64 release Clippy passed with -D warnings. The Android
  haptics backend was cross-compiled and linted, not exercised on a host motor.
- Chinese light/dark and scrolled settings previews were rendered and visually
  inspected. Independent unpacking verified 100 MiB, AVB unsigned SHA-256
  descriptor, stock-entry constraints and exact embedded device configuration.
  Embedded recovery/system-init hashes match the new compiled executables.
  Only recovery, system-init and device.toml differ from test03's inventory.
- test04 SHA-256:
  dd1667028ae2430c0b4bc87f8c896b7f060630ef864e2a0c72c9e42aae111591.
  No phone commands or flashing were performed for test04. Vibration, startup
  timing, Weaver registration, settings persistence and feature regressions
  still need phone testing. Existing unauthenticated root ADB and automatic
  /data read-write mounting remain; no zero-write guarantee is made.

## test04 measured follow-up

- The user-authorized flash to recovery_a succeeded. No other partition was
  flashed and no erase, format or slot switch was performed.
- User reports the controls and earlier fixes working, but vibration is weak
  even at maximum. Keep the same motor limits for the wake experiment rather
  than changing two hardware behaviors together.
- UI initialization logged at about 10.127 seconds versus test03's 29.87
  seconds. This is a startup log timestamp, not a bootloader-to-screen stopwatch.
- The normal password entry opened CE users 0 and 999; the remaining-closed
  users list was empty. This does not independently prove Weaver getConfig.

## test05 hardware gesture candidate

- Use exact installed focaltech_touch.ko and xiaomi_touch.ko, pulled read-only
  from vendor_dlkm, with vmlinux split-BTF base. The diagnostic parser and
  query-only C probe are under audit-tools; artifacts under
  out/device-test04-hw-wake. No other model's ABI is assumed.
- Module SHA-256: focaltech_touch.ko
  cd62852f74ac9fa42de481bc01b092c0d1cab22dfa7962e37d91f5dfef810b67;
  xiaomi_touch.ko
  228cd65522fcf58aab6036e3ae4f84320591ac338101a730fa07f33809639ca2.
- BTF common_data is 1032 bytes: u8 touch_id, u8 cmd, u16 mode, u16
  data_len, two padding bytes, int32 data_buf[256] at offset 8. Select primary
  controller on the fd with ioctl 0x5403, numeric argument 0; common-data
  request is 0xc4085400. Command 0 sets, command 1 reads current; mode 14
  is Touch_Doubletap_Mode. Do not use the older int[256] Xiaomi protocol.
- The exact Xiaomi driver maps mode 14 to gesture-type bit 1, which the
  FocalTech callback maps to D1 bit 7. fts_gesture_mode alone sets only
  gesture_support and is insufficient. The display suspend path programs
  the controller and arms IRQ wake; double click reports KEY_WAKEUP (143).
- Opt only miro into touch.xiaomi_v2_gesture. Configure while lit, read back
  mode 14, and wait at most 500 ms for the asynchronous FocalTech support
  flag. A failure retains the existing black-frame panel-on fallback and
  logs nonfatally. No raw registers, boot-mode changes or foreign HALs.
- With successful configuration, allow the existing real panel rest/off
  path. Consume KEY_WAKEUP from the panel or a key device as wake-only,
  gated by the user's persistent double-tap preference. Ignore normal dark
  touch events in this mode so software counting cannot mask a failed
  hardware gesture. The wake guard uses receipt time, not the pre-poll time.
- Keep CPU suspend inhibited for this entire first-stage experiment, even
  with the preference off or USB unplugged. Real display-off gesture wake
  and wake from system suspend are separate milestones; neither is claimed
  verified merely because the image compiles. No guessed fixed IRQ in code.
- Keep test04 intact. Do not flash this candidate without renewed user
  authorization. Test power-key recovery, double-tap off/on, repeated dark
  cycles and charging before considering a later full-suspend experiment.
- No credential, decryption, charging, WLAN, motor strength, kernel, vendor
  driver, partition-selection or wipe implementation changes in test05.
  Existing /data read-write mounting, metadata settings and root ADB remain.
- 551 host tests passed (362 UI), relevant host all-target and Android ARM64
  release Clippy passed with -D warnings. Source hashes match the Linux build
  mirror. Independent unpack/AVB validation passed; only recovery, system-init
  and device.toml differ from test04, and embedded binary hashes match output.
  The existing system-init linker warning about unused -no-pie remains.
- 100 MiB candidate SHA-256:
  1c0370c125b1a2c7717c6a7fc2b9836da200141a8957b2737c895fd1c406c46a.
  Not flashed or hardware-verified. The phone remains on test04; the current
  investigation only read driver state, apart from a volatile /tmp query probe.

## test05 authorized flash and startup

- The user subsequently authorized flashing test05. Rechecked the device serial,
  product miro, slot a, unlocked=yes, bootloader fastboot (not userspace),
  recovery_a size 0x6400000 and the exact candidate SHA-256 before writing.
- fastboot flash recovery_a: sending 102400 KB OKAY (2.193 s), writing OKAY
  (0.181 s). Reboot recovery OKAY; recovery ADB returned. No erase, format,
  slot switch or other partition flash was performed.
- Running /system/bin/recovery hash matches the candidate's embedded binary:
  8d2e3bb6a07dfecce8f25d63a714fd84acf08180dc7e974cf7bd8ae94978d6a4.
- UI startup logged at about 10.016 s. At 10.097 s, Xiaomi v2 double-tap=true
  readback succeeded. Lit-panel sysfs reports Gesture Mode:On, D0=0 (normal
  while lit); this alone is not proof of display-off or system-suspend wake.
- Battery status Charging, USB online=1. ADSP startup requested at 7.853 s.
  Evidence: out/device-test05-startup.log. Physical double-tap cycles, touch
  resume and test05 password-decryption regression await user testing.

## test05 failed touch resume; test04 restored

- User reports hardware double tap lights the display, but touch is lost
  after wake, and requests the previous compatibility approach. Mark test05
  unsuitable for use; successful KEY_WAKEUP is not successful touch resume.
- Saved recovery.log and dmesg before reboot as
  out/device-test05-touch-failed.log and device-test05-touch-failed-dmesg.txt.
  CRTC off at 30.791 s and 78.822 s was followed by KEY_WAKEUP at 58.279 s
  and 127.539 s. The kernel logs gesture resume callbacks as successful, but
  that does not override the user's failed touch test or establish root cause.
- With the user's rollback request, verified serial/product/slot/unlocked/
  bootloader/partition size and the preserved test04 SHA-256, then flashed only
  recovery_a. Sending OKAY (2.193 s), writing OKAY (0.165 s), reboot recovery
  OKAY. ADB returned and running recovery SHA-256 matches test04:
  25a1393027d97c7492478af76872c97d4fd0acf6b47caff9cd8d662b8d739069.
- No format, erase, slot switch or other partition flash. Test04's
  keep_panel_on=true black-frame compatibility and software double tap are
  restored; physical touch after this rollback still needs user confirmation.
- Set the working source's xiaomi_v2_gesture=false while retaining the
  experiment for reference. Future script output is named test06-compat to
  avoid overwriting the preserved test04/test05 artifacts. No test06 image was
  built or flashed. Current phone image is the original verified test04.

## test07 native waveform, tested feedback and unflashed image

Post-build result: user authorized flashing test07. Only recovery_a was
written, sending 102400 KB OKAY (2.191 s), writing OKAY (0.168 s), then
rebooted to recovery. Product miro, slot a, unlocked yes, partition size
0x6400000 confirmed before writing. Live recovery hash is
2cce50b1dfe937e2bfa649d2137fb5cafc6e2cfc04e87cb29f5c08841fef0f23,
matching the build. Boot log selects waveform 0 at 9.945 s; charging and
29 Wi-Fi APs observed. Data remained unmounted at the boot-only check;
password decryption and UI touch/haptics await user confirmation. No ROM
installation, formatting, credential operations or slot changes performed.
Historical pre-flash notes below describe the build-time state.

- User reports the constant 0.4/0.5/0.6 probes feel similar. test06 is
  superseded as a proposed haptics fix, not validated by those probes.
- Read the live device tree and disassembled the exact installed module,
  SHA-256 66dc7790ce10adf6ca8c8a6043a3cdb2b5ebbf258fc2343e8f4397ca4f4d7eae.
  event6 reports constant/periodic/custom support, no sine support. Its stored
  waveform ABI is six bytes: signed ID, returned seconds, returned millis.
  Keep the existing bank/ID ABI for other devices; opt miro into qcom_waveform.
- Miro's device tree contains effect IDs 0..5. ID 0 has four 0x7f samples,
  base vmax 3600 mV, versus the direct path's 1400 mV. These are configuration
  values, not measured motor output. No voltage/calibration sysfs was written.
- Standalone temporary probe A: constant 0.2/30 ms; B: native ID 0 at 0.2,
  driver duration 22 ms. User preferred B. Native 0.1/0.2/0.4 probes were
  distinct; user prefers 0.2. These are single-effect observations, not a
  full-image or long-term hardware-safety validation.
- New config: qcom_waveform, click=0, strength=0.1..0.4. Exponential mapping
  gives 0.2 at slider midpoint. Preserve saved settings; do not migrate or
  overwrite the user's current slider position. Screen-off ticks use the same
  waveform, first share 0.2 then 0.158, gap 55 ms; the pair is not yet tested.
- Read the duration returned by the driver and reject zero/negative/>40 ms
  durations before playback, erasing the allocated slot on that error.
- 552 selected workspace tests plus 3 host packet/duration tests passed.
  Android ARM64 release Clippy and host config all-target Clippy passed with
  -D warnings. Static system-init build retains an NDK unused -no-pie warning.
- Built test07-native-haptics, 104857600 bytes, SHA-256
  b425878e09664b02a945e3d4ab652a35f48d239e54975689fbe4a65821685ec3.
  Independent unpack and unsigned AVB digest verification passed. Compared
  with test04 only recovery, system-init and device.toml differ. Experimental
  hardware touch wake remains disabled, black-frame compatibility stays on.
- NOT FLASHED. Phone remains on test04. Full-image clicks/ticks, charging,
  decryption and touch regression checks remain pending. Only recovery_a may
  be flashed after explicit authorization; no data/metadata formatting or
  credential changes, slot switching, other-partition writes or reboots here.

## test06 haptic amplitude candidate and log review

- Isolated probes on running test04 played one 30-ms FF_CONSTANT click each
  at 0.4, 0.5 and 0.6, spaced by four seconds. Temporary TOML files and a CLI
  symlink live only under /tmp. No second UI/DRM process, persistent phone
  configuration change, reboot or flash. All commands succeeded; no correlated
  haptic driver errors were seen. Subjective feel is awaiting user feedback.
- Extend only miro's haptic amplitude range from 0.1..0.4 to 0.1..0.6.
  Duration remains 30 ms; no manual voltage, frequency, calibration or firmware
  changes. The logarithmic slider's intermediate positions also change.
  Existing screen-off tick strength is now 0.415 rather than clamped at 0.4.
- Hardware gesture experiment remains disabled; panel-on compatibility and
  software double tap remain enabled. Output is now test06-haptics, not the
  previously reserved test06-compat name. The phone remains on original test04.
- Current logs show CE users 0/999 opened with no users remaining closed,
  Wi-Fi scans of 21/27 APs and USB Charging. No panic/Oops/BUG/fatal signal was
  found in this sample; full network and all recovery workflows are not tested.
- A genuine CNSS IRQ wake imbalance warning occurs once at 19.655 s following
  calibration (cnss_disable_dev_sol_irq -> cnss_power_off_device). Preserve as
  an unresolved driver/startup issue, not a haptics regression or touch IRQ.
- Repeated fuelgauge -2730 temperature diagnostics predate this test (present
  in test02/03). Public battery temp reads 30.8..30.9 C, health Good, Charging.
  The source of the invalid reading is unresolved; never disable thermal or
  battery protections to hide it. Generic regulatory.db is absent, but the
  device's peach/regdb_xiaomi.bin loaded; band/country completeness unverified.
- Evidence: out/device-test04-haptic-review-*; Chinese report:
  out/测试说明-test06-振动增强.zh-CN.md. Current display log says SPR '-':
  inherited SPR code is present but not applied on this observed miro panel.
