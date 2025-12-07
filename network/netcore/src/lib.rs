// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core types and traits for building Peer to Peer networks.
//!
//! The `netcore` crate contains all of the core functionality needed to build a Peer to Peer
//! network from building `Transport`s and `StreamMultiplexer`s to negotiating protocols on a
//! socket.

pub mod framing;
pub mod transport;
