// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct InspectionServiceConfig {
    pub address: String,
    pub port: u16,
    pub expose_configuration: bool,
    pub expose_system_information: bool,
}

impl Default for InspectionServiceConfig {
    fn default() -> InspectionServiceConfig {
        InspectionServiceConfig {
            address: "0.0.0.0".to_string(),
            port: 9101,
            expose_configuration: false,
            expose_system_information: true,
        }
    }
}

impl InspectionServiceConfig {
    pub fn randomize_ports(&mut self) {
        self.port = utils::get_available_port();
    }
}
