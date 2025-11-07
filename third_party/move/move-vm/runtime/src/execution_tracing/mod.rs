// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines trace types and ability to replay traces after execution.

mod recorders;
pub use recorders::{FullTraceRecorder, NoOpTraceRecorder, TraceRecorder};

mod trace;
pub use trace::Trace;
pub(crate) use trace::TraceCursor;
