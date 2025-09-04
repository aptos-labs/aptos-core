// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This crate implements wrappers around our [Noise][noise] implementation.
//! Noise is a protocol framework to encrypt and authentication connections.
//! We use Noise to secure connections between peers in Velor.
//! Specifically, we use the [Noise IK][ik] handshake which is a one round-trip protocol
//! (the client sends one message, then the server responds).
//! For more information about Noise and our implementation, refer to the [crypto] crate.
//!
//! Usage example:
//!
//! ```
//! use velor_network::noise::{AntiReplayTimestamps, HandshakeAuthMode, NoiseUpgrader};
//! use futures::{executor, future, io::{AsyncReadExt, AsyncWriteExt}};
//! use velor_memsocket::MemorySocket;
//! use velor_config::{config::{Peer, PeerRole}, network_id::{NetworkContext, NetworkId}};
//! use velor_crypto::{x25519, ed25519, Uniform, PrivateKey, test_utils::TEST_SEED};
//! use velor_infallible::RwLock;
//! use rand::{rngs::StdRng, SeedableRng};
//! use velor_types::PeerId;
//! use std::{collections::{HashSet, HashMap}, io, sync::Arc};
//! use velor_config::config::RoleType;
//! use velor_network::application::storage::PeersAndMetadata;
//!
//! fn example() -> io::Result<()> {
//! // create client and server NoiseUpgrader
//! let mut rng = StdRng::from_seed(TEST_SEED);
//! let client_private = x25519::PrivateKey::generate(&mut rng);
//! let client_public = client_private.public_key();
//! let client_peer_id = PeerId::random();
//!
//! let server_private = x25519::PrivateKey::generate(&mut rng);
//! let server_public = server_private.public_key();
//! let server_peer_id = PeerId::random();
//!
//! // create list of trusted peers
//! let network_id = NetworkId::Validator;
//! let client_pubkey_set: HashSet<_> = vec![client_public].into_iter().collect();
//! let server_pubkey_set: HashSet<_> = vec![server_public].into_iter().collect();
//!
//! let peers_and_metadata = PeersAndMetadata::new(&[network_id]);
//! let mut trusted_peers = peers_and_metadata.get_trusted_peers(&network_id).unwrap();
//! trusted_peers.insert(client_peer_id, Peer::new(Vec::new(), client_pubkey_set, PeerRole::Validator));
//! trusted_peers.insert(server_peer_id, Peer::new(Vec::new(), server_pubkey_set, PeerRole::Validator));
//! peers_and_metadata.set_trusted_peers(&network_id, trusted_peers).unwrap();
//!
//! let client_auth = HandshakeAuthMode::mutual(peers_and_metadata.clone());
//! let client_context = NetworkContext::new(
//!     RoleType::Validator,
//!     network_id,
//!     client_peer_id,
//! );
//! let client = NoiseUpgrader::new(client_context, client_private, client_auth);
//!
//! let server_auth = HandshakeAuthMode::mutual(peers_and_metadata);
//! let server_context = NetworkContext::new(
//!     RoleType::Validator,
//!     network_id,
//!     server_peer_id,
//! );
//! let server = NoiseUpgrader::new(server_context, server_private, server_auth);
//!
//! // use an in-memory socket as example
//! let (dialer_socket, listener_socket) = MemorySocket::new_pair();
//!
//! // perform the handshake
//! let (client_session, server_session) = executor::block_on(future::join(
//!    client.upgrade_outbound(dialer_socket, server_peer_id, server_public, AntiReplayTimestamps::now),
//!    server.upgrade_inbound(listener_socket),
//! ));
//!
//! let (mut client_session, _) = client_session
//!     .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
//! let (mut server_session, _client_peer_id, _trust_level) = server_session
//!     .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
//!
//! // client -> server
//! executor::block_on(client_session.write_all(b"client hello"))?;
//! executor::block_on(client_session.flush())?;
//!
//! let mut buf = [0; 12];
//! executor::block_on(server_session.read_exact(&mut buf))?;
//! assert_eq!(&buf, b"client hello");
//!
//! // client <- server
//! executor::block_on(server_session.write_all(b"server hello"))?;
//! executor::block_on(server_session.flush())?;
//!
//! let mut buf = [0; 12];
//! executor::block_on(client_session.read_exact(&mut buf))?;
//! assert_eq!(&buf, b"server hello");
//!
//! Ok(())
//! }
//!
//! example().unwrap();
//! ```
//!
//! [noise]: http://noiseprotocol.org/
//! [ik]: https://noiseexplorer.com/patterns/IK
//! [crypto]: ../velor_crypto/noise/index.html

pub mod error;
pub mod handshake;
pub mod stream;

#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing;

pub use error::NoiseHandshakeError;
pub use handshake::{AntiReplayTimestamps, HandshakeAuthMode, NoiseUpgrader};
