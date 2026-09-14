// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared harness for the mono benches: each `mono` bench runs under wall
//! time and, where the platform allows, under hardware counters.

use criterion::{
    measurement::{Measurement, ValueFormatter},
    Criterion, Throughput,
};
use std::{cell::RefCell, io, time::Duration};

/// Criterion parameter naming the wall-time measurement in bench ids.
pub const WALL_TIME: &str = "time";

/// When set, a counter that cannot be opened aborts the run instead of being skipped.
pub const REQUIRE_COUNTERS: &str = "MONO_MOVE_BENCH_REQUIRE_COUNTERS";

/// A hardware event counted per iteration.
#[derive(Clone, Copy)]
pub enum Counter {
    Instructions,
    Cycles,
}

impl Counter {
    pub const ALL: [Counter; 2] = [Counter::Instructions, Counter::Cycles];

    /// Criterion parameter naming this measurement in bench ids.
    pub fn name(self) -> &'static str {
        match self {
            Counter::Instructions => "instructions",
            Counter::Cycles => "cycles",
        }
    }

    /// Unit labels for counts scaled by 1, 10^3, 10^6 and 10^9.
    fn units(self) -> [&'static str; 4] {
        match self {
            Counter::Instructions => ["instr", "Kinstr", "Minstr", "Ginstr"],
            Counter::Cycles => ["cycles", "Kcycles", "Mcycles", "Gcycles"],
        }
    }
}

/// Wall-time configuration shared by every bench.
pub fn wall_time() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
}

/// Configuration counting `counter` per iteration, or `None` if the counter
/// cannot be opened on this machine.
pub fn hardware_counter(counter: Counter) -> Option<Criterion<HardwareCounter>> {
    match HardwareCounter::open(counter) {
        Ok(measurement) => Some(
            // Counts barely vary between samples, so a few short samples suffice.
            Criterion::default()
                .with_measurement(measurement)
                .sample_size(10)
                .warm_up_time(Duration::from_millis(500))
                .measurement_time(Duration::from_millis(1500)),
        ),
        Err(err) => {
            let message = format!("cannot open the {} counter: {err}", counter.name());
            if std::env::var_os(REQUIRE_COUNTERS).is_some() {
                panic!("{message}");
            }
            eprintln!("warning: {message}; skipping");
            None
        },
    }
}

/// Defines `main`: `$mono` runs under wall time and every available hardware
/// counter, `$controls` under wall time only.
macro_rules! bench_main {
    ($mono:path, $controls:path) => {
        fn main() {
            {
                let mut criterion = $crate::support::wall_time().configure_from_args();
                $mono(&mut criterion, $crate::support::WALL_TIME);
                $controls(&mut criterion);
            }
            for counter in $crate::support::Counter::ALL {
                if let Some(criterion) = $crate::support::hardware_counter(counter) {
                    let mut criterion = criterion.configure_from_args();
                    $mono(&mut criterion, counter.name());
                }
            }
            $crate::support::wall_time()
                .configure_from_args()
                .final_summary();
        }
    };
}
pub(crate) use bench_main;

/// Criterion measurement counting a hardware event on the calling thread, in
/// user space only.
pub struct HardwareCounter {
    counter: RefCell<imp::Counter>,
    formatter: CountFormatter,
}

impl HardwareCounter {
    fn open(counter: Counter) -> io::Result<Self> {
        Ok(Self {
            counter: RefCell::new(imp::Counter::open(counter)?),
            formatter: CountFormatter {
                units: counter.units(),
            },
        })
    }

    fn read(&self) -> u64 {
        self.counter
            .borrow_mut()
            .read()
            .expect("a counter that opened can be read")
    }
}

impl Measurement for HardwareCounter {
    type Intermediate = u64;
    type Value = u64;

    fn start(&self) -> u64 {
        self.read()
    }

    fn end(&self, start: u64) -> u64 {
        self.read() - start
    }

    fn add(&self, v1: &u64, v2: &u64) -> u64 {
        v1 + v2
    }

    fn zero(&self) -> u64 {
        0
    }

    fn to_f64(&self, value: &u64) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &self.formatter
    }
}

/// Formats counts with a metric prefix.
struct CountFormatter {
    units: [&'static str; 4],
}

impl CountFormatter {
    fn scale(&self, typical: f64, values: &mut [f64]) -> &'static str {
        let (factor, unit) = if typical < 1e3 {
            (1.0, self.units[0])
        } else if typical < 1e6 {
            (1e-3, self.units[1])
        } else if typical < 1e9 {
            (1e-6, self.units[2])
        } else {
            (1e-9, self.units[3])
        };
        for value in values {
            *value *= factor;
        }
        unit
    }
}

impl ValueFormatter for CountFormatter {
    fn scale_values(&self, typical_value: f64, values: &mut [f64]) -> &'static str {
        self.scale(typical_value, values)
    }

    /// Reports the count per element or byte rather than a rate.
    fn scale_throughputs(
        &self,
        typical_value: f64,
        throughput: &Throughput,
        values: &mut [f64],
    ) -> &'static str {
        let per_iteration = match throughput {
            Throughput::Bytes(n) | Throughput::Elements(n) => *n as f64,
        };
        for value in values.iter_mut() {
            *value /= per_iteration;
        }
        self.scale(typical_value / per_iteration, values)
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        self.units[0]
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use perf_event::{events::Hardware, Builder};
    use std::io;

    pub struct Counter(perf_event::Counter);

    impl Counter {
        pub fn open(counter: super::Counter) -> io::Result<Self> {
            let event = match counter {
                super::Counter::Instructions => Hardware::INSTRUCTIONS,
                super::Counter::Cycles => Hardware::CPU_CYCLES,
            };
            // The builder's defaults observe only this thread and only user space.
            let mut inner = Builder::new().kind(event).build()?;
            inner.enable()?;
            Ok(Self(inner))
        }

        pub fn read(&mut self) -> io::Result<u64> {
            self.0.read()
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use std::io;

    pub enum Counter {}

    impl Counter {
        pub fn open(_counter: super::Counter) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "hardware counters need Linux perf_event",
            ))
        }

        pub fn read(&mut self) -> io::Result<u64> {
            match *self {}
        }
    }
}
