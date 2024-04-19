// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// An internal implementation to imitate the feature of `try` in unstable Rust.
/// Useful to use '?' chaining on option/result without the need to wrap the expression in a
/// function.
/// Obsolete once rust-lang/rust#31436 is resolved and try is stable.
#[macro_export]
macro_rules! aptos_try {
    ($e:expr) => {
        (|| $e)()
    };
}

/// When the expression is an error, return from the enclosing function.
/// Use this in cases where the enclosing function returns `Result<(), Error>`.
/// Writing 'f()?;' relies on a load-bearing '?' operator. If the operator is removed execution
/// would continue on error. This macro is a more explicit way to return on error, ie.
/// 'return_on_failure!(f());'
#[macro_export]
macro_rules! return_on_failure {
    ($e:expr) => {
        $e?;
    };
}
