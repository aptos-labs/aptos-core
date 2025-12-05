// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::Criterion;
use criterion_cpu_time::PosixTime;

pub fn cpu_time_measurement() -> Criterion<PosixTime> {
    Criterion::default().with_measurement(PosixTime::UserAndSystemTime)
}

pub fn wall_time_measurement() -> Criterion {
    Criterion::default()
}
