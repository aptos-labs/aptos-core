// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use crate::thread_profiler::ThreadProfiler;
use crate::cpu_profiler::CpuProfiler;
use crate::memory_profiler::MemProfiler;

mod cpu_profiler;
mod memory_profiler;
mod thread_profiler;
mod utils;

#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    cpu_profiler_config: Option<CpuProfilerConfig>,
    mem_profiler_config: Option<MemProfilerConfig>,
    thread_profiler_config: Option<ThreadProfilerConfig>,
}

impl ProfilerConfig {
    pub fn new_with_defaults() -> Self {
        Self {
            cpu_profiler_config: CpuProfilerConfig::new_with_defaults(),
            mem_profiler_config:  MemProfilerConfig::new_with_defaults(),
            thread_profiler_config: ThreadProfilerConfig::new_with_defaults(),
        }
    }
}

#[derive(Debug, Clone)]
struct CpuProfilerConfig {
    duration: u64,
    frequency: i32,
    cpu_profiling_result: PathBuf,
}

impl CpuProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            duration: 5,
            frequency: 100,
            cpu_profiling_result: PathBuf::from("./cpu_flamegraph.svg"),
        })
    }
}

#[derive(Debug, Clone)]
struct MemProfilerConfig {
    duration: u64,
    mem_profiling_result_txt: PathBuf,
    mem_profiling_result_svg: PathBuf,
} 

impl MemProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            duration: 60,
            mem_profiling_result_txt: PathBuf::from("./heap.txt"),
            mem_profiling_result_svg: PathBuf::from("./heap.svg"),

        })
    }
}

#[derive(Debug, Clone)]
struct ThreadProfilerConfig {
    profiling_thread_name: String,
    thread_profiling_data_files_dir: PathBuf,
}

impl ThreadProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            profiling_thread_name: "thread_profiling_thread".to_string(),
            thread_profiling_data_files_dir: PathBuf::from("./thread_profiling_result/thead_dump.txt"),
        })
    }
}

/// This defines the interface for caller to start profiling
pub trait Profiler {
    // Start profiling
    fn start_profiling(&self) -> Result<()>;
    // End profiling
    fn end_profiling(&self) -> Result<()>;
    // Expose the results as a JSON string for visualization
    fn expose_text_results(&self) -> Result<String>;
    // Expose the results as a JSON string for visualization
    fn expose_svg_results(&self) -> Result<String>;

    
}


pub struct ProfilerHandler {
    config: ProfilerConfig,
}

impl ProfilerHandler {

    pub fn new(config: ProfilerConfig) -> Self {
        //fs::create_dir("");
        Self {
            config
        }
    }
    

    pub fn get_thread_profiler(&self) -> Box<dyn Profiler> {
        Box::new(ThreadProfiler::new(self.config.thread_profiler_config.as_ref().expect("Thread profiler config is not set")))
    }

    pub fn get_cpu_profiler(&self) -> Box<dyn Profiler> {
        Box::new(CpuProfiler::new(self.config.cpu_profiler_config.as_ref().expect("CPU profiler config is not set")))
    }

    pub fn get_mem_profiler(&self) -> Box<dyn Profiler> {
        Box::new(MemProfiler::new(self.config.mem_profiler_config.as_ref().expect("Memory profiler config is not set")))
    }
}