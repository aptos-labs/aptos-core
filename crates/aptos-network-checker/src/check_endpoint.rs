// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use aptos_config::{
    config::{RoleType, HANDSHAKE_VERSION},
    network_id::{NetworkContext, NetworkId},
};
use aptos_crypto::x25519::{self, PRIVATE_KEY_SIZE};
use aptos_types::{account_address, chain_id::ChainId, network_address::NetworkAddress, PeerId};
use futures::{AsyncReadExt, AsyncWriteExt};
use network::{
    noise::{HandshakeAuthMode, NoiseUpgrader},
    protocols::wire::handshake::v1::ProtocolIdSet,
    transport::{resolve_and_connect, TcpSocket},
    transport::{upgrade_outbound, UpgradeContext, SUPPORTED_MESSAGING_PROTOCOL},
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::time::Duration;

use crate::args::CheckEndpointArgs;

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
    upgrade_context: Arc<UpgradeContext>,
    address: NetworkAddress,
    remote_pubkey: x25519::PublicKey,
) -> Result<String> {
    // Connect to the address, this should handle DNS resolution if necessary.
    let fut_socket = async {
        resolve_and_connect(address.clone())
            .await
            .map(TcpSocket::new)
    };

    // The peer id doesn't matter because we don't validate it.
    let remote_peer_id = account_address::from_identity_public_key(remote_pubkey);
    let conn = upgrade_outbound(
        upgrade_context,
        fut_socket,
        address.clone(),
        remote_peer_id,
        remote_pubkey,
    )
    .await
    .with_context(|| format!("Failed to connect to {}", address))?;
    let msg = format!("Successfully connected to {}", conn.metadata.addr);

    // Disconnect.
    drop(conn);
    Ok(msg)
}

const INVALID_NOISE_HEADER: &[u8; 152] = &[7; 152];

async fn check_endpoint_no_handshake(address: NetworkAddress) -> Result<String> {
    let mut socket = resolve_and_connect(address.clone())
        .await
        .map(TcpSocket::new)
        .with_context(|| format!("Failed to connect to {}", address))?;

    socket
        .write_all(INVALID_NOISE_HEADER)
        .await
        .with_context(|| format!("Failed to write to {}", address))?;

    let buf = &mut [0; 1];
    match socket.read(buf).await {
        Ok(size) => {
            // We should be able to write to the socket dummy data.
            if size == 0 {
                // Connection is open, and doesn't return anything.
                // This is the closest we can get to working.
                Ok(format!(
                    "Accepted write and responded with nothing at {}",
                    address
                ))
            } else {
                bail!("Endpoint {} responded with data when it shouldn't", address);
            }
        }
        Err(error) => {
            bail!("Failed to read from {} due to error: {:#}", address, error);
        }
    }
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
            HandshakeAuthMode::server_only(),
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
