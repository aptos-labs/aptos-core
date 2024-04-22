// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_experimental_active_state_set::pipeline::{ExecutionMode, Pipeline, PipelineConfig};
use aptos_logger::info;
use aptos_push_metrics::MetricsPusher;
use std::env;
use tempfile::TempDir;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

pub fn main() {
    // set the default log level to debug
    aptos_logger::Logger::new().init();
    aptos_node_resource_metrics::register_node_metrics_collector();
    let _mp = MetricsPusher::start_for_local_run("active-set-benchmark");
    env::set_var("RUST_LOG", "info");
    let path: String = TempDir::new().unwrap().path().to_str().unwrap().to_string();
    info!("Pipeline data stored at {}", path);
    let config = PipelineConfig::new(1_000_000, 64_000_000, path, ExecutionMode::AST);
    let pipeline = Pipeline::new(config);
    pipeline.run();
}
