// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters::{DISCOVERY_COUNTS, EVENT_PROCESSING_LOOP_BUSY_DURATION_S, NETWORK_KEY_MISMATCH},
    DiscoveryError,
};
use aptos_config::{
    config::{BaseConfig, NodeType, Peer, PeerRole, PeerSet, DEFAULT_PUBLIC_NETWORK_PORT},
    network_id::NetworkContext,
};
use aptos_crypto::x25519;
use aptos_event_notifications::ReconfigNotificationListener;
use aptos_logger::prelude::*;
use aptos_network::{counters::inc_by_with_context, logging::NetworkSchema};
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{
    account_address::AccountAddress,
    network_address::{NetworkAddress, Protocol},
    on_chain_config::{OnChainConfigPayload, OnChainConfigProvider, ValidatorSet},
};
use futures::Stream;
use std::{
    collections::HashSet,
    pin::Pin,
    task::{Context, Poll},
};

pub struct ValidatorSetStream<P: OnChainConfigProvider> {
    pub(crate) network_context: NetworkContext,
    expected_pubkey: x25519::PublicKey,
    reconfig_events: ReconfigNotificationListener<P>,
    node_type: NodeType,
    base_config: BaseConfig,
}

impl<P: OnChainConfigProvider> ValidatorSetStream<P> {
    pub(crate) fn new(
        network_context: NetworkContext,
        expected_pubkey: x25519::PublicKey,
        reconfig_events: ReconfigNotificationListener<P>,
        node_type: NodeType,
        base_config: BaseConfig,
    ) -> Self {
        Self {
            network_context,
            expected_pubkey,
            reconfig_events,
            node_type,
            base_config,
        }
    }

    fn find_key_mismatches(&self, onchain_keys: Option<&HashSet<x25519::PublicKey>>) {
        let mismatch = onchain_keys.map_or(0, |pubkeys| {
            if !pubkeys.contains(&self.expected_pubkey) {
                error!(
                    NetworkSchema::new(&self.network_context),
                    "Onchain pubkey {:?} differs from local pubkey {}",
                    pubkeys,
                    self.expected_pubkey
                );
                1
            } else {
                0
            }
        });

        NETWORK_KEY_MISMATCH
            .with_label_values(&[
                self.network_context.role().as_str(),
                self.network_context.network_id().as_str(),
                self.network_context.peer_id().short_str().as_str(),
            ])
            .set(mismatch);
    }

    fn extract_updates(&mut self, payload: OnChainConfigPayload<P>) -> PeerSet {
        let _process_timer = EVENT_PROCESSING_LOOP_BUSY_DURATION_S.start_timer();

        let node_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");

        let peer_set = extract_validator_set_updates(
            self.network_context,
            node_set,
            self.node_type,
            &self.base_config,
        );
        // Ensure that the public key matches what's onchain for this peer
        self.find_key_mismatches(
            peer_set
                .get(&self.network_context.peer_id())
                .map(|peer| &peer.keys),
        );

        inc_by_with_context(
            &DISCOVERY_COUNTS,
            &self.network_context,
            "new_nodes",
            peer_set.len() as u64,
        );

        peer_set
    }
}

impl<P: OnChainConfigProvider> Stream for ValidatorSetStream<P> {
    type Item = Result<PeerSet, DiscoveryError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.reconfig_events)
            .poll_next(cx)
            .map(|maybe_notification| {
                maybe_notification
                    .map(|notification| Ok(self.extract_updates(notification.on_chain_configs)))
            })
    }
}

/// Extracts a set of ConnectivityRequests from a ValidatorSet which are appropriate for a network with type role.
pub(crate) fn extract_validator_set_updates(
    network_context: NetworkContext,
    node_set: ValidatorSet,
    node_type: NodeType,
    base_config: &BaseConfig,
) -> PeerSet {
    let is_validator = network_context.network_id().is_validator_network();
    let is_pfn_with_validator_connections =
        node_type.is_public_fullnode() && base_config.enable_validator_pfn_connections;

    // Decode addresses while ignoring bad addresses
    node_set
        .into_iter()
        .map(|info| {
            let peer_id = *info.account_address();
            let config = info.into_config();

            let (addrs, peer_role) = if is_validator {
                // Validators should connect to advertised validator addresses
                let addrs = extract_addresses(
                    config.validator_network_addresses(),
                    network_context,
                    peer_id,
                );
                (addrs, PeerRole::Validator)
            } else if !is_pfn_with_validator_connections {
                // VFNs and PFNs should connect to advertised fullnode addresses
                let addrs = extract_addresses(
                    config.fullnode_network_addresses(),
                    network_context,
                    peer_id,
                );
                (addrs, PeerRole::ValidatorFullNode)
            } else {
                // Otherwise, PFNs with validator connections enabled should connect
                // to validator addresses directly (on the default public port).
                // However, as a fallback, if the validator connection fails, they
                // can try to connect to the fullnode addresses.
                let modified_validator_addresses: Vec<NetworkAddress> = extract_addresses(
                    config.validator_network_addresses(),
                    network_context,
                    peer_id,
                )
                .into_iter()
                .filter_map(set_port_to_public_network)
                .collect();

                // Extract the fullnode addresses as a fallback
                let fullnode_network_addresses = extract_addresses(
                    config.fullnode_network_addresses(),
                    network_context,
                    peer_id,
                );

                let addrs: Vec<NetworkAddress> = modified_validator_addresses
                    .into_iter()
                    .chain(fullnode_network_addresses)
                    .collect();
                (addrs, PeerRole::Validator)
            };

            (peer_id, Peer::from_addrs(peer_role, addrs))
        })
        .collect()
}

