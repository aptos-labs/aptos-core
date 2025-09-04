// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// Defines the version of Velor Validator software.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct VelorVersion {
    pub major: u64,
}

impl OnChainConfig for VelorVersion {
    const MODULE_IDENTIFIER: &'static str = "version";
    const TYPE_IDENTIFIER: &'static str = "Version";
}

// NOTE: version number for release 1.2 Velor
// Items gated by this version number include:
//  - the EntryFunction payload type
pub const VELOR_VERSION_2: VelorVersion = VelorVersion { major: 2 };

// NOTE: version number for release 1.3 of Velor
// Items gated by this version number include:
//  - Multi-agent transactions
pub const VELOR_VERSION_3: VelorVersion = VelorVersion { major: 3 };

// NOTE: version number for release 1.4 of Velor
// Items gated by this version number include:
//  - Conflict-Resistant Sequence Numbers
pub const VELOR_VERSION_4: VelorVersion = VelorVersion { major: 4 };

// Maximum current known version
pub const VELOR_MAX_KNOWN_VERSION: VelorVersion = VELOR_VERSION_4;
