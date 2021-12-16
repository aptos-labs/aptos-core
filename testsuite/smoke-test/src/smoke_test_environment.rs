// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use forge::{LocalFactory, LocalSwarm};
use once_cell::sync::Lazy;
use rand::rngs::OsRng;
use std::num::NonZeroUsize;

pub async fn new_local_swarm(num_validators: usize) -> LocalSwarm {
    static FACTORY: Lazy<LocalFactory> = Lazy::new(|| LocalFactory::from_workspace().unwrap());

    ::diem_logger::Logger::new().init();

    FACTORY
        .new_swarm(OsRng, NonZeroUsize::new(num_validators).unwrap())
        .await
        .unwrap()
}
