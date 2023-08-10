// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    utils::{convert_svg_to_string, create_file_with_parents},
    CpuProfilerConfig, Profiler,
};
use anyhow::Result;
use std::{path::PathBuf, thread, time};

pub struct CpuProfiler {
    frequency: i32,
    svg_result_path: PathBuf,
}

impl CpuProfiler {
    pub(crate) fn new(config: &CpuProfilerConfig) -> Self {
        Self {
            frequency: config.frequency,
            svg_result_path: config.svg_result_path.clone(),
        }
    }
}

impl Profiler for CpuProfiler {
    /// Perform CPU profiling for the given duration
    fn profile_for(&self, duration_secs: u64) -> Result<()> {
        let guard = pprof::ProfilerGuard::new(self.frequency).unwrap();
        thread::sleep(time::Duration::from_secs(duration_secs));

        if let Ok(report) = guard.report().build() {
            let file = create_file_with_parents(self.svg_result_path.as_path())?;
            let _result = report.flamegraph(file);
        };

        Ok(())
    }

    /// Start profiling until it is stopped
    fn start_profiling(&self) -> Result<()> {
        let _guard = pprof::ProfilerGuard::new(self.frequency).unwrap();
        let duration = u64::MAX;
        thread::sleep(time::Duration::from_secs(duration));

        Ok(())
    }

    /// End profiling
    fn end_profiling(&self) -> Result<()> {
        //TODO: pprof-rs crate may not have a direct way of stopping the profiling from another function.
        //Potential approach: return guard object to original scope and pass it here to stop and report results
        todo!();
    }

    /// Expose the results as TXT
    fn expose_text_results(&self) -> Result<String> {
        unimplemented!();
    }

    /// Expose the results as SVG
    fn expose_svg_results(&self) -> Result<String> {
        let content = convert_svg_to_string(self.svg_result_path.as_path());
        content
    }
}
