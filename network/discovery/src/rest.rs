// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::validator_set::extract_validator_set_updates;
use crate::DiscoveryError;
use aptos_config::config::PeerSet;
use aptos_config::network_id::NetworkContext;
use aptos_logger::info;
use aptos_time_service::{Interval, TimeService, TimeServiceTrait};
use aptos_types::account_address::AccountAddress;
use aptos_types::on_chain_config::ValidatorSet;
use futures::executor::block_on;
use futures::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

/// A discovery stream that uses the REST client to determine the validator
/// set nodes.  Useful for when genesis is significantly far behind in time
pub struct RestStream {
    network_context: NetworkContext,
    rest_client: aptos_rest_client::Client,
    initialized: bool,
    interval: Pin<Box<Interval>>,
}

impl RestStream {
    pub(crate) fn new(
        network_context: NetworkContext,
        rest_url: url::Url,
        interval_duration: Duration,
        time_service: TimeService,
    ) -> Self {
        // Ensure that this isn't spamming the full node
        if interval_duration < Duration::from_secs(60) {
            panic!("Must set a Rest interval duration greater than 60 seconds")
        }

        RestStream {
            network_context,
            rest_client: aptos_rest_client::Client::new(rest_url),
            initialized: false,
            interval: Box::pin(time_service.interval(interval_duration)),
        }
    }
}

impl Stream for RestStream {
    type Item = Result<PeerSet, DiscoveryError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Wait for delay, or add the delay for next call
        // When it's initialized, we want to wait 10x the time period because it's not as important
        // to update, since it already has a relatively up to date picture
        if self.initialized {
            for _ in 0..10 {
                futures::ready!(self.interval.as_mut().poll_next(cx));
            }
        } else {
            futures::ready!(self.interval.as_mut().poll_next(cx));
        }

        // Retrieve the onchain resource at the interval
        // TODO there should be a better way than converting this to a blocking call
        let response = block_on(self.rest_client.get_account_resource_bcs::<ValidatorSet>(
            AccountAddress::ONE,
            "0x1::stake::ValidatorSet",
        ));
        Poll::Ready(match response {
            Ok(inner) => {
                let validator_set = inner.into_inner();
                let peer_set = extract_validator_set_updates(self.network_context, validator_set);

                if !peer_set.is_empty() {
                    self.initialized = true;
                }
                Some(Ok(peer_set))
            }
            Err(err) => {
                info!(
                    "Failed to retrieve validator set by REST discovery {:?}",
                    err
                );
                Some(Err(DiscoveryError::Rest(err)))
            }
        })
    }
}
