// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    protocols::{
        network::SerializedRequest,
        wire::messaging::v1::{
            metadata::{MessageMetadata, MessageSendType, NetworkMessageWithMetadata},
            DirectSendMsg, NetworkMessage, Priority,
        },
    },
    ProtocolId,
};
use aptos_config::network_id::NetworkId;
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

    /// Transforms the message into a direct send network message with metadata
    pub fn into_network_message(self, network_id: NetworkId) -> NetworkMessageWithMetadata {
        // Create the direct send network message
        let network_message = NetworkMessage::DirectSendMsg(DirectSendMsg {
            protocol_id: self.protocol_id,
            priority: Priority::default(),
            raw_msg: Vec::from(self.data.as_ref()),
        });

        // Create and return the network message with metadata
        let message_metadata = MessageMetadata::new(
            network_id,
            Some(self.protocol_id),
            MessageSendType::DirectSend,
            Some(self.application_send_time),
        );
        NetworkMessageWithMetadata::new(message_metadata, network_message)
    }

    /// Consumes the message and returns the individual parts.
    /// Note: this is only for testing purposes (but, it cannot be marked
    /// as `#[cfg(test)]` because of several non-wrapped test utils).
    pub fn into_parts(self) -> (SystemTime, ProtocolId, Bytes) {
        (self.application_send_time, self.protocol_id, self.data)
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
