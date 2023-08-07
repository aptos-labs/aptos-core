// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
extern crate rstack_self;

use anyhow::Result;
use crate::{Profiler, OffCpuProfilerConfig};
use crate::utils::{convert_svg_to_string, create_file_with_parents};
use std::{process::Command, fs::File, io::Write, path::PathBuf};
use std::time::Duration;
use std::thread;
use std::process;
use std::fs::OpenOptions;


pub struct OffCpuProfiler {
    count: u64,
    offcpu_profiling_txt_output: PathBuf,
    offcpu_profiling_svg_output: PathBuf,
}

impl OffCpuProfiler {
    pub(crate) fn new(config: &OffCpuProfilerConfig) -> Self {
        Self {
            count: config.count,
            offcpu_profiling_txt_output: config.offcpu_profiling_txt_output.clone(),
            offcpu_profiling_svg_output: config.offcpu_profiling_svg_output.clone(),
        }
    }
}

impl Profiler for OffCpuProfiler {
    fn start_profiling(&self) -> Result<()> {
        let mut output = Vec::new();
    for _ in 0..self.count {
        let trace = rstack_self::trace(Command::new("cargo").arg("run").arg("-p").arg("aptos-profiler").arg("--release")).unwrap();
        for thread in trace.threads() {
            writeln!(output, "{} - {}", thread.id(), thread.name()).unwrap();
            for frame in thread.frames() {
                writeln!(output, "{:#016x}", frame.ip()).unwrap();
                for symbol in frame.symbols() {
                    write!(output, "    - {}", symbol.name().unwrap_or("????")).unwrap();
                    if let Some(file) = symbol.file() {
                        write!(output, " {}:{}", file.display(), symbol.line().unwrap_or(0)).unwrap();
                    }
                    writeln!(output).unwrap();
                }
            }
            writeln!(output).unwrap();
        }
        thread::sleep(Duration::from_secs(1));
    }

    // Write the thread information to a file
    let mut file = create_file_with_parents(self.offcpu_profiling_txt_output.as_path())?;


    file.write_all(&output).expect("Failed to write to the file");
        Ok(())
    }
    fn end_profiling(&self) -> Result<()> {
        unimplemented!()
    }
    fn expose_svg_results(&self) -> Result<String> {
        unimplemented!()
    }
    fn expose_text_results(&self) -> Result<String> {
        //let content = convert_svg_to_string(self.thread_profiling_data_file.join("/thread_dump.txt").as_path());
       // return Ok(content.unwrap());
       unimplemented!()
    }
}
