// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use crate::{Profiler, ThreadProfilerConfig};
use crate::utils::{convert_svg_to_string, create_file_with_parents};
use std::{process::Command, fs::File, io::Write, path::PathBuf};

//TODO: add other config fields
pub struct ThreadProfiler {
    thread_profiling_data_file: PathBuf,
}

impl ThreadProfiler {
    pub(crate) fn new(config: &ThreadProfilerConfig) -> Self {
        Self {
            thread_profiling_data_file: config.thread_profiling_data_files_dir.clone(),
        }
    }
}

impl Profiler for ThreadProfiler {
    fn start_profiling(&self) -> Result<()> {
        let trace = rstack_self::trace(
            Command::new("cargo")
                .arg("run")
                .arg("-p")
                .arg("aptos-profiler")
                .arg("--release"),
        )
            .unwrap();
        
        // Open a file for writing
        ///***let mut file = File::create(self.thread_profiling_data_file.as_path()).unwrap();***
        let mut file = create_file_with_parents(self.thread_profiling_data_file.as_path())?;

        // Write the trace information to the file
        write!(file, "{:#?}", trace).unwrap();
        Ok(())
    }

    fn end_profiling(&self) -> Result<()> {
        unimplemented!()
    }
    fn expose_svg_results(&self) -> Result<String> {
        unimplemented!()
    }
    fn expose_text_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.thread_profiling_data_file.join("/thread_dump.txt").as_path());
        return Ok(content.unwrap());
    }
}
