// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{IdentityBlob, NodeConfig};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_temppath::TempPath;
use rand::{rngs::StdRng, SeedableRng};

pub fn test_config() -> (NodeConfig, Ed25519PrivateKey) {
    let path = TempPath::new();
    path.create_as_dir().unwrap();
    let (root_key, _genesis, _genesis_waypoint, validators) =
        crate::builder::Builder::new(path.path(), cached_packages::head_release_bundle().clone())
            .unwrap()
            .build(StdRng::from_seed([0; 32]))
            .unwrap();
    let (
        IdentityBlob {
            account_address,
            account_private_key,
            consensus_private_key,
            ..
        },
        _,
        _,
        _,
    ) = validators[0].get_key_objects(None).unwrap();
    let mut configs = validators.into_iter().map(|v| v.config).collect::<Vec<_>>();

    let mut config = configs.swap_remove(0);
    config.set_data_dir(path.path().to_path_buf());
    let mut test = aptos_config::config::TestConfig::new_with_temp_dir(Some(path));

    test.owner_key(account_private_key.unwrap());
    config.test = Some(test);

    let mut sr_test = aptos_config::config::SafetyRulesTestConfig::new(account_address.unwrap());
    sr_test.consensus_key(consensus_private_key.unwrap());
    config.consensus.safety_rules.test = Some(sr_test);

    (config, root_key)
}
