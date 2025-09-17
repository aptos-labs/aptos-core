// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use log::LevelFilter;

struct LogLevelGuard(LevelFilter);

impl Drop for LogLevelGuard {
    fn drop(&mut self) {
        log::set_max_level(self.0);
    }
}

/// Execute a closure with logging completely disabled
pub fn with_logging_disabled<R, F: FnOnce() -> R>(f: F) -> R {
    let _guard = LogLevelGuard(log::max_level());
    log::set_max_level(LevelFilter::Off);
    f()
}

#[cfg(test)]
mod tests {
    use super::with_logging_disabled;
    use log::LevelFilter;
    use std::{panic, sync::Mutex};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_with_logging_disabled_restores_level_on_success() {
        let _guard = TEST_LOCK.lock().unwrap();
        log::set_max_level(LevelFilter::Info);
        let result = with_logging_disabled(|| {
            assert_eq!(log::max_level(), LevelFilter::Off);
            7
        });
        assert_eq!(result, 7);
        assert_eq!(log::max_level(), LevelFilter::Info);
    }

    #[test]
    fn test_with_logging_disabled_restores_level_on_panic() {
        let _guard = TEST_LOCK.lock().unwrap();
        log::set_max_level(LevelFilter::Debug);
        let caught = panic::catch_unwind(|| {
            with_logging_disabled(|| panic!("boom"));
        });
        assert!(caught.is_err());
        assert_eq!(log::max_level(), LevelFilter::Debug);
    }
}
