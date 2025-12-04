// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{test_utils, SafetyRulesManager};
use aptos_types::validator_signer::ValidatorSigner;

#[test]
fn test_reconnect() {
    let signer = ValidatorSigner::from_int(0);
    let storage = test_utils::test_storage(&signer);
    // test value for network timeout, in milliseconds.
    let network_timeout = 5_000;
    let safety_rules_manager = SafetyRulesManager::new_thread(storage, network_timeout);

    // Verify that after a client has disconnected a new client will connect and resume operations
    let state0 = safety_rules_manager.client().consensus_state().unwrap();
    let state1 = safety_rules_manager.client().consensus_state().unwrap();
    assert_eq!(state0, state1);
}
