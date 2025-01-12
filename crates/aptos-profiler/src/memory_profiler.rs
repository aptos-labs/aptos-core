// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{utils::convert_svg_to_string, MemProfilerConfig, Profiler};
use anyhow::{anyhow, Result};
use std::{path::PathBuf, process::Command, thread, time::Duration};

pub struct MemProfiler {
    txt_result_path: PathBuf,
    svg_result_path: PathBuf,
}

impl MemProfiler {
    pub(crate) fn new(config: &MemProfilerConfig) -> Self {
        Self {
            txt_result_path: config.txt_result_path.clone(),
            svg_result_path: config.svg_result_path.clone(),
        }
    }
}

impl Profiler for MemProfiler {
    fn profile_for(&self, duration_secs: u64, binary_path: &str) -> Result<()> {
        let mut prof_active: bool = true;

        let result = unsafe {
            jemalloc_sys::mallctl(
                c"prof.active".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            )
        };

        if result != 0 {
            return Err(anyhow!("Failed to activate jemalloc profiling"));
        }

        thread::sleep(Duration::from_secs(duration_secs));

        let mut prof_active: bool = false;
        let result = unsafe {
            jemalloc_sys::mallctl(
                c"prof.active".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            )
        };

        if result != 0 {
            return Err(anyhow!("Failed to deactivate jemalloc profiling"));
        }

        // TODO: Run jeprof commands from within Rust, current tries give unresolved errors
        Command::new("python3")
            .arg("./crates/aptos-profiler/src/jeprof.py")
            .arg(self.txt_result_path.to_string_lossy().as_ref())
            .arg(self.svg_result_path.to_string_lossy().as_ref())
            .arg(binary_path)
            .output()
            .expect("Failed to execute command");

        Ok(())
    }

    /// Enable memory profiling until it is disabled
    fn start_profiling(&mut self) -> Result<()> {
        let mut prof_active: bool = true;

        let result = unsafe {
            jemalloc_sys::mallctl(
                c"prof.active".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            )
        };

        if result != 0 {
            return Err(anyhow!("Failed to activate jemalloc profiling"));
        }

        Ok(())
    }

    /// Disable profiling and run jeprof to obtain results
    fn end_profiling(&mut self, binary_path: &str) -> Result<()> {
        let mut prof_active: bool = false;
        let result = unsafe {
            jemalloc_sys::mallctl(
                c"prof.active".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            )
        };

        if result != 0 {
            return Err(anyhow!("Failed to deactivate jemalloc profiling"));
        }

        // TODO: Run jeprof commands from within Rust, current tries give unresolved errors
        Command::new("python3")
            .arg("./crates/aptos-profiler/src/jeprof.py")
            .arg(self.txt_result_path.to_string_lossy().as_ref())
            .arg(self.svg_result_path.to_string_lossy().as_ref())
            .arg(binary_path)
            .output()
            .expect("Failed to execute command");

        Ok(())
    }

    /// Expose the results in TXT format
    fn expose_text_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.txt_result_path.as_path());
        content
    }

    /// Expose the results in SVG format
    fn expose_svg_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.svg_result_path.as_path());
        content
    }
}
