// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{test_utils, tests::suite, SafetyRulesManager};
use aptos_types::validator_signer::ValidatorSigner;

#[test]
fn test() {
    suite::run_test_suite_without_sig_check(&safety_rules());
}

fn safety_rules() -> suite::Callback {
    Box::new(move || {
        let signer = ValidatorSigner::from_int(0);
        let storage = test_utils::test_storage(&signer);
        let safety_rules_manager = SafetyRulesManager::new_local(storage);
        let safety_rules = safety_rules_manager.client();
        (safety_rules, signer)
    })
}
