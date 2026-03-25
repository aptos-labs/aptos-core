// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines trace types and ability to replay traces after execution.

mod recorders;
pub use recorders::{FullTraceRecorder, NoOpTraceRecorder, TraceRecorder};

mod trace;
pub use trace::Trace;
pub(crate) use trace::TraceCursor;
