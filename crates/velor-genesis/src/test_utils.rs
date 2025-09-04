// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::builder::InitGenesisConfigFn;
use velor_config::config::{IdentityBlob, NodeConfig};
use velor_crypto::ed25519::Ed25519PrivateKey;
use velor_temppath::TempPath;
use velor_types::on_chain_config::Features;
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;

pub fn test_config() -> (NodeConfig, Ed25519PrivateKey) {
    test_config_with_custom_onchain(None)
}

pub fn test_config_with_custom_features(
    init_features: Features,
) -> (NodeConfig, Ed25519PrivateKey) {
    test_config_with_custom_onchain(Some(Arc::new(move |config| {
        config.initial_features_override = Some(init_features.clone());
    })))
}

pub fn test_config_with_custom_onchain(
    init_genesis_config: Option<InitGenesisConfigFn>,
) -> (NodeConfig, Ed25519PrivateKey) {
    let path = TempPath::new();
    path.create_as_dir().unwrap();
    let (root_key, _genesis, _genesis_waypoint, validators) = crate::builder::Builder::new(
        path.path(),
        velor_cached_packages::head_release_bundle().clone(),
    )
    .unwrap()
    .with_init_genesis_config(init_genesis_config)
    .build(StdRng::from_seed([0; 32]))
    .unwrap();
    let (
        IdentityBlob {
            account_address,
            account_private_key: _,
            consensus_private_key,
            ..
        },
        _,
        _,
        _,
    ) = validators[0].get_key_objects(None).unwrap();
    let mut configs = validators.into_iter().map(|v| v.config).collect::<Vec<_>>();

    let mut config = configs.swap_remove(0);
    let config = config.override_config_mut();
    config.set_data_dir(path.path().to_path_buf());

    let mut sr_test = velor_config::config::SafetyRulesTestConfig::new(account_address.unwrap());
    sr_test.consensus_key(consensus_private_key.unwrap());
    config.consensus.safety_rules.test = Some(sr_test);

    (config.clone(), root_key)
}
