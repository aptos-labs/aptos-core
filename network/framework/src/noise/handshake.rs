// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The handshake module implements the handshake part of the protocol.
//! This module also implements additional anti-DoS mitigation,
//! by including a timestamp in each handshake initialization message.
//! Refer to the module's documentation for more information.
//! A successful handshake returns a [`NoiseStream`] which is defined in the
//! [stream] module.
//!
//! [stream]: crate::noise::stream

use crate::{
    application::storage::PeersAndMetadata,
    logging::NetworkSchema,
    noise::{error::NoiseHandshakeError, stream::NoiseStream},
};
use velor_config::{
    config::{Peer, PeerRole},
    network_id::{NetworkContext, NetworkId},
};
use velor_crypto::{noise, x25519};
use velor_infallible::{duration_since_epoch, RwLock};
use velor_logger::{error, trace};
use velor_short_hex_str::{AsShortHexStr, ShortHexStr};
use velor_types::PeerId;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::{collections::HashMap, convert::TryFrom as _, fmt::Debug, sync::Arc};

/// In a mutually authenticated network, a client message is accompanied with a timestamp.
/// This is in order to prevent replay attacks, where the attacker does not know the client's static key,
/// but can still replay a handshake message in order to force a peer into performing a few Diffie-Hellman key exchange operations.
///
/// Thus, to prevent replay attacks a responder will always check if the timestamp is strictly increasing,
/// effectively considering it as a stateful counter.
///
/// If the client timestamp has been seen before, or is not strictly increasing,
/// we can abort the handshake early and avoid heavy Diffie-Hellman computations.
/// If the client timestamp is valid, we store it.
#[derive(Default)]
pub struct AntiReplayTimestamps(HashMap<x25519::PublicKey, u64>);

impl AntiReplayTimestamps {
    /// The timestamp is sent as a payload, so that it is encrypted.
    /// Note that a millisecond value is a 16-byte value in rust,
    /// but as we use it to store a duration since UNIX_EPOCH we will never use more than 8 bytes.
    pub const TIMESTAMP_SIZE: usize = 8;

    /// obtain the current timestamp
    pub fn now() -> [u8; Self::TIMESTAMP_SIZE] {
        let now: u64 = duration_since_epoch().as_millis() as u64; // (TIMESTAMP_SIZE)

        // e.g. [157, 126, 253, 97, 114, 1, 0, 0]
        now.to_le_bytes()
    }

    /// Returns true if the timestamp has already been observed for this peer
    /// or if it's an old timestamp
    pub fn is_replay(&self, pubkey: x25519::PublicKey, timestamp: u64) -> bool {
        if let Some(last_timestamp) = self.0.get(&pubkey) {
            &timestamp <= last_timestamp
        } else {
            false
        }
    }

    /// Stores the timestamp
    pub fn store_timestamp(&mut self, pubkey: x25519::PublicKey, timestamp: u64) {
        self.0
            .entry(pubkey)
            .and_modify(|last_timestamp| *last_timestamp = timestamp)
            .or_insert(timestamp);
    }
}

/// Noise handshake authentication mode.
pub enum HandshakeAuthMode {
    /// In `Mutual` mode, both sides will authenticate each other with their
    /// `trusted_peers` set. We also include replay attack mitigation in this mode.
    ///
    /// For example, in the Velor validator network, validator peers will only
    /// allow connections from other validator peers. They will use this mode to
    /// check that inbound connections authenticate to a network public key
    /// actually contained in the current validator set.
    Mutual {
        // Only use anti replay protection in mutual-auth scenarios. In theory,
        // this is applicable everywhere; however, we would need to spend some
        // time making this more sophisticated so it garbage collects old
        // timestamps and doesn't use unbounded space. These are not problems in
        // mutual-auth scenarios because we have a bounded set of trusted peers
        // that rarely changes.
        anti_replay_timestamps: RwLock<AntiReplayTimestamps>,
        peers_and_metadata: Arc<PeersAndMetadata>,
    },
    /// In `MaybeMutual` mode, the dialer authenticates the server and the server will allow all
    /// inbound connections from any peer but will mark connections as `Trusted` if the incoming
    /// connection is apart of its trusted peers set.
    MaybeMutual(Arc<PeersAndMetadata>),
}

impl HandshakeAuthMode {
    pub fn mutual(peers_and_metadata: Arc<PeersAndMetadata>) -> Self {
        HandshakeAuthMode::Mutual {
            anti_replay_timestamps: RwLock::new(AntiReplayTimestamps::default()),
            peers_and_metadata,
        }
    }

    pub fn maybe_mutual(peers_and_metadata: Arc<PeersAndMetadata>) -> Self {
        HandshakeAuthMode::MaybeMutual(peers_and_metadata)
    }

