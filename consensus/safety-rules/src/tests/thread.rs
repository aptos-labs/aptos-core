// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{test_utils, tests::suite, SafetyRulesManager};
use aptos_types::validator_signer::ValidatorSigner;

#[test]
fn test() {
    suite::run_test_suite(&safety_rules());
}

fn safety_rules() -> suite::Callback {
    Box::new(move || {
        let signer = ValidatorSigner::from_int(0);
        let storage = test_utils::test_storage(&signer);
        // Test value for network_timeout, in milliseconds.
        let network_timeout = 5_000;
        let safety_rules_manager = SafetyRulesManager::new_thread(storage, network_timeout);
        let safety_rules = safety_rules_manager.client();
        (safety_rules, signer)
    })
}
