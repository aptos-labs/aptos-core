// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::SigningKey;
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, CORE_CODE_ADDRESS},
    chain_id::ChainId,
};
use move_core_types::parser::parse_struct_tag;
use rstest::rstest;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct RotationCapabilityOfferProofChallengeV2 {
    account_address: AccountAddress,
    module_name: String,
    struct_name: String,
    chain_id: u8,
    sequence_number: u64,
    source_address: AccountAddress,
    recipient_address: AccountAddress,
}

// TODO[Orderless]: Revisit this test for stateless accounts.
// Should the sequence number in the above test be made optional?
// Will the rotation capability fail for stateless accounts?

#[rstest(
    offerer_stateless_account,
    delegate_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn offer_rotation_capability_test(
    offerer_stateless_account: bool,
    delegate_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut harness =
        MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let offerer_account = harness.new_account_with_key_pair(
        if offerer_stateless_account {
            None
        } else {
            Some(0)
        },
    );
    let delegate_account = harness.new_account_with_key_pair(
        if delegate_stateless_account {
            None
        } else {
            Some(0)
        },
    );

    offer_rotation_capability_v2(&mut harness, &offerer_account, &delegate_account);
    revoke_rotation_capability(&mut harness, &offerer_account, *delegate_account.address());
}

pub fn offer_rotation_capability_v2(
    harness: &mut MoveHarness,
    offerer_account: &Account,
    delegate_account: &Account,
) {
    let rotation_capability_proof = RotationCapabilityOfferProofChallengeV2 {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationCapabilityOfferProofChallengeV2"),
        chain_id: ChainId::test().id(),
        sequence_number: 0,
        source_address: *offerer_account.address(),
        recipient_address: *delegate_account.address(),
    };

    let rotation_capability_proof_msg = bcs::to_bytes(&rotation_capability_proof);
    let rotation_proof_signed = offerer_account
        .privkey
        .sign_arbitrary_message(&rotation_capability_proof_msg.unwrap());

    assert_success!(harness.run_transaction_payload(
        offerer_account,
        aptos_stdlib::account_offer_rotation_capability(
            rotation_proof_signed.to_bytes().to_vec(),
            0,
            offerer_account.pubkey.to_bytes(),
            *delegate_account.address(),
        )
    ));

    let account_resource = parse_struct_tag("0x1::account::Account").unwrap();
    assert_eq!(
        harness
            .read_resource::<AccountResource>(offerer_account.address(), account_resource)
            .unwrap()
            .rotation_capability_offer()
            .unwrap(),
        *delegate_account.address()
    );
}

pub fn revoke_rotation_capability(
    harness: &mut MoveHarness,
    offerer_account: &Account,
    delegate_address: AccountAddress,
) {
    assert_success!(harness.run_transaction_payload(
        offerer_account,
        aptos_stdlib::account_revoke_rotation_capability(delegate_address,)
    ));
    let account_resource = parse_struct_tag("0x1::account::Account").unwrap();
    assert_eq!(
        harness
            .read_resource::<AccountResource>(offerer_account.address(), account_resource)
            .unwrap()
            .rotation_capability_offer(),
        Option::<AccountAddress>::None,
    );
}