    pub fn server_only(network_ids: &[NetworkId]) -> Self {
        let peers_and_metadata = PeersAndMetadata::new(network_ids);
        HandshakeAuthMode::maybe_mutual(peers_and_metadata)
    }

    #[cfg(test)]
    pub fn server_only_with_metadata(peers_and_metadata: Arc<PeersAndMetadata>) -> Self {
        HandshakeAuthMode::maybe_mutual(peers_and_metadata)
    }

    fn anti_replay_timestamps(&self) -> Option<&RwLock<AntiReplayTimestamps>> {
        match &self {
            HandshakeAuthMode::Mutual {
                anti_replay_timestamps,
                ..
            } => Some(anti_replay_timestamps),
            HandshakeAuthMode::MaybeMutual(_) => None,
        }
    }
}

// Noise Upgrader
// --------------
// Noise by default is not aware of the above or lower protocol layers,
// We thus need to build this wrapper around Noise to both:
//
// - fragment messages that need to be encrypted by noise (due to its maximum 65535-byte messages)
// - understand how long noise messages we send and receive are,
//   in order to pass them to the noise implementaiton
//

/// The Noise configuration to be used to perform a protocol upgrade on an underlying socket.
pub struct NoiseUpgrader {
    /// The validator's network context
    pub network_context: NetworkContext,
    /// Config for executing Noise handshakes. Includes our static private key.
    noise_config: noise::NoiseConfig,
    /// Handshake authentication can be either mutual or server-only authentication.
    auth_mode: HandshakeAuthMode,
}

impl NoiseUpgrader {
    /// The client message consist of the prologue + a noise message with a timestamp as payload.
    const CLIENT_MESSAGE_SIZE: usize =
        Self::PROLOGUE_SIZE + noise::handshake_init_msg_len(AntiReplayTimestamps::TIMESTAMP_SIZE);
    /// The prologue is the client's peer_id and the remote's expected public key.
    const PROLOGUE_SIZE: usize = PeerId::LENGTH + x25519::PUBLIC_KEY_SIZE;
    /// The server's message contains no payload.
    const SERVER_MESSAGE_SIZE: usize = noise::handshake_resp_msg_len(0);

    /// Create a new NoiseConfig with the provided keypair and authentication mode.
    pub fn new(
        network_context: NetworkContext,
        key: x25519::PrivateKey,
        auth_mode: HandshakeAuthMode,
    ) -> Self {
        Self {
            network_context,
            noise_config: noise::NoiseConfig::new(key),
            auth_mode,
        }
    }

    /// Perform an outbound protocol upgrade on this connection.
    ///
    /// This runs the "client" side of the Noise IK handshake to establish a
    /// secure Noise stream and send its static public key to the server.
    /// In mutual auth scenarios, we will also include an anti replay attack counter in the
    /// Noise handshake payload. Currently this counter is always a millisecond-
    /// granularity unix epoch timestamp.
    pub async fn upgrade_outbound<TSocket, F>(
        &self,
        mut socket: TSocket,
        remote_peer_id: PeerId,
        remote_public_key: x25519::PublicKey,
        time_provider: F,
    ) -> Result<(NoiseStream<TSocket>, PeerRole), NoiseHandshakeError>
    where
        TSocket: AsyncRead + AsyncWrite + Debug + Unpin,
        F: Fn() -> [u8; AntiReplayTimestamps::TIMESTAMP_SIZE],
    {
        // buffer to hold prologue + first noise handshake message
        let mut client_message = [0; Self::CLIENT_MESSAGE_SIZE];

        // craft prologue = self_peer_id | expected_public_key
        client_message[..PeerId::LENGTH].copy_from_slice(self.network_context.peer_id().as_ref());
        client_message[PeerId::LENGTH..Self::PROLOGUE_SIZE]
            .copy_from_slice(remote_public_key.as_slice());

        let (prologue_msg, client_noise_msg) = client_message.split_at_mut(Self::PROLOGUE_SIZE);

        // craft 8-byte payload as current timestamp (in milliseconds)
        let payload = time_provider();

        // craft first handshake message  (-> e, es, s, ss)
        let mut rng = rand::rngs::OsRng;
        let initiator_state = self
            .noise_config
            .initiate_connection(
                &mut rng,
                prologue_msg,
                remote_public_key,
                Some(&payload),
                client_noise_msg,
            )
            .map_err(NoiseHandshakeError::BuildClientHandshakeMessageFailed)?;

        // send the first handshake message
        trace!(
            "{} noise client: handshake write: remote_public_key: {}",
            self.network_context,
            remote_public_key,
        );
        socket
            .write_all(&client_message)
            .await
            .map_err(NoiseHandshakeError::ClientWriteFailed)?;
        socket
            .flush()
            .await
            .map_err(NoiseHandshakeError::ClientFlushFailed)?;

        // receive the server's response (<- e, ee, se)
        trace!(
            "{} noise client: handshake read: remote_public_key: {}",
            self.network_context,
            remote_public_key,
        );
        let mut server_response = [0u8; Self::SERVER_MESSAGE_SIZE];
        socket
            .read_exact(&mut server_response)
            .await
            .map_err(NoiseHandshakeError::ClientReadFailed)?;

        // parse the server's response
        trace!(
            "{} noise client: handshake finalize: remote_public_key: {}",
            self.network_context,
            remote_public_key,
        );
        let (_, session) = self
            .noise_config
            .finalize_connection(initiator_state, &server_response)
            .map_err(NoiseHandshakeError::ClientFinalizeFailed)?;

        // finalize the connection
        let noise_stream = NoiseStream::new(socket, session);
        let peer_role = self.extract_peer_role_from_trusted_peers(remote_peer_id);

        Ok((noise_stream, peer_role))
    }