/// Parses a list of network addresses, logging and counting any
/// failures, and returning an empty list if parsing fails.
fn extract_addresses<E: std::error::Error + Send + Sync + 'static>(
    result: Result<Vec<NetworkAddress>, E>,
    network_context: NetworkContext,
    peer_id: AccountAddress,
) -> Vec<NetworkAddress> {
    result
        .map_err(anyhow::Error::from)
        .map_err(|error| {
            inc_by_with_context(&DISCOVERY_COUNTS, &network_context, "read_failure", 1);
            warn!(
                NetworkSchema::new(&network_context),
                "OnChainDiscovery: Failed to parse network addresses: peer: {}, err: {}",
                peer_id,
                error
            )
        })
        .unwrap_or_default()
}

/// Set the port in the given network address to the default public network port
fn set_port_to_public_network(addr: NetworkAddress) -> Option<NetworkAddress> {
    let protocols: Vec<Protocol> = addr
        .as_slice()
        .iter()
        .cloned()
        .map(|protocol| match protocol {
            Protocol::Tcp(_) => Protocol::Tcp(DEFAULT_PUBLIC_NETWORK_PORT),
            protocol => protocol,
        })
        .collect();
    NetworkAddress::from_protocols(protocols).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DiscoveryChangeListener;
    use aptos_channels::{aptos_channel, message_queues::QueueStyle};
    use aptos_config::{
        config::{RoleType, HANDSHAKE_VERSION},
        network_id::NetworkId,
    };
    use aptos_crypto::{bls12381, x25519::PrivateKey, PrivateKey as PK, Uniform};
    use aptos_event_notifications::ReconfigNotification;
    use aptos_types::{
        account_address,
        network_address::NetworkAddress,
        on_chain_config::{InMemoryOnChainConfig, OnChainConfig},
        validator_config::ValidatorConfig,
        validator_info::ValidatorInfo,
        PeerId,
    };
    use futures::executor::block_on;
    use rand::{rngs::StdRng, SeedableRng};
    use std::{collections::HashMap, str::FromStr, time::Instant};
    use tokio::{
        runtime::Runtime,
        time::{timeout_at, Duration},
    };

    #[test]
    fn metric_if_key_mismatch() {
        aptos_logger::Logger::init_for_testing();
        let runtime = Runtime::new().unwrap();
        let consensus_private_key = bls12381::PrivateKey::generate_for_testing();
        let consensus_pubkey = consensus_private_key.public_key();
        let pubkey = test_pubkey([0u8; 32]);
        let different_pubkey = test_pubkey([1u8; 32]);
        let peer_id = aptos_types::account_address::from_identity_public_key(pubkey);

        // Build up the Reconfig Listener
        let (conn_mgr_reqs_tx, _rx) = aptos_channels::new_test(1);
        let (mut reconfig_sender, reconfig_events) = aptos_channel::new(QueueStyle::LIFO, 1, None);
        let reconfig_listener = ReconfigNotificationListener {
            notification_receiver: reconfig_events,
        };
        let network_context = NetworkContext::mock_with_peer_id(peer_id);
        let listener = DiscoveryChangeListener::validator_set(
            network_context,
            conn_mgr_reqs_tx,
            pubkey,
            reconfig_listener,
            NodeType::Validator,
            BaseConfig::default(),
        );

        // Build up and send an update with a different pubkey
        send_pubkey_update(
            peer_id,
            consensus_pubkey,
            different_pubkey,
            &mut reconfig_sender,
        );

        let listener_future = async move {
            // Run the test, ensuring we actually stop after a couple seconds in case it fails to fail
            timeout_at(
                tokio::time::Instant::from(Instant::now() + Duration::from_secs(1)),
                Box::pin(listener).run(),
            )
            .await
            .expect_err("Expect timeout");
        };

        // Ensure the metric is updated
        check_network_key_mismatch_metric(0, &network_context);
        block_on(runtime.spawn(listener_future)).unwrap();
        check_network_key_mismatch_metric(1, &network_context);
    }

    #[test]
    fn test_extract_validator_set_updates_pfn_connections() {
        // Build a test validator set
        let validator_port = 1000;
        let fullnode_port = 2000;
        let (validator_peer_id, validator_set) =
            build_test_validator_set([0u8; 32], validator_port, fullnode_port);

        // Extract validator set updates with validator-PFN connections on a PFN
        let pfn_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Public, validator_peer_id);
        let base_config = BaseConfig {
            enable_validator_pfn_connections: true,
            ..BaseConfig::default()
        };
        let peer_set = extract_validator_set_updates(
            pfn_context,
            validator_set.clone(),
            NodeType::PublicFullnode,
            &base_config,
        );

        // Verify that the peer is classified as a validator
        let validator_peer = peer_set.get(&validator_peer_id).unwrap();
        assert_eq!(validator_peer.role, PeerRole::Validator);

        // Verify that the primary validator address uses the default public network port
        let primary = &validator_peer.addresses[0];
        let has_public_network_port = primary
            .as_slice()
            .iter()
            .any(|protocol| matches!(protocol, Protocol::Tcp(DEFAULT_PUBLIC_NETWORK_PORT)));
        assert!(
            has_public_network_port,
            "Primary address should use the default public network port!"
        );

        // Fallback (fullnode) address should also be present
        assert!(
            validator_peer.addresses.len() >= 2,
            "Should have both primary and fallback addresses!"
        );

        // Extract validator set updates with validator-PFN connections disabled on a PFN
        let peer_set = extract_validator_set_updates(
            pfn_context,
            validator_set.clone(),
            NodeType::PublicFullnode,
            &BaseConfig::default(),
        );

        // Verify that the peer is classified as a VFN
        let vfn_peer = peer_set.get(&validator_peer_id).unwrap();
        assert_eq!(vfn_peer.role, PeerRole::ValidatorFullNode);

        // Verify that the primary address uses fullnode addresses
        let primary = &vfn_peer.addresses[0];
        let has_fullnode_addr = primary
            .as_slice()
            .iter()
            .any(|protocol| protocol == &Protocol::Tcp(fullnode_port));
        assert!(has_fullnode_addr, "Primary address should be the fullnode address when validator-PFN connections are disabled!");
    }

    #[test]
    fn test_extract_validator_set_updates_validator() {
        // Build a test validator set
        let validator_port = 3000;
        let fullnode_port = 1234;
        let (validator_peer_id, validator_set) =
            build_test_validator_set([0u8; 32], validator_port, fullnode_port);

        // Extract validator set updates for a validator network
        let validator_context =
            NetworkContext::new(RoleType::Validator, NetworkId::Validator, validator_peer_id);
        let peer_set = extract_validator_set_updates(
            validator_context,
            validator_set.clone(),
            NodeType::Validator,
            &BaseConfig::default(),
        );

        // Verify that the peer is classified as a validator
        let validator_peer = peer_set.get(&validator_peer_id).unwrap();
        assert_eq!(validator_peer.role, PeerRole::Validator);

        // Verify that the primary address uses the validator port
        let has_validator_port = validator_peer.addresses[0]
            .as_slice()
            .iter()
            .any(|p| p == &Protocol::Tcp(validator_port));
        assert!(
            has_validator_port,
            "Validator should use validator port 6180!"
        );

        // Extract validator set updates for a validator network (with validator-PFN connections enabled)
        let base_config = BaseConfig {
            enable_validator_pfn_connections: true,
            ..BaseConfig::default()
        };
        let peer_set = extract_validator_set_updates(
            validator_context,
            validator_set,
            NodeType::Validator,
            &base_config,
        );

        // Verify that the peer is still classified as a validator
        let peer = peer_set.get(&validator_peer_id).unwrap();
        assert_eq!(peer.role, PeerRole::Validator);

        // Verify that the primary address still uses the validator port
        let has_validator_port = peer.addresses[0]
            .as_slice()
            .iter()
            .any(|p| p == &Protocol::Tcp(validator_port));
        assert!(
            has_validator_port,
            "Validator should use validator port 6180!"
        );
    }

    #[test]
    fn test_extract_validator_set_updates_vfn() {
        // Build a test validator set
        let validator_port = 9000;
        let fullnode_port = 5432;
        let (validator_peer_id, validator_set) =
            build_test_validator_set([0u8; 32], validator_port, fullnode_port);

        // Test both with validator-PFN connections enabled and disabled
        for enable_validator_pfn_connections in [false, true] {
            // Extract validator set updates for a VFN network
            let vfn_context =
                NetworkContext::new(RoleType::FullNode, NetworkId::Public, validator_peer_id);
            let base_config = BaseConfig {
                enable_validator_pfn_connections,
                ..BaseConfig::default()
            };
            let peer_set = extract_validator_set_updates(
                vfn_context,
                validator_set.clone(),
                NodeType::ValidatorFullnode,
                &base_config,
            );

            // Verify that the peer is classified as a VFN
            let peer = peer_set.get(&validator_peer_id).unwrap();
            assert_eq!(peer.role, PeerRole::ValidatorFullNode);

            // Verify that the primary address uses the fullnode port regardless of validator-PFN connections
            let has_fullnode_port = peer.addresses[0]
                .as_slice()
                .iter()
                .any(|p| p == &Protocol::Tcp(fullnode_port));
            assert!(
                has_fullnode_port,
                "VFN should use fullnode port 6182 (enable_validator_pfn_connections={})",
                enable_validator_pfn_connections
            );
        }
    }

    /// Builds a test validator set (containing a single validator and VFN pair)
    fn build_test_validator_set(
        pubkey_seed: [u8; 32],
        validator_port: u16,
        fullnode_port: u16,
    ) -> (PeerId, ValidatorSet) {
        // Create a validator peer ID
        let consensus_private_key = bls12381::PrivateKey::generate_for_testing();
        let pubkey = test_pubkey(pubkey_seed);
        let validator_peer_id = account_address::from_identity_public_key(pubkey);

        // Create network addresses for the validator and VFN
        let validator_addr =
            NetworkAddress::from_str(&format!("/ip4/1.2.3.4/tcp/{}", validator_port))
                .unwrap()
                .append_prod_protos(pubkey, HANDSHAKE_VERSION);
        let fullnode_addr =
            NetworkAddress::from_str(&format!("/ip4/1.3.6.9/tcp/{}", fullnode_port))
                .unwrap()
                .append_prod_protos(pubkey, HANDSHAKE_VERSION);

        // Create the validator set containing the single validator/VFN pair
        let validator_info = ValidatorInfo::new(
            validator_peer_id,
            0,
            ValidatorConfig::new(
                consensus_private_key.public_key(),
                bcs::to_bytes(&vec![validator_addr]).unwrap(),
                bcs::to_bytes(&vec![fullnode_addr]).unwrap(),
                0,
            ),
        );
        let validator_set = ValidatorSet::new(vec![validator_info.clone()]);

        (validator_peer_id, validator_set)
    }

    fn check_network_key_mismatch_metric(expected: i64, network_context: &NetworkContext) {
        assert_eq!(
            expected,
            NETWORK_KEY_MISMATCH
                .get_metric_with_label_values(&[
                    network_context.role().as_str(),
                    network_context.network_id().as_str(),
                    network_context.peer_id().short_str().as_str()
                ])
                .unwrap()
                .get()
        )
    }

    fn send_pubkey_update(
        peer_id: PeerId,
        consensus_pubkey: bls12381::PublicKey,
        pubkey: x25519::PublicKey,
        reconfig_tx: &mut aptos_channels::aptos_channel::Sender<
            (),
            ReconfigNotification<InMemoryOnChainConfig>,
        >,
    ) {
        let validator_address =
            NetworkAddress::mock().append_prod_protos(pubkey, HANDSHAKE_VERSION);
        let addresses = vec![validator_address];
        let validator_encoded_addresses = bcs::to_bytes(&addresses).unwrap();
        let fullnode_encoded_addresses = bcs::to_bytes(&addresses).unwrap();
        let validator = ValidatorInfo::new(
            peer_id,
            0,
            ValidatorConfig::new(
                consensus_pubkey,
                validator_encoded_addresses,
                fullnode_encoded_addresses,
                0,
            ),
        );
        let validator_set = ValidatorSet::new(vec![validator]);
        let mut configs = HashMap::new();
        configs.insert(
            ValidatorSet::CONFIG_ID,
            bcs::to_bytes(&validator_set).unwrap(),
        );
        let payload = OnChainConfigPayload::new(1, InMemoryOnChainConfig::new(configs));
        reconfig_tx
            .push((), ReconfigNotification {
                version: 1,
                on_chain_configs: payload,
            })
            .unwrap();
    }

    fn test_pubkey(seed: [u8; 32]) -> x25519::PublicKey {
        let mut rng: StdRng = SeedableRng::from_seed(seed);
        let private_key = PrivateKey::generate(&mut rng);
        private_key.public_key()
    }
}
