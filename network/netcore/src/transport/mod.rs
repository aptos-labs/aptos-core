// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Low-level module for establishing connections with peers
//!
//! The main component of this module is the [`Transport`] trait, which provides an interface for
//! establishing both inbound and outbound connections with remote peers.
//!
//! [`Transport`]: crate::transport::Transport

use aptos_types::{network_address::NetworkAddress, PeerId};
use futures::{future::Future, stream::Stream};
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
pub mod memory;
pub mod proxy_protocol;
pub mod quic;
pub mod tcp;
mod utils;

/// Origin of how a Connection was established.
#[derive(Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ConnectionOrigin {
    /// `Inbound` indicates that we are the listener for this connection.
    Inbound,
    /// `Outbound` indicates that we are the dialer for this connection.
    Outbound,
}

impl ConnectionOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectionOrigin::Inbound => "inbound",
            ConnectionOrigin::Outbound => "outbound",
        }
    }
}

impl fmt::Debug for ConnectionOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for ConnectionOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug)]
/// A simple wrapper around a vector of sockets
pub struct MultiSocket<TSocket> {
    sockets: Vec<TSocket>,
}

impl<TSocket> MultiSocket<TSocket> {
    /// Creates a new MultiSocket with a single socket
    pub fn new_with_single_socket(socket: TSocket) -> Self {
        Self {
            sockets: vec![socket],
        }
    }

    /// Creates a new MultiSocket with multiple sockets
    pub fn new_with_multiple_sockets(sockets: Vec<TSocket>) -> Self {
        Self { sockets }
    }

    /// Consumes the MultiSocket and returns the underlying sockets
    pub fn into_sockets(self) -> Vec<TSocket> {
        self.sockets
    }

    /// Consumes the MultiSocket and returns the underlying single socket
    pub fn get_one_and_only(mut self) -> TSocket {
        if self.sockets.len() != 1 {
            panic!(
                "MultiSocket::get_one_and_only() called on a MultiSocket with {} sockets",
                self.sockets.len()
            );
        }
        self.sockets.remove(0)
    }
}

/// A Transport is responsible for establishing connections with remote Peers.
///
/// Connections are established either by [listening](Transport::listen_on)
/// or [dialing](Transport::dial) on a [`Transport`]. A peer that
/// obtains a connection by listening is often referred to as the *listener* and the
/// peer that initiated the connection through dialing as the *dialer*.
pub trait Transport {
    /// The result of establishing a connection.
    ///
    /// Generally this would include a socket-like streams which allows for sending and receiving
    /// of data through the connection.
    type Output;

    /// The Error type of errors which can happen while establishing a connection.
    type Error: ::std::error::Error + Send + Sync + 'static;

    /// A stream of [`Inbound`](Transport::Inbound) connections and the address of the dialer.
    ///
    /// An item should be produced whenever a connection is received at the lowest level of the
    /// transport stack. Each item is an [`Inbound`](Transport::Inbound) future
    /// that resolves to an [`Output`](Transport::Output) value once all protocol upgrades
    /// have been applied.
    type Listener: Stream<Item = Result<(Self::Inbound, NetworkAddress), Self::Error>>
        + Send
        + Unpin;

    /// A pending [`Output`](Transport::Output) for an inbound connection,
    /// obtained from the [`Listener`](Transport::Listener) stream.
    ///
    /// After a connection has been accepted by the transport, it may need to go through
    /// asynchronous post-processing (i.e. protocol upgrade negotiations). Such
    /// post-processing should not block the `Listener` from producing the next
    /// connection, hence further connection setup proceeds asynchronously.
    /// Once a `Inbound` future resolves it yields the [`Output`](Transport::Output)
    /// of the connection setup process.
    type Inbound: Future<Output = Result<MultiSocket<Self::Output>, Self::Error>> + Send;

    /// A pending [`Output`](Transport::Output) for an outbound connection,
    /// obtained from [dialing](Transport::dial) stream.
    type Outbound: Future<Output = Result<MultiSocket<Self::Output>, Self::Error>> + Send;

    /// Listens on the given [`NetworkAddress`], returning a stream of incoming connections.
    ///
    /// The returned [`NetworkAddress`] is the actual listening address, this is done to take into
    /// account OS-assigned port numbers (e.g. listening on port 0).
    fn listen_on(
        &mut self,
        addr: NetworkAddress,
    ) -> Result<(Self::Listener, NetworkAddress), Self::Error>
    where
        Self: Sized;

    /// Dials the given [`NetworkAddress`], returning a future for a pending outbound connection.
    fn dial(&self, peer_id: PeerId, addr: NetworkAddress) -> Result<Self::Outbound, Self::Error>
    where
        Self: Sized;
}