    /// Returns the peer role for the remote peer based on the trusted peer set.
    /// If the trusted peers is not found, or the trusted peers doesn't contain
    /// the remote peer, an error is logged and we return an unknown peer role.
    fn extract_peer_role_from_trusted_peers(&self, remote_peer_id: PeerId) -> PeerRole {
        // Get the peers and metadata struct
        let peers_and_metadata = match &self.auth_mode {
            HandshakeAuthMode::Mutual {
                peers_and_metadata, ..
            } => peers_and_metadata.clone(),
            HandshakeAuthMode::MaybeMutual(peers_and_metadata) => peers_and_metadata.clone(),
        };

        // Determine the peer role
        match peers_and_metadata.get_trusted_peers(&self.network_context.network_id()) {
            Ok(trusted_peers) => {
                match trusted_peers.get(&remote_peer_id) {
                    Some(trusted_peer) => {
                        return trusted_peer.role; // We've found the peer!
                    },
                    None => {
                        error!(NetworkSchema::new(&self.network_context).message(format!(
                            "{} Outbound connection made with unknown peer (not in the trusted peers set)! Missing peer: {:?}",
                            self.network_context, remote_peer_id

                        )));
                    },
                }
            },
            Err(error) => {
                error!(NetworkSchema::new(&self.network_context).message(format!(
                    "Failed to get trusted peers for network context: {:?}, error: {:?}",
                    self.network_context, error
                )));
            },
        };

        // If we couldn't determine the peer role, return an unknown peer role
        PeerRole::Unknown
    }

