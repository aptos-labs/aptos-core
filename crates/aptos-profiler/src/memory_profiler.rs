// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use crate::{MemProfilerConfig, Profiler, utils::convert_svg_to_string};
use std::{thread, fs, path::{Path, PathBuf}, time::Duration, process::Command};

pub struct MemProfiler {
    duration: u64,
    memory_profiling_data_file: PathBuf,
}

impl MemProfiler {
    pub(crate) fn new(config: &MemProfilerConfig) -> Self {
        Self {
            duration: config.duration,
            memory_profiling_data_file: config.mem_profiling_data_files_dir.clone(),
        
        }
    }
}

impl Profiler for MemProfiler {
    fn start_profiling(&self) -> Result<()> {
        unsafe {
            let mut prof_active: bool = true;
    
            let result = jemalloc_sys::mallctl(
                b"prof.active\0".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            );
    
            println!("{}", result);
            if result != 0 {
                panic!("Failed to activate jemalloc profiling");
            }
            let duration = self.duration;
            let handle = thread::spawn(move || {
                thread::sleep(Duration::from_secs(duration));
    
                // Disable the profiling
                let mut prof_active: bool = false;
                let result = jemalloc_sys::mallctl(
                    b"prof.active\0".as_ptr() as *const _,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut prof_active as *mut _ as *mut _,
                    std::mem::size_of::<bool>(),
                );
    
                println!("{}", result);
                if result != 0 {
                    panic!("Failed to deactivate jemalloc profiling");
                }
            });
    
            handle.join().unwrap();
        }
        let output = Command::new("python3")
            .arg("./crates/aptos=-profiler/src/jeprof.py")
            .output()
            .expect("Failed to execute command");
    
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("Command executed successfully. Output:\n{}", stdout);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Command failed. Error:\n{}", stderr);
        }
        Ok(())
    }

    fn end_profiling(&self) -> Result<()> {
        unimplemented!()
    }

    // End profiling
    fn expose_text_results(&self) -> Result<String> {
        let content = fs::read_to_string(self.memory_profiling_data_file.join("/heap.txt").as_path())
        .expect("Failed to read input");
        return Ok(content);
    }
    // Expose the results as a JSON string for visualization
    fn expose_svg_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.memory_profiling_data_file.join("/heap.svg").as_path())
        .expect("Failed to read input");
        return Ok(content);
    }
}