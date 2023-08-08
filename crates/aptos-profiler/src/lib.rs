// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs,
    path::{Path, PathBuf},
};
use serde::{Deserialize, Serialize};
use serde_yaml::{self};

use anyhow::Result;
use crate::thread_profiler::ThreadProfiler;
use crate::cpu_profiler::CpuProfiler;
use crate::offcpu_profiler::OffCpuProfiler;
use crate::memory_profiler::MemProfiler;

mod cpu_profiler;
mod offcpu_profiler;
mod memory_profiler;
mod thread_profiler;
mod utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilerConfig {
    cpu_profiler_config: Option<CpuProfilerConfig>,
    offcpu_profiler_config: Option<OffCpuProfilerConfig>,
    mem_profiler_config: Option<MemProfilerConfig>,
    thread_profiler_config: Option<ThreadProfilerConfig>,
}

impl ProfilerConfig {
   // pub fn load_from_file(path: &Path) -> Result<Self> {
    //    let path_str = path.to_str().unwrap_or_default();
      //  let mut file = tokio::fs::File::open(path);
       // let mut content = Vec::new();
        //file.read_to_end(&mut content);

        //Ok(serde_yaml::from_slice(&content)?)
    //}
    
    pub fn new_with_defaults() -> Self {
        Self {
            cpu_profiler_config: CpuProfilerConfig::new_with_defaults(),
            mem_profiler_config:  MemProfilerConfig::new_with_defaults(),
            thread_profiler_config: ThreadProfilerConfig::new_with_defaults(),
            offcpu_profiler_config: OffCpuProfilerConfig::new_with_defaults(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CpuProfilerConfig {
    duration: u64,
    frequency: i32,
    cpu_profiling_result: PathBuf,
}

impl CpuProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            duration: 100,
            frequency: 100,
            cpu_profiling_result: PathBuf::from("./profiling_results/cpu_flamegraph.svg"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemProfilerConfig {
    duration: u64,
    mem_profiling_result_txt: PathBuf,
    mem_profiling_result_svg: PathBuf,
} 

impl MemProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            duration: 60,
            mem_profiling_result_txt: PathBuf::from("./profiling_results/heap.txt"),
            mem_profiling_result_svg: PathBuf::from("./profiling_results/heap.svg"),

        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThreadProfilerConfig {
    thread_profiling_result: PathBuf,
}

impl ThreadProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            thread_profiling_result: PathBuf::from("./profiling_results/thead_dump.txt"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OffCpuProfilerConfig {
    count: u64,
    offcpu_profiling_txt_output: PathBuf,
    offcpu_profiling_svg_output: PathBuf,

}

impl OffCpuProfilerConfig {
    pub fn new_with_defaults() -> Option<Self> {
        Some(Self {
            count: 30,
            offcpu_profiling_txt_output: PathBuf::from("./profiling_results/offcpu.txt"),
            offcpu_profiling_svg_output: PathBuf::from("./profiling_results/offcpu.svg"),
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

    pub fn get_offcpu_profiler(&self) -> Box<dyn Profiler> {
        Box::new(OffCpuProfiler::new(self.config.offcpu_profiler_config.as_ref().expect("Off CPU profiler config is not set")))
    }

    pub fn get_cpu_profiler(&self) -> Box<dyn Profiler> {
        Box::new(CpuProfiler::new(self.config.cpu_profiler_config.as_ref().expect("CPU profiler config is not set")))
    }

    pub fn get_mem_profiler(&self) -> Box<dyn Profiler> {
        Box::new(MemProfiler::new(self.config.mem_profiler_config.as_ref().expect("Memory profiler config is not set")))
    }
}