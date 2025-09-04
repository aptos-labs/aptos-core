// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::DISCOVERY_COUNTS, file::FileStream, rest::RestStream,
    validator_set::ValidatorSetStream,
};
use velor_config::{config::PeerSet, network_id::NetworkContext};
use velor_crypto::x25519;
use velor_event_notifications::ReconfigNotificationListener;
use velor_logger::prelude::*;
use velor_network::{
    connectivity_manager::{ConnectivityRequest, DiscoverySource},
    counters::inc_by_with_context,
    logging::NetworkSchema,
};
use velor_time_service::TimeService;
use velor_types::on_chain_config::OnChainConfigProvider;
use futures::{Stream, StreamExt};
use std::{
    path::Path,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::runtime::Handle;

mod counters;
mod file;
mod rest;
mod validator_set;

#[derive(Debug)]
pub enum DiscoveryError {
    IO(std::io::Error),
    Parsing(String),
    Rest(velor_rest_client::error::RestError),
}

/// A union type for all implementations of `DiscoveryChangeListenerTrait`
pub struct DiscoveryChangeListener<P: OnChainConfigProvider> {
    discovery_source: DiscoverySource,
    network_context: NetworkContext,
    update_channel: velor_channels::Sender<ConnectivityRequest>,
    source_stream: DiscoveryChangeStream<P>,
}

enum DiscoveryChangeStream<P: OnChainConfigProvider> {
    ValidatorSet(ValidatorSetStream<P>),
    File(FileStream),
    Rest(RestStream),
}

impl<P: OnChainConfigProvider> Stream for DiscoveryChangeStream<P> {
    type Item = Result<PeerSet, DiscoveryError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            Self::ValidatorSet(stream) => Pin::new(stream).poll_next(cx),
            Self::File(stream) => Pin::new(stream).poll_next(cx),
            Self::Rest(stream) => Pin::new(stream).poll_next(cx),
        }
    }
}

impl<P: OnChainConfigProvider> DiscoveryChangeListener<P> {
    pub fn validator_set(
        network_context: NetworkContext,
        update_channel: velor_channels::Sender<ConnectivityRequest>,
        expected_pubkey: x25519::PublicKey,
        reconfig_events: ReconfigNotificationListener<P>,
    ) -> Self {
        let source_stream = DiscoveryChangeStream::ValidatorSet(ValidatorSetStream::new(
            network_context,
            expected_pubkey,
            reconfig_events,
        ));
        DiscoveryChangeListener {
            discovery_source: DiscoverySource::OnChainValidatorSet,
            network_context,
            update_channel,
            source_stream,
        }
    }

    pub fn file(
        network_context: NetworkContext,
        update_channel: velor_channels::Sender<ConnectivityRequest>,
        file_path: &Path,
        interval_duration: Duration,
        time_service: TimeService,
    ) -> Self {
        let source_stream = DiscoveryChangeStream::File(FileStream::new(
            file_path,
            interval_duration,
            time_service,
        ));
        DiscoveryChangeListener {
            discovery_source: DiscoverySource::File,
            network_context,
            update_channel,
            source_stream,
        }
    }

    pub fn rest(
        network_context: NetworkContext,
        update_channel: velor_channels::Sender<ConnectivityRequest>,
        rest_url: url::Url,
        interval_duration: Duration,
        time_service: TimeService,
    ) -> Self {
        let source_stream = DiscoveryChangeStream::Rest(RestStream::new(
            network_context,
            rest_url,
            interval_duration,
            time_service,
        ));
        DiscoveryChangeListener {
            discovery_source: DiscoverySource::Rest,
            network_context,
            update_channel,
            source_stream,
        }
    }

    pub fn start(self, executor: &Handle) {
        spawn_named!("DiscoveryChangeListener", executor, Box::pin(self).run());
    }

    async fn run(mut self: Pin<Box<Self>>) {
        let network_context = self.network_context;
        let discovery_source = self.discovery_source;
        let mut update_channel = self.update_channel.clone();
        let source_stream = &mut self.source_stream;
        info!(
            NetworkSchema::new(&network_context),
            "{} Starting {} Discovery", network_context, discovery_source
        );

        while let Some(update) = source_stream.next().await {
            if let Ok(update) = update {
                trace!(
                    NetworkSchema::new(&network_context),
                    "{} Sending update: {:?}",
                    network_context,
                    update
                );
                let request = ConnectivityRequest::UpdateDiscoveredPeers(discovery_source, update);
                if let Err(error) = update_channel.try_send(request) {
                    inc_by_with_context(&DISCOVERY_COUNTS, &network_context, "send_failure", 1);
                    warn!(
                        NetworkSchema::new(&network_context),
                        "{} Failed to send update {:?}", network_context, error
                    );
                }
            } else {
                warn!(
                    NetworkSchema::new(&network_context),
                    "{} {} Discovery update failed {:?}",
                    &network_context,
                    discovery_source,
                    update
                );
            }
        }
        warn!(
            NetworkSchema::new(&network_context),
            "{} {} Discovery actor terminated", &network_context, discovery_source
        );
    }

    pub fn discovery_source(&self) -> DiscoverySource {
        self.discovery_source
    }
}
