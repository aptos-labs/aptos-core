// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use criterion::Criterion;
use criterion_cpu_time::PosixTime;

pub fn cpu_time_measurement() -> Criterion<PosixTime> {
    Criterion::default().with_measurement(PosixTime::UserAndSystemTime)
}

pub fn wall_time_measurement() -> Criterion {
    Criterion::default()
}