    /// Perform an inbound protocol upgrade on this connection.
    ///
    /// This runs the "server" side of the Noise IK handshake to establish a
    /// secure Noise stream and exchange static public keys. If the configuration
    /// requires mutual authentication, we will only allow connections from peers
    /// that successfully authenticate to a public key in our `trusted_peers` set.
    /// In addition, we will expect the client to include an anti replay attack
    /// counter in the Noise handshake payload in mutual auth scenarios.
    pub async fn upgrade_inbound<TSocket>(
        &self,
        mut socket: TSocket,
    ) -> Result<(NoiseStream<TSocket>, PeerId, PeerRole), NoiseHandshakeError>
    where
        TSocket: AsyncRead + AsyncWrite + Debug + Unpin,
    {
        // buffer to contain the client first message
        let mut client_message = [0; Self::CLIENT_MESSAGE_SIZE];

        // receive the prologue + first noise handshake message
        trace!("{} noise server: handshake read", self.network_context);
        socket
            .read_exact(&mut client_message)
            .await
            .map_err(NoiseHandshakeError::ServerReadFailed)?;

        // extract prologue (remote_peer_id | self_public_key)
        let (remote_peer_id, self_expected_public_key) =
            client_message[..Self::PROLOGUE_SIZE].split_at(PeerId::LENGTH);

        // parse the client's peer id
        // note: in mutual authenticated network, we could verify that their peer_id is in the trust peer set now.
        // We do this later in this function instead (to batch a number of checks) as there is no known attack here.
        let remote_peer_id = PeerId::try_from(remote_peer_id)
            .map_err(|_| NoiseHandshakeError::InvalidClientPeerId(hex::encode(remote_peer_id)))?;
        let remote_peer_short = remote_peer_id.short_str();

        // reject accidental self-dials
        // this situation could occur either as a result of our own discovery
        // mis-configuration or a potentially malicious discovery peer advertising
        // a (loopback ip or mirror proxy) and our public key.
        if remote_peer_id == self.network_context.peer_id() {
            return Err(NoiseHandshakeError::SelfDialDetected);
        }

        // verify that this is indeed our public key
        let actual_public_key = self.noise_config.public_key();
        if self_expected_public_key != actual_public_key.as_slice() {
            return Err(NoiseHandshakeError::ClientExpectingDifferentPubkey(
                remote_peer_short,
                hex::encode(self_expected_public_key),
                hex::encode(actual_public_key.as_slice()),
            ));
        }

        // parse it
        let (prologue, client_init_message) = client_message.split_at(Self::PROLOGUE_SIZE);
        let (remote_public_key, handshake_state, payload) = self
            .noise_config
            .parse_client_init_message(prologue, client_init_message)
            .map_err(|err| NoiseHandshakeError::ServerParseClient(remote_peer_short, err))?;

        // if mutual auth mode, verify the remote pubkey is in our set of trusted peers
        let network_id = self.network_context.network_id();
        let peer_role = match &self.auth_mode {
            HandshakeAuthMode::Mutual {
                peers_and_metadata, ..
            } => {
                let trusted_peers = peers_and_metadata.get_trusted_peers(&network_id)?;
                let trusted_peer = trusted_peers.get(&remote_peer_id).cloned();
                match trusted_peer {
                    Some(peer) => {
                        Self::authenticate_inbound(remote_peer_short, &peer, &remote_public_key)
                    },
                    None => Err(NoiseHandshakeError::UnauthenticatedClient(
                        remote_peer_short,
                        remote_peer_id,
                    )),
                }
            },
            HandshakeAuthMode::MaybeMutual(peers_and_metadata) => {
                let trusted_peers = peers_and_metadata.get_trusted_peers(&network_id)?;
                let trusted_peer = trusted_peers.get(&remote_peer_id).cloned();
                match trusted_peer {
                    Some(peer) => {
                        Self::authenticate_inbound(remote_peer_short, &peer, &remote_public_key)
                    },
                    None => {
                        // The peer is not in the trusted peer set. Verify that the Peer ID is
                        // constructed correctly from the public key.
                        let derived_remote_peer_id =
                            velor_types::account_address::from_identity_public_key(
                                remote_public_key,
                            );
                        if derived_remote_peer_id != remote_peer_id {
                            // The peer ID is not constructed correctly from the public key
                            Err(NoiseHandshakeError::ClientPeerIdMismatch(
                                remote_peer_short,
                                remote_peer_id,
                                derived_remote_peer_id,
                            ))
                        } else {
                            // Try to infer the role from the network context
                            if self.network_context.role().is_validator() {
                                if network_id.is_vfn_network() {
                                    // Inbound connections to validators on the VFN network must be VFNs
                                    Ok(PeerRole::ValidatorFullNode)
                                } else {
                                    // Otherwise, they're unknown. Validators will connect through
                                    // authenticated channels (on the validator network) so shouldn't hit
                                    // this, and PFNs will connect on public networks (which aren't common).
                                    Ok(PeerRole::Unknown)
                                }
                            } else {
                                // We're a VFN or PFN. VFNs get no inbound connections on the vfn network
                                // (so the peer won't be a validator). Thus, we're on the public network
                                // so mark the peer as unknown.
                                Ok(PeerRole::Unknown)
                            }
                        }
                    },
                }
            },
        }?;

        // if on a mutually authenticated network,
        // the payload should contain a u64 client timestamp
        if let Some(anti_replay_timestamps) = self.auth_mode.anti_replay_timestamps() {
            // check that the payload received as the client timestamp (in seconds)
            if payload.len() != AntiReplayTimestamps::TIMESTAMP_SIZE {
                return Err(NoiseHandshakeError::MissingAntiReplayTimestamp(
                    remote_peer_short,
                ));
            }

            let mut client_timestamp = [0u8; AntiReplayTimestamps::TIMESTAMP_SIZE];
            client_timestamp.copy_from_slice(&payload);
            let client_timestamp = u64::from_le_bytes(client_timestamp);

            // check the timestamp is not a replay
            let mut anti_replay_timestamps = anti_replay_timestamps.write();
            if anti_replay_timestamps.is_replay(remote_public_key, client_timestamp) {
                return Err(NoiseHandshakeError::ServerReplayDetected(
                    remote_peer_short,
                    client_timestamp,
                ));
            }

            // store the timestamp
            anti_replay_timestamps.store_timestamp(remote_public_key, client_timestamp);
        }

        // construct the response
        let mut rng = rand::rngs::OsRng;
        let mut server_response = [0u8; Self::SERVER_MESSAGE_SIZE];
        let session = self
            .noise_config
            .respond_to_client(&mut rng, handshake_state, None, &mut server_response)
            .map_err(|err| {
                NoiseHandshakeError::BuildServerHandshakeMessageFailed(remote_peer_short, err)
            })?;

        // send the response
        trace!(
            "{} noise server: handshake write: remote_peer_id: {}",
            self.network_context,
            remote_peer_short,
        );
        socket
            .write_all(&server_response)
            .await
            .map_err(|err| NoiseHandshakeError::ServerWriteFailed(remote_peer_short, err))?;

        // finalize the connection
        trace!(
            "{} noise server: handshake finalize: remote_peer_id: {}",
            self.network_context,
            remote_peer_short,
        );

        let noise_stream = NoiseStream::new(socket, session);
        Ok((noise_stream, remote_peer_id, peer_role))
    }

