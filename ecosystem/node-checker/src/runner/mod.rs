// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod sync_runner;
mod traits;

pub use sync_runner::{SyncRunner, SyncRunnerConfig};
pub use traits::Runner;
