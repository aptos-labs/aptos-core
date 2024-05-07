// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// <Black magic>
// Increase recursion limit to allow for use of select! macro.
#![recursion_limit = "1024"]
// </Black magic>

// TODO(philiphayes): uncomment when feature stabilizes (est. 1.50.0)
// tracking issue: https://github.com/rust-lang/rust/issues/78835
// #![doc = include_str!("../README.md")]

use rand::{Rng, thread_rng};
use fail::fail_point;

pub mod application;
pub mod connectivity_manager;
pub mod constants;
pub mod counters;
pub mod error;
pub mod logging;
pub mod noise;
pub mod peer;
pub mod peer_manager;
pub mod protocols;
pub mod transport;

#[cfg(feature = "fuzzing")]
pub mod fuzzing;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
pub mod testutils;

pub type DisconnectReason = peer::DisconnectReason;
pub type ConnectivityRequest = connectivity_manager::ConnectivityRequest;
pub type ProtocolId = protocols::wire::handshake::v1::ProtocolId;

pub fn maul(bytes: Vec<u8>) -> Vec<u8> {
    fail_point!("network::maul_outgoing_msgs", |_| {
        let mut bytes = bytes.clone();
        let mut rng = thread_rng();
        let sample = rng.gen_range(0.0, 1.0);
        let n = bytes.len();
        if sample < 0.25 {
            // Insert a byte.
            let ins_idx = rng.gen_range(0, n + 1);
            bytes.insert(ins_idx, rng.gen());
        } else if sample < 0.50 && n >= 1 {
            // Delete a byte.
            let del_idx = rng.gen_range(0, n);
            bytes.remove(del_idx);
        } else if sample < 0.75 && n >= 1 {
            // Update a byte.
            let upd_idx = rng.gen_range(0, n);
            bytes[upd_idx] = rng.gen();
        }
        bytes
    });
    bytes
}
