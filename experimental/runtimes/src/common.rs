// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_os = "linux")]
use libc::{cpu_set_t, sched_setaffinity, setpriority, PRIO_PROCESS};

#[cfg(target_os = "linux")]
pub(crate) fn new_cpu_set() -> cpu_set_t {
    unsafe { std::mem::zeroed::<cpu_set_t>() }
}

#[cfg(target_os = "linux")]
pub(crate) fn pin_cpu_set(cpu_set: cpu_set_t) -> impl Fn() + Send + Sync + 'static {
    move || {
        unsafe {
            sched_setaffinity(
                0, // Defaults to current thread
                std::mem::size_of::<cpu_set_t>(),
                &cpu_set,
            );
        };
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn set_thread_nice_value(nice_value: i32) -> impl Fn() + Send + Sync + 'static {
    move || unsafe {
        setpriority(PRIO_PROCESS, 0, nice_value);
    }
}
