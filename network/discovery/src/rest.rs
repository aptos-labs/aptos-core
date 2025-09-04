// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{validator_set::extract_validator_set_updates, DiscoveryError};
use velor_config::{config::PeerSet, network_id::NetworkContext};
use velor_logger::info;
use velor_time_service::{Interval, TimeService, TimeServiceTrait};
use velor_types::{account_address::AccountAddress, on_chain_config::ValidatorSet};
use futures::{executor::block_on, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

/// A discovery stream that uses the REST client to determine the validator
/// set nodes.  Useful for when genesis is significantly far behind in time
pub struct RestStream {
    network_context: NetworkContext,
    rest_client: velor_rest_client::Client,
    interval: Pin<Box<Interval>>,
}

impl RestStream {
    pub(crate) fn new(
        network_context: NetworkContext,
        rest_url: url::Url,
        interval_duration: Duration,
        time_service: TimeService,
    ) -> Self {
        RestStream {
            network_context,
            rest_client: velor_rest_client::Client::new(rest_url),
            interval: Box::pin(time_service.interval(interval_duration)),
        }
    }
}

impl Stream for RestStream {
    type Item = Result<PeerSet, DiscoveryError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Wait for delay, or add the delay for next call
        futures::ready!(self.interval.as_mut().poll_next(cx));

        // Retrieve the onchain resource at the interval
        // TODO there should be a better way than converting this to a blocking call
        let response = block_on(self.rest_client.get_account_resource_bcs::<ValidatorSet>(
            AccountAddress::ONE,
            "0x1::stake::ValidatorSet",
        ));
        Poll::Ready(match response {
            Ok(inner) => {
                let validator_set = inner.into_inner();
                Some(Ok(extract_validator_set_updates(
                    self.network_context,
                    validator_set,
                )))
            },
            Err(err) => {
                info!(
                    "Failed to retrieve validator set by REST discovery {:?}",
                    err
                );
                Some(Err(DiscoveryError::Rest(err)))
            },
        })
    }
}
