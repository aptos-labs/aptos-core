// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{cpu_profiler::CpuProfiler, memory_profiler::MemProfiler, thread_profiler::ThreadProfiler, offcpu_profiler::OffCpuProfiler};
use anyhow::Result;
use std::path::PathBuf;

mod cpu_profiler;
mod memory_profiler;
mod thread_profiler;
mod offcpu_profiler;

mod utils;

#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    cpu_profiler_config: Option<CpuProfilerConfig>,
    mem_profiler_config: Option<MemProfilerConfig>,
    thread_profiler_config: Option<ThreadProfilerConfig>,
    offcpu_profiler_config: Option<OffCpuProfilerConfig>,
}

impl ProfilerConfig {
    pub fn new_with_defaults() -> Self {
        Self {
            cpu_profiler_config: CpuProfilerConfig::new_with_defaults(),
            mem_profiler_config: MemProfilerConfig::new_with_defaults(),
            thread_profiler_config: ThreadProfilerConfig::new_with_defaults(),
            offcpu_profiler_config: OffCpuProfilerConfig::new_with_defaults(),

        }
    }
}

#[derive(Debug, Clone)]
struct CpuProfilerConfig {
    frequency: i32,
    svg_result_path: PathBuf,
}

impl CpuProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            frequency: 100,
            svg_result_path: PathBuf::from("./profiling_results/cpu_flamegraph.svg"),
        })
    }
}

#[derive(Debug, Clone)]
struct MemProfilerConfig {
    txt_result_path: PathBuf,
    svg_result_path: PathBuf,
}

impl MemProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            txt_result_path: PathBuf::from("./profiling_results/heap.txt"),
            svg_result_path: PathBuf::from("./profiling_results/heap.svg"),
        })
    }
}

#[derive(Debug, Clone)]
struct ThreadProfilerConfig {
    txt_result_path: PathBuf,
}

impl ThreadProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            txt_result_path: PathBuf::from("./profiling_results/thread_dump.txt"),
        })
    }
}

#[derive(Debug, Clone)]
struct OffCpuProfilerConfig {
    svg_result_path: PathBuf,
    txt_result_path: PathBuf,
}

impl OffCpuProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            svg_result_path: PathBuf::from("./profiling_results/offcpu.svg"),
            txt_result_path: PathBuf::from("./profiling_results/collapsed.txt"),
        })
    }
}

/// This defines the interface for caller to start profiling
pub trait Profiler {
    // Perform profiling for duration_secs
    fn profile_for(&self, duration_secs: u64, binary_path: &str) -> Result<()>;
    // Start profiling
    fn start_profiling(&mut self) -> Result<()>;
    // End profiling
    fn end_profiling(&mut self, binary_path: &str) -> Result<()>;
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
        Self { config }
    }

    pub fn get_cpu_profiler(&self) -> Box<dyn Profiler> {
        Box::new(CpuProfiler::new(
            self.config
                .cpu_profiler_config
                .as_ref()
                .expect("CPU profiler config is not set"),
        ))
    }

    pub fn get_mem_profiler(&self) -> Box<dyn Profiler> {
        Box::new(MemProfiler::new(
            self.config
                .mem_profiler_config
                .as_ref()
                .expect("Memory profiler config is not set"),
        ))
    }

    pub fn get_thread_profiler(&self) -> Box<dyn Profiler> {
        Box::new(ThreadProfiler::new(
            self.config
                .thread_profiler_config
                .as_ref()
                .expect("Thread profiler config is not set"),
        ))
    }

    pub fn get_offcpu_profiler(&self) -> Box<dyn Profiler> {
        Box::new(OffCpuProfiler::new(
            self.config
                .offcpu_profiler_config
                .as_ref()
                .expect("Off-CPU profiler config is not set"),
        ))
    }
}
