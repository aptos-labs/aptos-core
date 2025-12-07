// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{test_utils, tests::suite, SafetyRules};
use aptos_types::validator_signer::ValidatorSigner;

#[test]
fn test() {
    suite::run_test_suite(&safety_rules());
}

fn safety_rules() -> suite::Callback {
    Box::new(move || {
        let signer = ValidatorSigner::from_int(0);
        let storage = test_utils::test_storage(&signer);
        let safety_rules = Box::new(SafetyRules::new(storage, false));
        (safety_rules, signer)
    })
}
