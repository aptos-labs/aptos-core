#![forbid(unsafe_code)] // Copyright (c) Aptos Foundation
                        // Copyright (c) Aptos Foundation
                        // SPDX-License-Identifier: Innovation-Enabling Source Code License

// SPDX-License-Identifier: Innovation-Enabling Source Code License

mod config;
mod gather_metrics;
mod server;

pub use config::MetricsServerConfig;
pub use server::run_metrics_server;
