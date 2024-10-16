// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// Defines the version of Aptos Validator software.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct AptosVersion {
    pub major: u64,
}

impl OnChainConfig for AptosVersion {
    const MODULE_IDENTIFIER: &'static str = "version";
    const TYPE_IDENTIFIER: &'static str = "Version";
}

// NOTE: version number for release 1.2 Aptos
// Items gated by this version number include:
//  - the EntryFunction payload type
pub const APTOS_VERSION_2: AptosVersion = AptosVersion { major: 2 };

// NOTE: version number for release 1.3 of Aptos
// Items gated by this version number include:
//  - Multi-agent transactions
pub const APTOS_VERSION_3: AptosVersion = AptosVersion { major: 3 };

// NOTE: version number for release 1.4 of Aptos
// Items gated by this version number include:
//  - Conflict-Resistant Sequence Numbers
pub const APTOS_VERSION_4: AptosVersion = AptosVersion { major: 4 };

// Maximum current known version
pub const APTOS_MAX_KNOWN_VERSION: AptosVersion = APTOS_VERSION_4;
