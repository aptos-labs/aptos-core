// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NodeStartupConfig {
    pub skip_config_optimizer: bool, // Whether or not to skip the config optimizer at startup
    pub skip_config_sanitizer: bool, // Whether or not to skip the config sanitizer at startup
}

#[allow(clippy::derivable_impls)] // Derive default manually (this is safer than guessing defaults)
impl Default for NodeStartupConfig {
    fn default() -> Self {
        Self {
            skip_config_optimizer: false,
            skip_config_sanitizer: false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_node_startup_config_default() {
        // Create the default config
        let config = NodeStartupConfig::default();

        // Verify both fields are set to false
        assert!(!config.skip_config_optimizer);
        assert!(!config.skip_config_sanitizer);
    }
}
