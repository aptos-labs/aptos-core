// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{protocols::network::SerializedRequest, ProtocolId};
use bytes::Bytes;
use serde::Serialize;
use std::fmt::Debug;

#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct Message {
    /// The [`ProtocolId`] for which of our upstream application modules should
    /// handle (i.e., deserialize and then respond to) this inbound rpc request.
    ///
    /// For example, if `protocol_id == ProtocolId::ConsensusRpc`, then this
    /// inbound rpc request will be dispatched to consensus for handling.
    pub protocol_id: ProtocolId,
    /// The serialized request data received from the sender. At this layer in
    /// the stack, the request data is just an opaque blob and will only be fully
    /// deserialized later in the handling application module.
    #[serde(skip)]
    pub mdata: Bytes,
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mdata_str = if self.mdata.len() <= 10 {
            format!("{:?}", self.mdata)
        } else {
            format!("{:?}...", self.mdata.slice(..10))
        };
        write!(
            f,
            "Message {{ protocol: {:?}, mdata: {} }}",
            self.protocol_id, mdata_str
        )
    }
}

impl SerializedRequest for Message {
    fn protocol_id(&self) -> ProtocolId {
        self.protocol_id
    }

    fn data(&self) -> &Bytes {
        &self.mdata
    }
}
