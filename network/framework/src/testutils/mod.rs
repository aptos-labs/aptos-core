// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::application::storage::PeersAndMetadata;
use velor_config::network_id::NetworkContext;
use velor_crypto::x25519::PublicKey;
use velor_types::PeerId;
use std::sync::Arc;

pub mod builder;
pub mod fake_socket;
pub mod test_framework;
pub mod test_node;

/// Creates a network context for a client and server, and returns the
/// contexts alongside peers and metadata.
pub fn create_client_server_network_context(
    client_public_key: Option<PublicKey>,
    server_public_key: Option<PublicKey>,
    peers_and_metadata: Option<Arc<PeersAndMetadata>>,
) -> (NetworkContext, NetworkContext, Arc<PeersAndMetadata>) {
    // Create the client context
    let client_network_context = create_context_for_public_key(client_public_key);

    // Create the server context
    let server_network_context = create_context_for_public_key(server_public_key);
    assert_eq!(
        client_network_context.network_id(),
        server_network_context.network_id()
    );

    // Create the trusted peers and metadata
    let peers_and_metadata = peers_and_metadata
        .unwrap_or_else(|| PeersAndMetadata::new(&[client_network_context.network_id()]));

    (
        client_network_context,
        server_network_context,
        peers_and_metadata,
    )
}

/// Creates a network context for the given public key.
/// Otherwise, uses a random peer ID.
fn create_context_for_public_key(public_key: Option<PublicKey>) -> NetworkContext {
    let peer_id = match public_key {
        Some(public_key) => velor_types::account_address::from_identity_public_key(public_key),
        None => PeerId::random(),
    };
    NetworkContext::mock_with_peer_id(peer_id)
}
