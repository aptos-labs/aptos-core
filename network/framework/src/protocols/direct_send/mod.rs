// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{protocols::network::SerializedRequest, ProtocolId};
use bytes::Bytes;
use serde::Serialize;
use std::{fmt::Debug, time::SystemTime};

#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct Message {
    /// The time at which the message was sent by the application
    application_send_time: SystemTime,
    /// The [`ProtocolId`] for which of our upstream application modules should
    /// handle (i.e., deserialize and then respond to) this inbound rpc request.
    ///
    /// For example, if `protocol_id == ProtocolId::ConsensusRpcBcs`, then this
    /// inbound rpc request will be dispatched to consensus for handling.
    protocol_id: ProtocolId,
    /// The serialized request data received from the sender. At this layer in
    /// the stack, the request data is just an opaque blob and will only be fully
    /// deserialized later in the handling application module.
    #[serde(skip)]
    data: Bytes,
}

impl Message {
    pub fn new(protocol_id: ProtocolId, data: Bytes) -> Self {
        Self {
            application_send_time: SystemTime::now(),
            protocol_id,
            data,
        }
    }

    /// Returns the time at which the message was sent by the application
    pub fn application_send_time(&self) -> SystemTime {
        self.application_send_time
    }

    /// Consumes the message and returns the protocol id and data
    pub fn into_parts(self) -> (ProtocolId, Bytes) {
        (self.protocol_id, self.data)
    }
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mdata_str = if self.data().len() <= 10 {
            format!("{:?}", self.data())
        } else {
            format!("{:?}...", self.data().slice(..10))
        };
        write!(
            f,
            "Message {{ protocol: {:?}, data: {} }}",
            self.protocol_id(),
            mdata_str
        )
    }
}

impl SerializedRequest for Message {
    fn protocol_id(&self) -> ProtocolId {
        self.protocol_id
    }

    fn data(&self) -> &Bytes {
        &self.data
    }
}
