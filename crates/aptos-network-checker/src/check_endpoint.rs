// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::args::CheckEndpointArgs;
use anyhow::{Context, Result};
use aptos_config::{
    config::{RoleType, HANDSHAKE_VERSION},
    network_id::{NetworkContext, NetworkId},
};
use aptos_crypto::x25519::{self, PRIVATE_KEY_SIZE};
use aptos_network::{
    noise::{HandshakeAuthMode, NoiseUpgrader},
    protocols::wire::handshake::v1::ProtocolIdSet,
    transport::{UpgradeContext, SUPPORTED_MESSAGING_PROTOCOL},
};
use aptos_types::{account_address, chain_id::ChainId, network_address::NetworkAddress, PeerId};
use std::{collections::BTreeMap, sync::Arc};
use tokio::time::Duration;

// This function must take the private key in as an owned value vs as part of
// the args struct because private key needs to be owned, and cannot be cloned.
pub async fn check_endpoint(
    args: &CheckEndpointArgs,
    private_key: Option<x25519::PrivateKey>,
) -> Result<String> {
    let private_key = private_key.unwrap_or_else(|| {
        let dummy = [0; PRIVATE_KEY_SIZE];
        x25519::PrivateKey::from(dummy)
    });
    let (peer_id, public_key) = private_key_to_public_info(&private_key);
    let timeout = Duration::from_secs(args.handshake_args.timeout_seconds);
    aptos_logger::debug!(
        "Connecting with peer ID {} and pubkey {} to {} with timeout: {:?}",
        peer_id,
        public_key,
        args.node_address_args.address,
        timeout
    );
    check_endpoint_wrapper(
        build_upgrade_context(
            args.node_address_args.chain_id,
            args.handshake_args.network_id,
            peer_id,
            private_key,
        ),
        &args.node_address_args.address,
        timeout,
        args.handshake_args.no_handshake,
    )
    .await
}

async fn check_endpoint_wrapper(
    upgrade_context: Arc<UpgradeContext>,
    address: &NetworkAddress,
    timeout: Duration,
    no_handshake: bool,
) -> Result<String> {
    let remote_pubkey = address.find_noise_proto().with_context(|| {
        format!(
            "Failed to find noise protocol in {}, /noise-ik/<pubkey> missing",
            address
        )
    })?;

    tokio::time::timeout(timeout, async {
        if no_handshake {
            check_endpoint_no_handshake(address.clone()).await
        } else {
            check_endpoint_with_handshake(upgrade_context.clone(), address.clone(), remote_pubkey)
                .await
        }
    })
    .await
    .with_context(|| format!("Timed out while checking endpoint {}", address))?
}

/// Connects via Noise, then drops the connection.
async fn check_endpoint_with_handshake(
    _upgrade_context: Arc<UpgradeContext>,
    _address: NetworkAddress,
    _remote_pubkey: x25519::PublicKey,
) -> Result<String> {
    unimplemented!()
}

async fn check_endpoint_no_handshake(_address: NetworkAddress) -> Result<String> {
    unimplemented!()
}

/// Builds a listener free noise connector
fn build_upgrade_context(
    chain_id: ChainId,
    network_id: NetworkId,
    peer_id: PeerId,
    private_key: x25519::PrivateKey,
) -> Arc<UpgradeContext> {
    // RoleType doesn't matter, but the `NetworkId` and `PeerId` are used in
    // handshakes.
    let network_context = NetworkContext::new(RoleType::FullNode, network_id, peer_id);

    // Build supported protocols.
    let mut supported_protocols = BTreeMap::new();
    supported_protocols.insert(SUPPORTED_MESSAGING_PROTOCOL, ProtocolIdSet::all_known());

    // Build the noise and network handshake, without running a full Noise server
    // with listener.
    Arc::new(UpgradeContext::new(
        NoiseUpgrader::new(
            network_context,
            private_key,
            // If we had an incoming message, auth mode would matter.
            HandshakeAuthMode::server_only(&[network_id]),
        ),
        HANDSHAKE_VERSION,
        supported_protocols,
        chain_id,
        network_id,
    ))
}

/// Derive the peer id that we're using. This is a convenience to only have to
/// provide a private key.
fn private_key_to_public_info(private_key: &x25519::PrivateKey) -> (PeerId, x25519::PublicKey) {
    let public_key = private_key.public_key();
    let peer_id = account_address::from_identity_public_key(public_key);
    (peer_id, public_key)
}
