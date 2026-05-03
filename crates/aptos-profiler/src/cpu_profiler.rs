// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    utils::{convert_svg_to_string, create_file_with_parents},
    CpuProfilerConfig, Profiler,
};
use anyhow::Result;
use pprof::{ProfilerGuard, Report};
use regex::Regex;
use std::{io::Write, path::PathBuf, thread, time};

pub struct CpuProfiler<'a> {
    frequency: i32,
    txt_result_path: PathBuf,
    svg_result_path: PathBuf,
    guard: Option<ProfilerGuard<'a>>,
}

impl<'a> CpuProfiler<'a> {
    pub(crate) fn new(config: &CpuProfilerConfig) -> Self {
        Self {
            frequency: config.frequency,
            txt_result_path: config.txt_result_path.clone(),
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

    fn frames_post_processor() -> impl Fn(&mut pprof::Frames) {
        let regex = Regex::new(r"^(.*)-(\d*)$").unwrap();

        move |frames| {
            if let Some((_, [name, _])) = regex.captures(&frames.thread_name).map(|c| c.extract()) {
                frames.thread_name = name.to_string();
            }
        }
    }

    fn write_report_outputs(&self, report: &Report) -> Result<()> {
        let mut text_file = create_file_with_parents(self.txt_result_path.as_path())?;
        write!(text_file, "{:?}", report)?;

        let svg_file = create_file_with_parents(self.svg_result_path.as_path())?;
        let _result = report.flamegraph(svg_file);

        Ok(())
    }
}

impl Profiler for CpuProfiler<'static> {
    /// Perform CPU profiling for the given duration
    fn profile_for(&self, duration_secs: u64, _binary_path: &str) -> Result<()> {
        let guard = pprof::ProfilerGuard::new(self.frequency).unwrap();
        thread::sleep(time::Duration::from_secs(duration_secs));

        if let Ok(report) = guard.report().build() {
            self.write_report_outputs(&report)?;
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
                self.write_report_outputs(&report)?;
            }
            self.destory_guard()?;
        }
        Ok(())
    }

    /// Expose the results as TXT
    fn expose_text_results(&self) -> Result<String> {
        convert_svg_to_string(self.txt_result_path.as_path())
    }

    /// Expose the results as SVG
    fn expose_svg_results(&self) -> Result<String> {
        convert_svg_to_string(self.svg_result_path.as_path())
    }
}
