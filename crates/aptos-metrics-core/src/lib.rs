// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Re-export counter types from prometheus crate
pub use avg_counter::{register_avg_counter, register_avg_counter_vec};
pub use prometheus::{
    exponential_buckets, gather, histogram_opts, register_counter, register_gauge,
    register_gauge_vec, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, register_int_gauge, register_int_gauge_vec, Counter, Encoder, Gauge,
    GaugeVec, Histogram, HistogramTimer, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, TextEncoder,
};
use std::time::{Duration, Instant};

mod avg_counter;
pub mod const_metric;
pub mod op_counters;

pub mod __private {
    pub use once_cell::sync::Lazy;
    pub use paste::paste;
}

pub trait TimerHelper {
    type TimerType;

    fn timer_with(&'static self, labels: &[&str]) -> Self::TimerType;

    fn observe_with(&'static self, labels: &[&str], val: f64);
}

impl TimerHelper for HistogramVec {
    type TimerType = HistogramTimer;

    fn timer_with(&'static self, labels: &[&str]) -> Self::TimerType {
        self.with_label_values(labels).start_timer()
    }

    fn observe_with(&'static self, labels: &[&str], val: f64) {
        self.with_label_values(labels).observe(val);
    }
}

pub struct ConcurrencyGauge {
    gauge: IntGauge,
}

impl ConcurrencyGauge {
    fn new(gauge: IntGauge) -> Self {
        gauge.inc();
        Self { gauge }
    }
}

impl Drop for ConcurrencyGauge {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}

pub trait IntGaugeHelper {
    fn set_with(&self, labels: &[&str], val: i64);

    fn concurrency_with(&self, labels: &[&str]) -> ConcurrencyGauge;
}

impl IntGaugeHelper for IntGaugeVec {
    fn set_with(&self, labels: &[&str], val: i64) {
        self.with_label_values(labels).set(val)
    }

    fn concurrency_with(&self, labels: &[&str]) -> ConcurrencyGauge {
        ConcurrencyGauge::new(self.with_label_values(labels))
    }
}

pub trait IntCounterHelper {
    type IntType;

    fn inc_with(&'static self, labels: &[&str]);

    fn inc_with_by(&'static self, labels: &[&str], by: Self::IntType);
}

impl IntCounterHelper for IntCounterVec {
    type IntType = u64;

    fn inc_with(&'static self, labels: &[&str]) {
        self.with_label_values(labels).inc()
    }

    fn inc_with_by(&'static self, labels: &[&str], v: Self::IntType) {
        self.with_label_values(labels).inc_by(v)
    }
}

pub struct LocalHistogramVec {
    inner: prometheus::local::LocalHistogramVec,
    last_flush: Instant,
}

// pub struct LocalHistogramTimer<'a> {
//     inner: prometheus::local::LocalHistogramTimer,
//     histogram_vec: &'a mut prometheus::local::LocalHistogramVec,
// }
//
// impl<'a> Drop for LocalHistogramTimer<'a> {
//     fn drop(&mut self) {
//         self.histogram_vec.maybe_flush();
//     }
// }

impl LocalHistogramVec {
    pub fn new(inner: prometheus::local::LocalHistogramVec) -> Self {
        Self {
            inner,
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_flush) > Duration::from_secs(1) {
            self.inner.flush();
        }
        self.last_flush = now;
    }
}

impl TimerHelper for std::thread::LocalKey<std::cell::RefCell<LocalHistogramVec>> {
    type TimerType = prometheus::local::LocalHistogramTimer;

    fn timer_with(&'static self, labels: &[&str]) -> Self::TimerType {
        self.with_borrow_mut(|inner| {
            inner.maybe_flush();
            inner.inner.with_label_values(labels).start_timer()
        })
    }

    fn observe_with(&'static self, labels: &[&str], val: f64) {
        self.with_borrow_mut(|inner| {
            inner.maybe_flush();
            inner.inner.with_label_values(labels).observe(val);
        });
    }
}

pub struct LocalIntCounterVec {
    inner: prometheus::local::LocalIntCounterVec,
    last_flush: Instant,
}

impl LocalIntCounterVec {
    pub fn new(inner: prometheus::local::LocalIntCounterVec) -> Self {
        Self {
            inner,
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_flush) > Duration::from_secs(1) {
            self.inner.flush();
        }
        self.last_flush = now;
    }
}

impl IntCounterHelper for std::thread::LocalKey<std::cell::RefCell<LocalIntCounterVec>> {
    type IntType = u64;

    fn inc_with(&'static self, labels: &[&str]) {
        self.with_borrow_mut(|inner| {
            inner.inner.with_label_values(labels).inc();
            inner.maybe_flush();
        });
    }

    fn inc_with_by(&'static self, labels: &[&str], v: Self::IntType) {
        self.with_borrow_mut(|inner| {
            inner.inner.with_label_values(labels).inc_by(v);
            inner.maybe_flush();
        });
    }
}

#[macro_export]
macro_rules! make_local_histogram_vec {
    ($vis:vis, $var_name:ident, $name:expr, $help:expr, $labels_names:expr, $buckets:expr $(,)?) => {
        $crate::__private::paste! {
            static [<__ $var_name>]: $crate::__private::Lazy<$crate::HistogramVec> =
                $crate::__private::Lazy::new(|| {
                    $crate::register_histogram_vec!($name, $help, $labels_names, $buckets).expect("foo")
                });
            ::std::thread_local! {
                $vis static $var_name: ::std::cell::RefCell<$crate::LocalHistogramVec> =
                    ::std::cell::RefCell::new($crate::LocalHistogramVec::new([<__ $var_name>].local()));
            }
        }
    }
}
#[macro_export]
macro_rules! make_local_int_counter_vec {
    ($vis:vis, $var_name:ident, $name:expr, $help:expr, $labels_names:expr $(,)?) => {
        $crate::__private::paste! {
            static [<__ $var_name>]: $crate::__private::Lazy<$crate::IntCounterVec> =
                $crate::__private::Lazy::new(|| {
                    $crate::register_int_counter_vec!($name, $help, $labels_names).expect("foo")
                });
            ::std::thread_local! {
                $vis static $var_name: ::std::cell::RefCell<$crate::LocalIntCounterVec> =
                    ::std::cell::RefCell::new($crate::LocalIntCounterVec::new([<__ $var_name>].local()));
            }
        }
    }
}
