// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use crate::{Profiler, ThreadProfilerConfig};
use crate::utils::{convert_svg_to_string, create_file_with_parents};
use std::{process::Command, io::Write, path::PathBuf};

pub struct ThreadProfiler {
    txt_result_path: PathBuf,
}

impl ThreadProfiler {
    pub(crate) fn new(config: &ThreadProfilerConfig) -> Self {
        Self {
            txt_result_path: config.txt_result_path.clone(),
        }
    }
}

impl Profiler for ThreadProfiler {
    fn profile_for(&self, _duration_secs: u64, _binary_path: &str) -> Result<()> {
        unimplemented!()
    }

    fn start_profiling(&mut self) -> Result<()> {
        let trace = rstack_self::trace(
            Command::new("cargo")
                .arg("run")
                .arg("-p")
                .arg("aptos-profiler")
                .arg("--release"),
        )
            .unwrap();
        
        let mut file = create_file_with_parents(self.txt_result_path.as_path())?;

        // Write the trace information to the file
        write!(file, "{:#?}", trace).unwrap();
        Ok(())
    }
    fn end_profiling(&mut self, _binary_path: &str) -> Result<()> {
        unimplemented!()
    }
    fn expose_svg_results(&self) -> Result<String> {
        unimplemented!()
    }
    fn expose_text_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.txt_result_path.as_path());
        return Ok(content.unwrap());
    }
}
