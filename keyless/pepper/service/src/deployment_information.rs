// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_build_info::build_information;
use aptos_infallible::Mutex;
use std::{collections::BTreeMap, sync::Arc};

/// A simple struct to hold deployment information as key-value pairs
#[derive(Clone, Debug)]
pub struct DeploymentInformation {
    deployment_information_map: Arc<Mutex<BTreeMap<String, String>>>,
}

impl DeploymentInformation {
    pub fn new() -> Self {
        // Collect the build information and initialize the map
        let build_information = build_information!();
        let deployment_information_map = Arc::new(Mutex::new(build_information));

        Self {
            deployment_information_map,
        }
    }

    /// Adds a new key-value pair to the deployment information map
    pub fn extend_deployment_information(&mut self, key: String, value: String) {
        self.deployment_information_map.lock().insert(key, value);
    }

    /// Returns a copy of the deployment information map
    pub fn get_deployment_information(&self) -> BTreeMap<String, String> {
        self.deployment_information_map.lock().clone()
    }
}

impl Default for DeploymentInformation {
    fn default() -> Self {
        Self::new()
    }
}
