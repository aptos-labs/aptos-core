// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod __private {
    pub use once_cell::sync::Lazy;
    pub use paste::paste;
}

use crate::{IntCounterHelper, IntCounterVecHelper, TimerHelper};
use std::{
    cell::RefCell,
    thread::LocalKey,
    time::{Duration, Instant},
};

const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

pub struct ThreadLocalIntCounter {
    inner: prometheus::local::LocalIntCounter,
    last_flush: Instant,
}

impl ThreadLocalIntCounter {
    pub fn new(shared: &prometheus::IntCounter) -> Self {
        Self {
            inner: shared.local(),
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        if self.last_flush.elapsed() > FLUSH_INTERVAL {
            self.inner.flush();
        }
        self.last_flush = Instant::now();
    }
}

impl IntCounterHelper for LocalKey<RefCell<ThreadLocalIntCounter>> {
    type IntType = u64;

    fn inc(&'static self) {
        self.inc_by(1);
    }

    fn inc_by(&'static self, v: Self::IntType) {
        self.with_borrow_mut(|x| {
            x.inner.inc_by(v);
            x.maybe_flush();
        })
    }
}

pub struct ThreadLocalIntCounterVec {
    inner: prometheus::local::LocalIntCounterVec,
    last_flush: Instant,
}

impl ThreadLocalIntCounterVec {
    pub fn new(shared: &prometheus::IntCounterVec) -> Self {
        Self {
            inner: shared.local(),
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        if self.last_flush.elapsed() > FLUSH_INTERVAL {
            self.inner.flush();
        }
        self.last_flush = Instant::now();
    }
}

impl IntCounterVecHelper for LocalKey<RefCell<ThreadLocalIntCounterVec>> {
    type IntType = u64;

    fn inc_with(&'static self, labels: &[&str]) {
        self.with_borrow_mut(|x| {
            x.inner.with_label_values(labels).inc();
            x.maybe_flush();
        });
    }

    fn inc_with_by(&'static self, labels: &[&str], v: Self::IntType) {
        self.with_borrow_mut(|x| {
            x.inner.with_label_values(labels).inc_by(v);
            x.maybe_flush();
        });
    }
}

pub struct LocalHistogramTimer {
    inner: Option<prometheus::local::LocalHistogramTimer>,
    parent: &'static LocalKey<RefCell<ThreadLocalHistogramVec>>,
}

impl LocalHistogramTimer {
    fn new(
        inner: prometheus::local::LocalHistogramTimer,
        parent: &'static LocalKey<RefCell<ThreadLocalHistogramVec>>,
    ) -> Self {
        Self {
            inner: Some(inner),
            parent,
        }
    }
}

impl Drop for LocalHistogramTimer {
    fn drop(&mut self) {
        self.inner = None;
        self.parent.with_borrow_mut(|x| x.maybe_flush());
    }
}

pub struct ThreadLocalHistogramVec {
    inner: prometheus::local::LocalHistogramVec,
    last_flush: Instant,
}

impl ThreadLocalHistogramVec {
    pub fn new(shared: &prometheus::HistogramVec) -> Self {
        Self {
            inner: shared.local(),
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        if self.last_flush.elapsed() > FLUSH_INTERVAL {
            self.inner.flush();
        }
        self.last_flush = Instant::now();
    }
}

impl TimerHelper for LocalKey<RefCell<ThreadLocalHistogramVec>> {
    type TimerType = LocalHistogramTimer;

    fn timer_with(&'static self, labels: &[&str]) -> LocalHistogramTimer {
        self.with_borrow_mut(|x| {
            let timer = x.inner.with_label_values(labels).start_timer();
            LocalHistogramTimer::new(timer, self)
        })
    }

