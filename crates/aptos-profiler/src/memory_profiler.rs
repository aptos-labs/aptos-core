// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use crate::MemProfilerConfig;
use crate::Profiler;

pub struct MemProfiler {
}

impl MemProfiler {
    pub(crate) fn new(config: &MemProfilerConfig) -> Self {
        Self {
        }
    }
}

impl Profiler for MemProfiler {
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