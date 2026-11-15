//! AERA browser transport and verified runtime preparation, separate from recovery operations.

pub mod protocol;
pub mod runtime;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub mod session;

/// Reference source revision. The runtime is never downloaded automatically.
pub const AERA_COMMIT: &str = "e438f6de62090a79e81daf00b449b48da7377753";
pub const RUNTIME_FILE: &str = "/skyward/browser/runtime.xz";
pub const LAUNCHER: &str = "/system/bin/skyward-browser-jail";
pub const LAUNCHER_SHA256: &str = "5179cac8806724d6c3faa0956134d26f09f676c55f6cb964a5e7161953943d02";
