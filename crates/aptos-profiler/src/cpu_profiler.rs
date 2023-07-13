// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use crate::{CpuProfilerConfig, Profiler};

pub struct CpuProfiler {
}

impl CpuProfiler {
    pub(crate) fn new(config: &CpuProfilerConfig) -> Self {
        Self {
        }
    }
}

impl Profiler for CpuProfiler {
    fn start_profiling(&self) -> Result<()> {
        unimplemented!()
    }
    // End profiling
    fn end_profiling(&self) -> Result<()> {
        unimplemented!()
    }
    // Expose the results as a JSON string for visualization
    fn expose_results(&self) -> Result<String> {
        unimplemented!()
    }
}