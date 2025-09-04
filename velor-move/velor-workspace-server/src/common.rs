// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Common utilities and constants for networking and asynchronous operations.

use futures::{future::Shared, FutureExt};
use std::{
    fmt::{self, Debug, Display},
    future::Future,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

/// Custom macro to allow writing to stdout while ignoring any errors.
/// This is to allow handling of closed stdout (e.g. in case of a broken pipe).
#[macro_export]
macro_rules! no_panic_println {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stdout(), $($arg)*);
    }};
}

/// Custom macro to allow writing to stderr while ignoring any errors.
/// This is to allow handling of closed stderr (e.g. in case of a broken pipe).
#[macro_export]
macro_rules! no_panic_eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

/// An wrapper to ensure propagation of chain of errors.
pub(crate) struct ArcError(Arc<anyhow::Error>);

impl Clone for ArcError {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl std::error::Error for ArcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl Display for ArcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for ArcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// The local IP address services are bound to.
pub(crate) const IP_LOCAL_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Converts a future into a shared future by wrapping the error in an `Arc`.
pub(crate) fn make_shared<F, T>(fut: F) -> Shared<impl Future<Output = Result<T, ArcError>>>
where
    T: Clone,
    F: Future<Output = Result<T, anyhow::Error>>,
{
    fut.map(|r| r.map_err(|err| ArcError(Arc::new(err))))
        .shared()
}
