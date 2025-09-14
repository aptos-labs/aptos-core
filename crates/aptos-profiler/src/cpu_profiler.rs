// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    utils::{convert_svg_to_string, create_file_with_parents},
    CpuProfilerConfig, Profiler,
};
use anyhow::Result;
use pprof::ProfilerGuard;
use regex::Regex;
use std::{path::PathBuf, thread, time};

pub struct CpuProfiler<'a> {
    frequency: i32,
    svg_result_path: PathBuf,
    guard: Option<ProfilerGuard<'a>>,
}

impl<'a> CpuProfiler<'a> {
    pub(crate) fn new(config: &CpuProfilerConfig) -> Self {
        Self {
            frequency: config.frequency,
            svg_result_path: config.svg_result_path.clone(),
            guard: None,
        }
    }

    pub(crate) fn set_guard(&mut self, guard: ProfilerGuard<'a>) -> Result<()> {
        self.guard = Some(guard);
        Ok(())
    }

    pub(crate) fn destory_guard(&mut self) -> Result<()> {
        self.guard = None;
        Ok(())
    }

    fn frames_post_processor() -> impl Fn(&mut pprof::Frames) + use<> {
        let regex = Regex::new(r"^(.*)-(\d*)$").unwrap();

        move |frames| {
            if let Some((_, [name, _])) = regex.captures(&frames.thread_name).map(|c| c.extract()) {
                frames.thread_name = name.to_string();
            }
        }
    }
}

impl Profiler for CpuProfiler<'static> {
    /// Perform CPU profiling for the given duration
    fn profile_for(&self, duration_secs: u64, _binary_path: &str) -> Result<()> {
        let guard = pprof::ProfilerGuard::new(self.frequency).unwrap();
        thread::sleep(time::Duration::from_secs(duration_secs));

        if let Ok(report) = guard.report().build() {
            let file = create_file_with_parents(self.svg_result_path.as_path())?;
            let _result = report.flamegraph(file);
        };

        Ok(())
    }

    /// Start profiling until it is stopped
    fn start_profiling(&mut self) -> Result<()> {
        let guard = pprof::ProfilerGuard::new(self.frequency).unwrap();
        self.set_guard(guard)?;
        Ok(())
    }

    /// End profiling
    fn end_profiling(&mut self, _binary_path: &str) -> Result<()> {
        if let Some(guard) = self.guard.take() {
            if let Ok(report) = guard
                .report()
                .frames_post_processor(Self::frames_post_processor())
                .build()
            {
                let file = create_file_with_parents(self.svg_result_path.as_path())?;
                let _result = report.flamegraph(file);
            }
            self.destory_guard()?;
        }
        Ok(())
    }

    /// Expose the results as TXT
    fn expose_text_results(&self) -> Result<String> {
        unimplemented!();
    }

    /// Expose the results as SVG
    fn expose_svg_results(&self) -> Result<String> {
        convert_svg_to_string(self.svg_result_path.as_path())
    }
}