    fn observe_with(&'static self, labels: &[&str], val: f64) {
        self.with_borrow_mut(|x| {
            x.inner.with_label_values(labels).observe(val);
            x.maybe_flush();
        });
    }
}

#[macro_export]
macro_rules! make_thread_local_int_counter {
    (
        $(#[$attr:meta])*
        $vis:vis,
        $var_name:ident,
        $name:expr,
        $help:expr $(,)?
    ) => {
        $crate::thread_local::__private::paste! {
            static [<__ $var_name>]: $crate::thread_local::__private::Lazy<$crate::IntCounter> =
                $crate::thread_local::__private::Lazy::new(|| {
                    $crate::register_int_counter!($name, $help)
                        .expect("register_int_counter should succeed")
                });
            ::std::thread_local! {
                $(#[$attr])*
                $vis static $var_name: ::std::cell::RefCell<$crate::thread_local::ThreadLocalIntCounter> =
                    ::std::cell::RefCell::new($crate::thread_local::ThreadLocalIntCounter::new(&[<__ $var_name>]));
            }
        }
    }
}

#[macro_export]
macro_rules! make_thread_local_int_counter_vec {
    (
        $(#[$attr:meta])*
        $vis:vis,
        $var_name:ident,
        $name:expr,
        $help:expr,
        $labels_names:expr $(,)?
    ) => {
        $crate::thread_local::__private::paste! {
            static [<__ $var_name>]: $crate::thread_local::__private::Lazy<$crate::IntCounterVec> =
                $crate::thread_local::__private::Lazy::new(|| {
                    $crate::register_int_counter_vec!($name, $help, $labels_names)
                        .expect("register_int_counter_vec should succeed")
                });
            ::std::thread_local! {
                $(#[$attr])*
                $vis static $var_name: ::std::cell::RefCell<$crate::thread_local::ThreadLocalIntCounterVec> =
                    ::std::cell::RefCell::new($crate::thread_local::ThreadLocalIntCounterVec::new(&[<__ $var_name>]));
            }
        }
    }
}

#[macro_export]
macro_rules! make_thread_local_histogram_vec {
    (
        $(#[$attr:meta])*
        $vis:vis,
        $var_name:ident,
        $name:expr,
        $help:expr,
        $labels_names:expr
        $(, $buckets:expr)? $(,)?
    ) => {
        $crate::thread_local::__private::paste! {
            static [<__ $var_name>]: $crate::thread_local::__private::Lazy<$crate::HistogramVec> =
                $crate::thread_local::__private::Lazy::new(|| {
                    $crate::register_histogram_vec!($name, $help, $labels_names $(, $buckets)?)
                        .expect("register_histogram_vec should succeed")
                });
            ::std::thread_local! {
                $(#[$attr])*
                $vis static $var_name: ::std::cell::RefCell<$crate::thread_local::ThreadLocalHistogramVec> =
                    ::std::cell::RefCell::new($crate::thread_local::ThreadLocalHistogramVec::new(&[<__ $var_name>]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{IntCounterHelper, IntCounterVecHelper, TimerHelper};

    make_thread_local_int_counter!(
        pub(self),
        TEST_INT_COUNTER,
        "aptos_test_int_counter",
        "this is a help message",
    );
    make_thread_local_int_counter_vec!(
        pub(self),
        TEST_INT_COUNTER_VEC,
        "aptos_test_int_counter_vec",
        "this is a help message",
        &["label"],
    );
    make_thread_local_histogram_vec!(
        pub(self),
        TEST_HISTOGRAM_VEC,
        "aptos_test_histogram_vec",
        "this is a help message",
        &["label"],
    );

    #[test]
    fn test_thread_local_int_counter() {
        TEST_INT_COUNTER.inc();
        TEST_INT_COUNTER.inc_by(2);
    }

    #[test]
    fn test_thread_local_int_counter_vec() {
        TEST_INT_COUNTER_VEC.inc_with(&["foo"]);
    }

    #[test]
    fn test_thread_local_histogram_vec() {
        let _timer = TEST_HISTOGRAM_VEC.timer_with(&["foo"]);
        TEST_HISTOGRAM_VEC.observe_with(&["bar"], 1.0);
    }
}
