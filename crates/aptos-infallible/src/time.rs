// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::time::{Duration, SystemTime};

/// Gives the duration since the Unix epoch, notice the expect.
pub fn duration_since_epoch() -> Duration {
    let system_time = SystemTime::now();
    system_time_since_epoch(&system_time)
}

/// Gives the duration of the given time since the Unix epoch, notice the expect.
pub fn system_time_since_epoch(system_time: &SystemTime) -> Duration {
    system_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("System time is before the UNIX_EPOCH")
}
