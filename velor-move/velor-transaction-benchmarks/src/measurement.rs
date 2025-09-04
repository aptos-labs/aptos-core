// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use criterion::Criterion;
use criterion_cpu_time::PosixTime;

pub fn cpu_time_measurement() -> Criterion<PosixTime> {
    Criterion::default().with_measurement(PosixTime::UserAndSystemTime)
}

pub fn wall_time_measurement() -> Criterion {
    Criterion::default()
}
