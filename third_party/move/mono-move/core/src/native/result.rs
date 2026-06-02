// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Terminal outcome of a native function invocation.
#[derive(Debug, Clone)]
pub enum NativeStatus {
    Success,
    Abort { code: u64, message: Option<String> },
}

/// Error originating from VM-internal mechanisms invoked by a native.
///
/// Intended ONLY for errors that should just be propagated back to the VM runtime
/// rather than being inspected by the native functions themselves.
#[derive(Debug, Clone)]
pub enum VMInternalError {
    InvariantViolation(String),
    // TODO: Gas Metering
}
