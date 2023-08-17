// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
extern crate rstack_self;

use anyhow::Result;
use crate::{Profiler, OffCpuProfilerConfig};
use crate::utils::{convert_svg_to_string, create_file_with_parents};
use std::{process::Command, io::Write, path::PathBuf};
use std::time::Duration;
use std::thread;
use std::fs::OpenOptions;


pub struct OffCpuProfiler {
    svg_result_path: PathBuf,
    txt_result_path: PathBuf,
}

impl OffCpuProfiler {
    pub(crate) fn new(config: &OffCpuProfilerConfig) -> Self {
        Self {
            svg_result_path: config.svg_result_path.clone(),
            txt_result_path: config.txt_result_path.clone(),
        }
    }
}

impl Profiler for OffCpuProfiler {
    fn profile_for(&self, duration_secs: u64, binary_path: &str) -> Result<()> {
        unimplemented!()
    }

    fn start_profiling(&mut self) -> Result<()> {
        let mut output = Vec::new();
        for _ in 0..60 {
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
    let mut file = create_file_with_parents(self.txt_result_path.as_path())?;


    file.write_all(&output).expect("Failed to write to the file");
        Ok(())
    }
    fn end_profiling(&mut self, binary_path: &str) -> Result<()> {
        unimplemented!()
    }
    fn expose_svg_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.svg_result_path.as_path());
        content
    }
    fn expose_text_results(&self) -> Result<String> {
        //let content = convert_svg_to_string(self.thread_profiling_data_file.join("/thread_dump.txt").as_path());
       // return Ok(content.unwrap());
       unimplemented!()
    }

}