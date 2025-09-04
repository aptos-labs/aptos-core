// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::TIMER;
use velor_infallible::Mutex;
use velor_metrics_core::TimerHelper;
use once_cell::sync::OnceCell;
use rayon::ThreadPool;
use std::{ops::Deref, sync::mpsc::Receiver};

#[derive(Debug)]
pub struct Planned<T> {
    value: OnceCell<T>,
    rx: OnceCell<Mutex<Receiver<T>>>,
}

impl<T> Planned<T> {
    pub fn place_holder() -> Self {
        Self {
            value: OnceCell::new(),
            rx: OnceCell::new(),
        }
    }

    pub fn plan(&self, thread_pool: &ThreadPool, getter: impl FnOnce() -> T + Send + 'static)
    where
        T: Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();

        thread_pool.spawn(move || {
            tx.send(getter()).ok();
        });

        self.rx.set(Mutex::new(rx)).expect("Already planned.");
    }

    pub fn ready(t: T) -> Self {
        Self {
            value: OnceCell::with_value(t),
            rx: OnceCell::new(),
        }
    }

    pub fn get(&self, name_for_timer: Option<&str>) -> &T {
        if let Some(t) = self.value.get() {
            t
        } else {
            let _timer = name_for_timer.map(|name| TIMER.timer_with(&[name]));

            let rx = self.rx.get().expect("Not planned").lock();
            if self.value.get().is_none() {
                let t = rx.recv().expect("Plan failed.");
                self.value.set(t).map_err(|_| "").expect("Already set.");
            }
            self.value.get().expect("Must have been set.")
        }
    }
}

impl<T> Deref for Planned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get(None)
    }
}

pub trait Plan {
    fn plan<T: Send + 'static>(&self, getter: impl FnOnce() -> T + Send + 'static) -> Planned<T>;
}

impl Plan for ThreadPool {
    fn plan<T: Send + 'static>(&self, getter: impl FnOnce() -> T + Send + 'static) -> Planned<T> {
        let planned = Planned::<T>::place_holder();
        planned.plan(self, getter);
        planned
    }
}