    fn authenticate_inbound(
        remote_peer_short: ShortHexStr,
        peer: &Peer,
        remote_public_key: &x25519::PublicKey,
    ) -> Result<PeerRole, NoiseHandshakeError> {
        if !peer.keys.contains(remote_public_key) {
            return Err(NoiseHandshakeError::UnauthenticatedClientPubkey(
                remote_peer_short,
                hex::encode(remote_public_key.as_slice()),
            ));
        }
        Ok(peer.role)
    }
}

//
// Tests
// -----
//

#[cfg(test)]
mod test {
    use super::*;
    use crate::{testutils, testutils::fake_socket::ReadWriteTestSocket};
    use velor_config::config::{Peer, PeerRole, RoleType};
    use velor_crypto::{
        test_utils::TEST_SEED,
        traits::Uniform as _,
        x25519::{PrivateKey, PublicKey},
    };
    use velor_memsocket::MemorySocket;
    use velor_types::account_address::AccountAddress;
    use futures::{executor::block_on, future::join};
    use rand::{prelude::StdRng, SeedableRng as _};

    const TEST_SEED_2: [u8; 32] = [42; 32];

    /// helper to setup two testing peers
    fn build_peers(
        is_mutual_auth: bool,
        peers_and_metadata: Option<Arc<PeersAndMetadata>>,
    ) -> (
        (NoiseUpgrader, x25519::PublicKey),
        (NoiseUpgrader, x25519::PublicKey),
    ) {
        // Create a client and server keypair
        let mut rng = ::rand::rngs::StdRng::from_seed(TEST_SEED);
        let (client_private_key, client_public_key) = create_key_pair(&mut rng);
        let (server_private_key, server_public_key) = create_key_pair(&mut rng);

        let (client_auth, server_auth, client_network_context, server_network_context) =
            if is_mutual_auth {
                // Create the client and server network context
                let insert_trusted_peers = peers_and_metadata.is_none();
                let (client_network_context, server_network_context, peers_and_metadata) =
                    testutils::create_client_server_network_context(None, None, peers_and_metadata);

                // Update the trusted peers with the client and server
                // iff we're not using a provided peers_and_metadata struct.
                if insert_trusted_peers {
                    let peer_role = PeerRole::Validator;
                    let client = (
                        client_network_context.peer_id(),
                        Peer::new(vec![], [client_public_key].into_iter().collect(), peer_role),
                    );
                    let server = (
                        server_network_context.peer_id(),
                        Peer::new(vec![], [server_public_key].into_iter().collect(), peer_role),
                    );
                    insert_new_trusted_peers(
                        &peers_and_metadata,
                        client_network_context.network_id(),
                        vec![client, server],
                    );
                }

                (
                    HandshakeAuthMode::mutual(peers_and_metadata.clone()),
                    HandshakeAuthMode::mutual(peers_and_metadata),
                    client_network_context,
                    server_network_context,
                )
            } else {
                // Create the client and server network context
                let (client_network_context, server_network_context, peers_and_metadata) =
                    testutils::create_client_server_network_context(
                        Some(client_public_key),
                        Some(server_public_key),
                        peers_and_metadata,
                    );

                (
                    HandshakeAuthMode::server_only_with_metadata(peers_and_metadata.clone()),
                    HandshakeAuthMode::server_only_with_metadata(peers_and_metadata),
                    client_network_context,
                    server_network_context,
                )
            };

        let client = NoiseUpgrader::new(client_network_context, client_private_key, client_auth);
        let server = NoiseUpgrader::new(server_network_context, server_private_key, server_auth);

        ((client, client_public_key), (server, server_public_key))
    }

    /// Creates a key pair using the given RNG
    fn create_key_pair(mut rng: &mut StdRng) -> (PrivateKey, PublicKey) {
        let client_private_key = x25519::PrivateKey::generate(&mut rng);
        let client_public_key = client_private_key.public_key();
        (client_private_key, client_public_key)
    }

