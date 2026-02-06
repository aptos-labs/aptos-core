// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines the structs transported during the AptosNet handshake protocol and
//! the AptosNet messaging protocol.
//! The handshake protocol is executed prior to executing the messaging protocol, and is used to
//! determine the version of messaging protocol to use. Each node only supports one version of the
//! handshake protocol on an end-point, and that is advertised as part of its discovery
//! NetworkAddress.

pub mod handshake;
pub mod messaging;
pub mod serde_bytes_compat;
