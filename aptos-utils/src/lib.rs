// Copyright © Aptos Foundation

/// An internal implementation to imitate the feature of `try` in unstable Rust.
/// Useful to use '?' chaining on option/result without the need to wrap the expression in a
/// function.
#[macro_export]
macro_rules! aptos_try {
    ($e:expr) => {
        (|| $e)()
    };
}