    /// helper to perform a noise handshake with two peers
    fn perform_handshake(
        client: &NoiseUpgrader,
        server: &NoiseUpgrader,
        server_public_key: x25519::PublicKey,
    ) -> (
        Result<(NoiseStream<MemorySocket>, PeerRole), NoiseHandshakeError>,
        Result<(NoiseStream<MemorySocket>, PeerId, PeerRole), NoiseHandshakeError>,
    ) {
        // create an in-memory socket for testing
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();

        // perform the handshake
        block_on(join(
            client.upgrade_outbound(
                dialer_socket,
                server.network_context.peer_id(),
                server_public_key,
                AntiReplayTimestamps::now,
            ),
            server.upgrade_inbound(listener_socket),
        ))
    }

    /// provide a function that will return the same given value as a timestamp
    fn bad_timestamp(value: u64) -> impl Fn() -> [u8; AntiReplayTimestamps::TIMESTAMP_SIZE] {
        move || value.to_le_bytes()
    }

    #[test]
    fn test_timestamp_replay() {
        // 1. generate peers
        let ((client, _), (server, server_public_key)) = build_peers(true, None);
        let server_peer_id = server.network_context.peer_id();

        // 2. perform the handshake with some timestamp, it should work
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();
        let (client_session, server_session) = block_on(join(
            client.upgrade_outbound(
                dialer_socket,
                server_peer_id,
                server_public_key,
                bad_timestamp(1),
            ),
            server.upgrade_inbound(listener_socket),
        ));

        client_session.unwrap();
        server_session.unwrap();

        // 3. perform the handshake again with timestamp in the past, it should fail
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();
        let (client_session, server_session) = block_on(join(
            client.upgrade_outbound(
                dialer_socket,
                server_peer_id,
                server_public_key,
                bad_timestamp(0),
            ),
            server.upgrade_inbound(listener_socket),
        ));

        client_session.unwrap_err();
        server_session.unwrap_err();

        // 4. perform the handshake again with the same timestamp, it should fail
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();
        let (client_session, server_session) = block_on(join(
            client.upgrade_outbound(
                dialer_socket,
                server_peer_id,
                server_public_key,
                bad_timestamp(1),
            ),
            server.upgrade_inbound(listener_socket),
        ));

        client_session.unwrap_err();
        server_session.unwrap_err();

        // 5. perform the handshake again with a valid timestamp in the future, it should work
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();
        let (client_session, server_session) = block_on(join(
            client.upgrade_outbound(
                dialer_socket,
                server_peer_id,
                server_public_key,
                bad_timestamp(2),
            ),
            server.upgrade_inbound(listener_socket),
        ));

        client_session.unwrap();
        server_session.unwrap();
    }

    fn test_handshake_success(is_mutual_auth: bool) {
        // perform handshake with two testing peers
        let ((client, client_public_key), (server, server_public_key)) =
            build_peers(is_mutual_auth, None);

        let (client_res, server_res) = perform_handshake(&client, &server, server_public_key);
        let (client_stream, _) = client_res.unwrap();
        let (server_stream, _, _) = server_res.unwrap();

        assert_eq!(client_stream.get_remote_static(), server_public_key);
        assert_eq!(server_stream.get_remote_static(), client_public_key);
    }

    #[test]
    fn test_handshake_success_server_only_auth() {
        test_handshake_success(false /* is_mutual_auth */);
    }

    #[test]
    fn test_handshake_success_mutual_auth() {
        test_handshake_success(true /* is_mutual_auth */);
    }

    fn test_handshake_self_fails(is_mutual_auth: bool) {
        let (_, (server, server_public_key)) = build_peers(is_mutual_auth, None);
        let (client_res, server_res) = perform_handshake(&server, &server, server_public_key);

        client_res.unwrap_err();
        server_res.unwrap_err();
    }

    #[test]
    fn test_handshake_self_fails_server_only_auth() {
        test_handshake_self_fails(false /* is_mutual_auth */);
    }

    #[test]
    fn test_handshake_self_fails_mutual_auth() {
        test_handshake_self_fails(true /* is_mutual_auth */);
    }

    #[test]
    fn test_handshake_unauthed_keypair_fails_mutual_auth() {
        let mut rng = ::rand::rngs::StdRng::from_seed(TEST_SEED_2);
        let client_private_key = x25519::PrivateKey::generate(&mut rng);

        let ((mut client, _), (server, server_public_key)) = build_peers(true, None);

        // swap in a different keypair, so the connection will be unauthenticated
        client.noise_config = noise::NoiseConfig::new(client_private_key);
        let (client_res, server_res) = perform_handshake(&client, &server, server_public_key);

        client_res.unwrap_err();
        server_res.unwrap_err();
    }

