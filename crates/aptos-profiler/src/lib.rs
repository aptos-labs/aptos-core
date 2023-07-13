// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::{Path, PathBuf};

pub struct ProfilerConfig {
    cpu_profiler_config: Option<CpuProfilerConfig>,
    mem_profiler_config: Option<MemProfilerConfig>,
    thread_profiler_config: Option<ThreadProfilerConfig>,
}

struct CpuProfilerConfig {
    sleep_duration: u64,
    frequency: u64,
    cpu_profiling_data_files_dir: PathBuf,
}

struct MemProfilerConfig {
    sleep_duration: u64,
    frequency: u64,
    mem_profiling_data_files_dir: PathBuf,
}

struct ThreadProfilerConfig {
    profiling_thread_name: String,
    thread_profiling_data_files_dir: PathBuf,
}

/// This defines the interface for caller to start profiling
pub trait Profiler {
    // Start profiling
    fn start_profiling(&self);
    // End profiling
    fn end_profiling(&self);
    // Expose the results as a JSON string for visualization
    fn expose_results(&self) -> String;
}

pub fn convert_svg_to_string(svg_file_path: &Path) -> String {
    fs::read_to_string(svg_file_path).expect("Failed to read input")
}
