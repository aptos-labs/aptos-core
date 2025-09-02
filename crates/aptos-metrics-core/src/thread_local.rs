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

const FLUSH_INTERVAL: Duration = Duration::from_millis(100);

pub struct LocalIntCounter {
    inner: prometheus::local::LocalIntCounter,
    last_flush: Instant,
}

impl LocalIntCounter {
    pub fn new(shared: &prometheus::IntCounter) -> Self {
        Self {
            inner: shared.local(),
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_flush) > FLUSH_INTERVAL {
            self.inner.flush();
        }
        self.last_flush = now;
    }
}

impl IntCounterHelper for LocalKey<RefCell<LocalIntCounter>> {
    type IntType = u64;

    fn get(&'static self) -> Self::IntType {
        self.with_borrow(|x| x.inner.get())
    }

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

pub struct LocalIntCounterVec {
    inner: prometheus::local::LocalIntCounterVec,
    last_flush: Instant,
}

impl LocalIntCounterVec {
    pub fn new(shared: &prometheus::IntCounterVec) -> Self {
        Self {
            inner: shared.local(),
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_flush) > FLUSH_INTERVAL {
            self.inner.flush();
        }
        self.last_flush = now;
    }
}

impl IntCounterVecHelper for LocalKey<RefCell<LocalIntCounterVec>> {
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

pub struct LocalHistogramVec {
    inner: prometheus::local::LocalHistogramVec,
    last_flush: Instant,
}

impl LocalHistogramVec {
    pub fn new(shared: &prometheus::HistogramVec) -> Self {
        Self {
            inner: shared.local(),
            last_flush: Instant::now(),
        }
    }

    fn maybe_flush(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_flush) > FLUSH_INTERVAL {
            self.inner.flush();
        }
        self.last_flush = now;
    }
}

impl TimerHelper for LocalKey<RefCell<LocalHistogramVec>> {
    type TimerType = prometheus::local::LocalHistogramTimer;

    fn timer_with(&'static self, labels: &[&str]) -> Self::TimerType {
        self.with_borrow_mut(|x| {
            x.maybe_flush();
            x.inner.with_label_values(labels).start_timer()
        })
    }

    fn observe_with(&'static self, labels: &[&str], val: f64) {
        self.with_borrow_mut(|x| {
            x.maybe_flush();
            x.inner.with_label_values(labels).observe(val);
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
        $crate::__private::paste! {
            static [<__ $var_name>]: $crate::__private::Lazy<$crate::IntCounter> =
                $crate::__private::Lazy::new(|| {
                    $crate::register_int_counter!($name, $help).expect("foo")
                });
            ::std::thread_local! {
                $(#[$attr])*
                $vis static $var_name: ::std::cell::RefCell<$crate::LocalIntCounter> =
                    ::std::cell::RefCell::new($crate::LocalIntCounter::new(&[<__ $var_name>]));
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
        $crate::__private::paste! {
            static [<__ $var_name>]: $crate::__private::Lazy<$crate::IntCounterVec> =
                $crate::__private::Lazy::new(|| {
                    $crate::register_int_counter_vec!($name, $help, $labels_names).expect("foo")
                });
            ::std::thread_local! {
                $(#[$attr])*
                $vis static $var_name: ::std::cell::RefCell<$crate::LocalIntCounterVec> =
                    ::std::cell::RefCell::new($crate::LocalIntCounterVec::new(&[<__ $var_name>]));
            }
        }
    }
}

#[macro_export]
macro_rules! make_thread_local_histgram {
    (
        $(#[$attr:meta])*
        $vis:vis,
        $var_name:ident,
        $name:expr,
        $help:expr
        $(, $buckets:expr)? $(,)?
    ) => {
        $crate::__private::paste! {
            static [<__ $var_name>]: $crate::__private::Lazy<$crate::Histogram> =
                $crate::__private::Lazy::new(|| {
                    $crate::register_histogram!($name, $help, $(, $buckets)?).expect("foo")
                });
            ::std::thread_local! {
                $(#[$attr])*
                $vis static $var_name: ::std::cell::RefCell<$crate::LocalHistogram> =
                    ::std::cell::RefCell::new($crate::LocalHistogram::new(&[<__ $var_name>]));
            }
        }
    };
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
        $crate::__private::paste! {
            static [<__ $var_name>]: $crate::__private::Lazy<$crate::HistogramVec> =
                $crate::__private::Lazy::new(|| {
                    $crate::register_histogram_vec!($name, $help, $labels_names $(, $buckets)?).expect("foo")
                });
            ::std::thread_local! {
                $(#[$attr])*
                $vis static $var_name: ::std::cell::RefCell<$crate::LocalHistogramVec> =
                    ::std::cell::RefCell::new($crate::LocalHistogramVec::new(&[<__ $var_name>]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{IntCounterHelper, IntCounterVecHelper};

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

    #[test]
    fn test_thread_local_int_counter() {
        TEST_INT_COUNTER.inc();
        assert_eq!(TEST_INT_COUNTER.get(), 1);
        TEST_INT_COUNTER.inc_by(2);
        assert_eq!(TEST_INT_COUNTER.get(), 3);
    }

    // #[test]
    // fn test_thread_local_int_counter_vec() {
    //     TEST_INT_COUNTER_VEC.inc_with(&["label"]);
    //     assert_eq!(TEST_INT_COUNTER_VEC.
    // }
}
