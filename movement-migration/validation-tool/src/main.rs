// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    validation_tool::ValidationTool::parse().run().await
}
