// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{tests::suite, PersistentSafetyStorage, SafetyRulesManager};
use aptos_crypto::{ed25519::Ed25519PrivateKey, Uniform};
use aptos_secure_storage::{KVStorage, Storage, VaultStorage};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vault_client::dev::{self, ROOT_TOKEN};

/// A test for verifying VaultStorage properly supports the SafetyRule backend.  This test
/// depends on running Vault, which can be done by using the provided docker run script in
/// `docker/testutils/start_vault_container.sh`
#[test]
fn test() {
    if dev::test_host_safe().is_none() {
        return;
    }

    let boolean_values = [false, true];
    for verify_vote_proposal_signature in &boolean_values {
        for export_consensus_key in &boolean_values {
            suite::run_test_suite(&safety_rules(
                *verify_vote_proposal_signature,
                *export_consensus_key,
            ));
        }
    }
}

fn safety_rules(
    verify_vote_proposal_signature: bool,
    export_consensus_key: bool,
) -> suite::Callback {
    Box::new(move || {
        let signer = ValidatorSigner::from_int(0);
        let mut storage = Storage::from(VaultStorage::new(
            dev::test_host(),
            ROOT_TOKEN.to_string(),
            None,
            None,
            true,
            None,
            None,
        ));
        storage.reset_and_clear().unwrap();

        let waypoint = crate::test_utils::validator_signers_to_waypoint(&[&signer]);
        let storage = PersistentSafetyStorage::initialize(
            storage,
            signer.author(),
            signer.private_key().clone(),
            Ed25519PrivateKey::generate_for_testing(),
            waypoint,
            true,
        );
        let safety_rules_manager = SafetyRulesManager::new_local(
            storage,
            verify_vote_proposal_signature,
            export_consensus_key,
        );
        let safety_rules = safety_rules_manager.client();
        (
            safety_rules,
            signer,
            if verify_vote_proposal_signature {
                Some(Ed25519PrivateKey::generate_for_testing())
            } else {
                None
            },
        )
    })
}
