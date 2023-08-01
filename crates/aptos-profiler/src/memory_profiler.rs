// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use crate::{MemProfilerConfig, Profiler, utils::convert_svg_to_string};
use std::{thread, fs, path::{Path, PathBuf}, time::Duration, process::Command};
use std::ffi::CString;

pub struct MemProfiler {
    duration: u64,
    memory_profiling_result_txt: PathBuf,
    memory_profiling_result_svg: PathBuf,
}

impl MemProfiler {
    pub(crate) fn new(config: &MemProfilerConfig) -> Self {
        Self {
            duration: config.duration,
            memory_profiling_result_txt: config.mem_profiling_result_txt.clone(),
            memory_profiling_result_svg: config.mem_profiling_result_svg.clone(),
        }
    }
}
impl Profiler for MemProfiler {
    
    fn start_profiling(&self) -> Result<()> {
        let mut prof_active: bool = true;

        let result = unsafe {
            jemalloc_sys::mallctl(
                b"prof.active\0".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            )
        };
    
        if result != 0 {
            return Err(anyhow!("Failed to activate jemalloc profiling"));
        }

    
        let duration = self.duration; // Replace with your desired duration
        thread::sleep(Duration::from_secs(duration));
    
        // Disable the profiling
        let mut prof_active: bool = false;
        let result = unsafe {
            jemalloc_sys::mallctl(
                b"prof.active\0".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            )
        };
    
        if result != 0 {
            return Err(anyhow!("Failed to deactivate jemalloc profiling"));
        }
    
        let output = Command::new("python3")
            .arg("./crates/aptos-profiler/src/jeprof.py")
            .arg(&self.memory_profiling_result_txt.to_string_lossy().as_ref())
            .arg(&self.memory_profiling_result_svg.to_string_lossy().as_ref())
            .output()
            .expect("Failed to execute command");

        Ok(())
    }

    fn end_profiling(&self) -> Result<()> {
        let mut prof_active: bool = false;
        let result = unsafe {
            jemalloc_sys::mallctl(
                b"prof.active\0".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            )
        };
        
        if result != 0 {
            return Err(anyhow!("Failed to deactivate jemalloc profiling"));
        }
    
        let output = Command::new("python3")
            .arg("./crates/aptos-profiler/src/jeprof.py")
            .arg(&self.memory_profiling_result_txt.to_string_lossy().as_ref())
            .arg(&self.memory_profiling_result_svg.to_string_lossy().as_ref())
            .output()
            .expect("Failed to execute command");
        Ok(())
    }

    // End profiling
    fn expose_text_results(&self) -> Result<String> {
        let content = fs::read_to_string(self.memory_profiling_result_txt.as_path())
        .expect("Failed to read input");
        return Ok(content);
    }
    // Expose the results as a JSON string for visualization
    fn expose_svg_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.memory_profiling_result_svg.as_path())
        .expect("Failed to read input");
        return Ok(content);
    }
}