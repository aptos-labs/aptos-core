// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{counters::DISCOVERY_COUNTS, file::FileStream, validator_set::ValidatorSetStream};
use aptos_config::{config::PeerSet, network_id::NetworkContext};
use aptos_crypto::x25519;
use aptos_logger::prelude::*;
use aptos_time_service::TimeService;
use event_notifications::ReconfigNotificationListener;
use futures::{Stream, StreamExt};
use network::{
    connectivity_manager::{ConnectivityRequest, DiscoverySource},
    counters::inc_by_with_context,
    logging::NetworkSchema,
};
use std::{
    path::Path,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::runtime::Handle;

mod counters;
mod file;
mod validator_set;

#[derive(Debug)]
pub enum DiscoveryError {
    IO(std::io::Error),
    Parsing(String),
}

/// A union type for all implementations of `DiscoveryChangeListenerTrait`
pub struct DiscoveryChangeListener {
    discovery_source: DiscoverySource,
    network_context: NetworkContext,
    update_channel: channel::Sender<ConnectivityRequest>,
    source_stream: DiscoveryChangeStream,
}

enum DiscoveryChangeStream {
    ValidatorSet(ValidatorSetStream),
    File(FileStream),
}

impl Stream for DiscoveryChangeStream {
    type Item = Result<PeerSet, DiscoveryError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            Self::ValidatorSet(stream) => Pin::new(stream).poll_next(cx),
            Self::File(stream) => Pin::new(stream).poll_next(cx),
        }
    }
}

impl DiscoveryChangeListener {
    pub fn validator_set(
        network_context: NetworkContext,
        update_channel: channel::Sender<ConnectivityRequest>,
        expected_pubkey: x25519::PublicKey,
        reconfig_events: ReconfigNotificationListener,
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
        update_channel: channel::Sender<ConnectivityRequest>,
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
