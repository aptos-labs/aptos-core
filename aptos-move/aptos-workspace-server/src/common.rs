// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use futures::{future::Shared, FutureExt};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

pub(crate) const IP_LOCAL_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Converts a future into a shared one by putting the error into an Arc.
pub(crate) fn make_shared<F, T, E>(fut: F) -> Shared<impl Future<Output = Result<T, Arc<E>>>>
where
    T: Clone,
    F: Future<Output = Result<T, E>>,
{
    fut.map(|r| r.map_err(|err| Arc::new(err))).shared()
}