    #[test]
    fn test_handshake_unauthed_peerid_fails_mutual_auth() {
        let mut rng = ::rand::rngs::StdRng::from_seed(TEST_SEED_2);
        let client_private_key = x25519::PrivateKey::generate(&mut rng);

        // build a client with an unrecognized peer id, so the connection will be
        // unauthenticated
        let client_peer_id = PeerId::random();
        let client = NoiseUpgrader::new(
            NetworkContext::mock_with_peer_id(client_peer_id),
            client_private_key,
            HandshakeAuthMode::mutual(PeersAndMetadata::new(&[])),
        );

        let (_, (server, server_public_key)) = build_peers(true, None);
        let (client_res, server_res) = perform_handshake(&client, &server, server_public_key);

        client_res.unwrap_err();
        server_res.unwrap_err();
    }

    #[test]
    fn test_handshake_client_peerid_mismatch_fails_server_only_auth() {
        ::velor_logger::Logger::init_for_testing();

        let ((mut client, _), (server, server_public_key)) = build_peers(false, None);
        client.network_context = NetworkContext::mock_with_peer_id(PeerId::random());
        let (client_res, server_res) = perform_handshake(&client, &server, server_public_key);

        client_res.unwrap_err();
        server_res.unwrap_err();
    }

    #[test]
    fn test_handshake_fragmented_reads() {
        // create an in-memory socket for testing
        let (mut dialer_socket, mut listener_socket) = ReadWriteTestSocket::new_pair();

        // fragment reads
        dialer_socket.set_fragmented_read();
        listener_socket.set_fragmented_read();

        // get peers
        let ((client, _client_public_key), (server, server_public_key)) = build_peers(false, None);
        let server_peer_id = server.network_context.peer_id();

        // perform the handshake
        let (client_session, server_session) = block_on(join(
            client.upgrade_outbound(
                dialer_socket,
                server_peer_id,
                server_public_key,
                AntiReplayTimestamps::now,
            ),
            server.upgrade_inbound(listener_socket),
        ));

        client_session.unwrap();
        server_session.unwrap();
    }

    #[test]
    fn test_handshake_peer_roles_pfn_dials_vfn() {
        // Initialize the logger
        ::velor_logger::Logger::init_for_testing();

        // Create a peers and metadata struct
        let network_ids = vec![NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);

        // Create a client and server with mutual auth disabled
        let ((mut client, _), (mut server, server_public_key)) =
            build_peers(false, Some(peers_and_metadata.clone()));

        // Update the client network context
        let client_peer_id = client.network_context.peer_id();
        let client_network_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Public, client_peer_id);
        client.network_context = client_network_context;

