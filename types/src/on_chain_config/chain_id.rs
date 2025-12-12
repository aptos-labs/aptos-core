// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{chain_id::ChainId, on_chain_config::OnChainConfig};

impl OnChainConfig for ChainId {
    const MODULE_IDENTIFIER: &'static str = "chain_id";
    const TYPE_IDENTIFIER: &'static str = "ChainId";
}
