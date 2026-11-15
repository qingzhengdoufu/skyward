//! AERA browser transport and verified runtime preparation, separate from recovery operations.

pub mod protocol;
pub mod runtime;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod session;

/// Reference source revision. The runtime is never downloaded automatically.
pub const AERA_COMMIT: &str = "e438f6de62090a79e81daf00b449b48da7377753";
pub const RUNTIME_FILE: &str = "/sdcard/Skyward/browser/runtime.xz";
pub const LAUNCHER: &str = "/system/bin/skyward-browser-jail";
