// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines trace types and ability to replay traces after execution.

mod loggers;
pub use loggers::{FullTraceLogger, NoOpTraceLogger, TraceLogger};

mod trace;
pub use trace::Trace;