        // Update the server network context
        let server_peer_id = server.network_context.peer_id();
        let server_network_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Public, server_peer_id);
        server.network_context = server_network_context;

        // Add the VFN to the trusted peers set
        let server_peer = (
            server_peer_id,
            Peer::new(
                vec![],
                [server_public_key].into_iter().collect(),
                PeerRole::ValidatorFullNode,
            ),
        );
        insert_new_trusted_peers(&peers_and_metadata, NetworkId::Public, vec![server_peer]);

        // Create an in-memory socket for testing
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();

        // Create the client connection task
        let client_connection_task = async move {
            let (_, peer_role) = client
                .upgrade_outbound(
                    dialer_socket,
                    server_peer_id,
                    server_public_key,
                    AntiReplayTimestamps::now,
                )
                .await
                .unwrap();
            assert_eq!(peer_role, PeerRole::ValidatorFullNode);
        };

        // Create the server connection task
        let server_connection_task = async move {
            let (_, peer_id, peer_role) = server.upgrade_inbound(listener_socket).await.unwrap();
            assert_eq!(peer_id, client_peer_id);
            assert_eq!(peer_role, PeerRole::Unknown);
        };

        // Perform the handshake
        block_on(join(client_connection_task, server_connection_task));
    }

    #[test]
    fn test_handshake_peer_roles_validator_dials_validator() {
        // Initialize the logger
        ::velor_logger::Logger::init_for_testing();

        // Create a client and server with mutual auth enabled
        let ((client, _), (server, server_public_key)) = build_peers(true, None);
        let server_peer_id = server.network_context.peer_id();

        // Create an in-memory socket for testing
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();

        // Create the client connection task
        let client_peer_id = client.network_context.peer_id();
        let client_connection_task = async move {
            let (_, peer_role) = client
                .upgrade_outbound(
                    dialer_socket,
                    server_peer_id,
                    server_public_key,
                    AntiReplayTimestamps::now,
                )
                .await
                .unwrap();
            assert_eq!(peer_role, PeerRole::Validator);
        };

        // Create the server connection task
        let server_connection_task = async move {
            let (_, peer_id, peer_role) = server.upgrade_inbound(listener_socket).await.unwrap();
            assert_eq!(peer_id, client_peer_id);
            assert_eq!(peer_role, PeerRole::Validator);
        };

        // Perform the handshake
        block_on(join(client_connection_task, server_connection_task));
    }

    #[test]
    fn test_handshake_peer_roles_vfn_dials_validator() {
        // Initialize the logger
        ::velor_logger::Logger::init_for_testing();

        // Create a peers and metadata struct with no trusted peers
        let network_ids = vec![NetworkId::Validator, NetworkId::Vfn];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);

        // Create a client and server with mutual auth disabled
        let ((mut client, _), (mut server, server_public_key)) =
            build_peers(false, Some(peers_and_metadata.clone()));

        // Update the client network context
        let client_peer_id = client.network_context.peer_id();
        let client_network_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Vfn, client_peer_id);
        client.network_context = client_network_context;

        // Update the server network context
        let server_peer_id = server.network_context.peer_id();
        let server_network_context =
            NetworkContext::new(RoleType::Validator, NetworkId::Vfn, server_peer_id);
        server.network_context = server_network_context;

        // Add the validator to the trusted peers set
        let server_peer = (
            server_peer_id,
            Peer::new(
                vec![],
                [server_public_key].into_iter().collect(),
                PeerRole::Validator,
            ),
        );
        insert_new_trusted_peers(&peers_and_metadata, NetworkId::Vfn, vec![server_peer]);

        // Create an in-memory socket for testing
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();

        // Create the client connection task
        let client_connection_task = async move {
            let (_, peer_role) = client
                .upgrade_outbound(
                    dialer_socket,
                    server_peer_id,
                    server_public_key,
                    AntiReplayTimestamps::now,
                )
                .await
                .unwrap();
            assert_eq!(peer_role, PeerRole::Validator);
        };

        // Create the server connection task
        let server_connection_task = async move {
            let (_, peer_id, peer_role) = server.upgrade_inbound(listener_socket).await.unwrap();
            assert_eq!(peer_id, client_peer_id);
            assert_eq!(peer_role, PeerRole::ValidatorFullNode);
        };

        // Perform the handshake
        block_on(join(client_connection_task, server_connection_task));
    }

    #[test]
    fn test_handshake_peer_roles_vfn_dials_vfn() {
        // Initialize the logger
        ::velor_logger::Logger::init_for_testing();

        // Create a peers and metadata struct with no trusted peers
        let network_ids = vec![NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);

        // Create a client and server with mutual auth disabled
        let ((mut client, client_public_key), (mut server, server_public_key)) =
            build_peers(false, Some(peers_and_metadata.clone()));

        // Update the client network context
        let client_peer_id = client.network_context.peer_id();
        let client_network_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Public, client_peer_id);
        client.network_context = client_network_context;

        // Update the server network context
        let server_peer_id = server.network_context.peer_id();
        let server_network_context =
            NetworkContext::new(RoleType::FullNode, NetworkId::Public, server_peer_id);
        server.network_context = server_network_context;

        // Add the client VFN and server VFN to the trusted peers set
        let client_peer = (
            client_peer_id,
            Peer::new(
                vec![],
                [client_public_key].into_iter().collect(),
                PeerRole::ValidatorFullNode,
            ),
        );
        let server_peer = (
            server_peer_id,
            Peer::new(
                vec![],
                [server_public_key].into_iter().collect(),
                PeerRole::ValidatorFullNode,
            ),
        );
        insert_new_trusted_peers(&peers_and_metadata, NetworkId::Public, vec![
            client_peer,
            server_peer,
        ]);

        // Create an in-memory socket for testing
        let (dialer_socket, listener_socket) = MemorySocket::new_pair();

        // Create the client connection task
        let client_connection_task = async move {
            let (_, peer_role) = client
                .upgrade_outbound(
                    dialer_socket,
                    server_peer_id,
                    server_public_key,
                    AntiReplayTimestamps::now,
                )
                .await
                .unwrap();
            assert_eq!(peer_role, PeerRole::ValidatorFullNode);
        };

        // Create the server connection task
        let server_connection_task = async move {
            let (_, peer_id, peer_role) = server.upgrade_inbound(listener_socket).await.unwrap();
            assert_eq!(peer_id, client_peer_id);
            assert_eq!(peer_role, PeerRole::ValidatorFullNode);
        };

        // Perform the handshake
        block_on(join(client_connection_task, server_connection_task));
    }

    /// Inserts the given peers into the trusted peer set for the specified network
    fn insert_new_trusted_peers(
        peers_and_metadata: &Arc<PeersAndMetadata>,
        network_id: NetworkId,
        peers: Vec<(AccountAddress, Peer)>,
    ) {
        // Get a copy of the trusted peers
        let mut trusted_peers = peers_and_metadata.get_trusted_peers(&network_id).unwrap();

        // Insert the new peers
        for (peer_address, peer) in peers {
            trusted_peers.insert(peer_address, peer);
        }

        // Update the trusted peers
        peers_and_metadata
            .set_trusted_peers(&network_id, trusted_peers)
            .unwrap();
    }
}
