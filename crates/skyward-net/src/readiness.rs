//! Bounded retry for a network device whose sysfs node precedes ioctl readiness.

use anyhow::Result;
use std::time::{Duration, Instant};

pub(crate) fn retry<T>(timeout: Duration, interval: Duration, mut attempt: impl FnMut() -> Result<T>) -> Result<T> {
    let until = Instant::now() + timeout;
    loop {
        match attempt() {
            Ok(value) => return Ok(value),
            Err(error) => {
                let transient = error.downcast_ref::<std::io::Error>().and_then(|e| e.raw_os_error())
                    .is_some_and(|code| [libc::ENODEV, libc::ENXIO, libc::EAGAIN].contains(&code));
                let now = Instant::now();
                if !transient || now >= until { return Err(error); }
                std::thread::sleep(interval.min(until.saturating_duration_since(now)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn transient_registration_errors_retry_then_succeed() {
        let mut count = 0;
        let result = retry(Duration::from_secs(1), Duration::ZERO, || {
            count += 1;
            if count < 4 {
                return Err(std::io::Error::from_raw_os_error([libc::ENODEV, libc::ENXIO, libc::EAGAIN][count - 1])).context("netdev not ready");
            }
            Ok(42)
        }).unwrap();
        assert_eq!(result, 42);
        assert_eq!(count, 4);
    }

    #[test]
    fn permission_errors_and_expired_wait_do_not_loop() {
        for (code, timeout) in [(libc::EPERM, Duration::from_secs(1)), (libc::ENODEV, Duration::ZERO)] {
            let mut count = 0;
            let result: Result<()> = retry(timeout, Duration::ZERO, || {
                count += 1;
                Err(std::io::Error::from_raw_os_error(code)).context("not ready")
            });
            assert!(result.is_err());
            assert_eq!(count, 1);
        }
    }
}
