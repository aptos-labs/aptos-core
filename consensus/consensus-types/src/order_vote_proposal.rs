// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block::Block, quorum_cert::QuorumCert};
use velor_types::block_info::BlockInfo;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

/// This structure contains all the information needed by safety rules to
/// evaluate a QC on a block to produce an order vote.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderVoteProposal {
    #[serde(bound(deserialize = "Block: Deserialize<'de>"))]
    block: Block,
    /// BlockInfo for the above block
    block_info: BlockInfo,
    /// QuorumCert for the above block
    quorum_cert: Arc<QuorumCert>,
}

impl OrderVoteProposal {
    pub fn new(block: Block, block_info: BlockInfo, quorum_cert: Arc<QuorumCert>) -> Self {
        Self {
            block,
            block_info,
            quorum_cert,
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn block_info(&self) -> &BlockInfo {
        &self.block_info
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        &self.quorum_cert
    }
}

impl Display for OrderVoteProposal {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "OrderVoteProposal[block: {}]", self.block,)
    }
}
