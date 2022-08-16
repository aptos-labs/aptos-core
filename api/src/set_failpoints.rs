// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;
#[allow(unused_imports)]
use anyhow::{format_err, Result};
#[cfg(feature = "failpoints")]
use aptos_logger::prelude::*;
use poem::{
    handler,
    web::{Data, Query},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct FailpointConf {
    name: String,
    actions: String,
}

#[cfg(feature = "failpoints")]
#[handler]
pub fn set_failpoint_poem(
    context: Data<&std::sync::Arc<Context>>,
    Query(failpoint_conf): Query<FailpointConf>,
) -> poem::Result<String> {
    if context.failpoints_enabled() {
        fail::cfg(&failpoint_conf.name, &failpoint_conf.actions)
            .map_err(|e| poem::Error::from(anyhow::anyhow!(e)))?;
        info!(
            "Configured failpoint {} to {}",
            failpoint_conf.name, failpoint_conf.actions
        );
        Ok(format!("Set failpoint {}", failpoint_conf.name))
    } else {
        Err(poem::Error::from(anyhow::anyhow!(
            "Failpoints are not enabled at a config level"
        )))
    }
}

#[allow(unused_variables)]
#[cfg(not(feature = "failpoints"))]
#[handler]
pub fn set_failpoint_poem(
    context: Data<&std::sync::Arc<Context>>,
    Query(failpoint_conf): Query<FailpointConf>,
) -> poem::Result<String> {
    Err(poem::Error::from(anyhow::anyhow!(
        "Failpoints are not enabled at a feature level"
    )))
}
