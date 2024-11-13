// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_db::AptosDB;
use aptos_db_indexer::db_indexer::DBIndexer;
use aptos_executor_test_helpers::{
    gen_block_id, gen_ledger_info_with_sigs, integration_test_impl::create_db_and_executor,
};
use aptos_executor_types::BlockExecutorTrait;
use aptos_indexer_grpc_table_info::internal_indexer_db_service::InternalIndexerDBService;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{AccountKey, LocalAccount},
};
use aptos_storage_interface::DbReader;
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    account_config::aptos_test_root_address,
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    state_store::state_key::{prefix::StateKeyPrefix, StateKey},
    test_helpers::transaction_test_helpers::TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
    transaction::{
        signature_verified_transaction::into_signature_verified_block,
        Transaction::{self, UserTransaction},
        WriteSetPayload,
    },
};
use move_core_types::{ident_str, language_storage::StructTag};
use rand::SeedableRng;
use std::{fmt::Debug, str::FromStr, sync::Arc};

const B: u64 = 1_000_000_000;

#[cfg(test)]
pub fn create_test_db() -> (Arc<AptosDB>, LocalAccount) {
    // create test db
    let path = aptos_temppath::TempPath::new();
    let (genesis, validators) = aptos_vm_genesis::test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
    let core_resources_account: LocalAccount = LocalAccount::new(
        aptos_test_root_address(),
        AccountKey::from_private_key(aptos_vm_genesis::GENESIS_KEYPAIR.0.clone()),
        0,
    );
    let (aptos_db, _db, executor, _waypoint) =
        create_db_and_executor(path.path(), &genesis_txn, true);
    let parent_block_id = executor.committed_block_id();

    // This generates accounts that do not overlap with genesis
    let seed = [3u8; 32];
    let mut rng = ::rand::rngs::StdRng::from_seed(seed);
    let signer = aptos_types::validator_signer::ValidatorSigner::new(
        validators[0].data.owner_address,
        Arc::new(validators[0].consensus_key.clone()),
    );
    let account1 = LocalAccount::generate(&mut rng);
    let account2 = LocalAccount::generate(&mut rng);
    let account3 = LocalAccount::generate(&mut rng);

    let txn_factory = TransactionFactory::new(ChainId::test());

    let block1_id = gen_block_id(1);
    let block1_meta = Transaction::BlockMetadata(BlockMetadata::new(
        block1_id,
        1,
        0,
        signer.author(),
        vec![0],
        vec![],
        1,
    ));
    let tx1 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account1.public_key()));
    let tx2 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account2.public_key()));
    let tx3 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account3.public_key()));
    // Create account1 with 2T coins.
    let txn1 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account1.address(), 2_000 * B));
    // Create account2 with 1.2T coins.
    let txn2 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account2.address(), 1_200 * B));
    // Create account3 with 1T coins.
    let txn3 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account3.address(), 1_000 * B));

    // Transfer 20B coins from account1 to account2.
    // balance: <1.98T, 1.22T, 1T
    let txn4 =
        account1.sign_with_transaction_builder(txn_factory.transfer(account2.address(), 20 * B));

    // Transfer 10B coins from account2 to account3.
    // balance: <1.98T, <1.21T, 1.01T
    let txn5 =
        account2.sign_with_transaction_builder(txn_factory.transfer(account3.address(), 10 * B));

    // Transfer 70B coins from account1 to account3.
    // balance: <1.91T, <1.21T, 1.08T
    let txn6 =
        account1.sign_with_transaction_builder(txn_factory.transfer(account3.address(), 70 * B));

    let reconfig1 = core_resources_account.sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::aptos_governance_force_end_epoch_test_only()),
    );

    let block1: Vec<_> = into_signature_verified_block(vec![
        block1_meta,
        UserTransaction(tx1),
        UserTransaction(tx2),
        UserTransaction(tx3),
        UserTransaction(txn1),
        UserTransaction(txn2),
        UserTransaction(txn3),
        UserTransaction(txn4),
        UserTransaction(txn5),
        UserTransaction(txn6),
        UserTransaction(reconfig1),
    ]);
    let output1 = executor
        .execute_block(
            (block1_id, block1.clone()).into(),
            parent_block_id,
            TEST_BLOCK_EXECUTOR_ONCHAIN_CONFIG,
        )
        .unwrap();
    let li1 = gen_ledger_info_with_sigs(1, &output1, block1_id, &[signer.clone()]);
    executor.commit_blocks(vec![block1_id], li1).unwrap();
    (aptos_db, core_resources_account)
}

#[test]
fn test_db_indexer_data() {
    use std::{thread, time::Duration};
    // create test db
    let (aptos_db, core_account) = create_test_db();
    let total_version = aptos_db.expect_synced_version();
    assert_eq!(total_version, 11);
    let temp_path = TempPath::new();
    let mut node_config = aptos_config::config::NodeConfig::default();
    node_config.storage.dir = temp_path.path().to_path_buf();
    node_config.indexer_db_config.enable_event = true;
    node_config.indexer_db_config.enable_transaction = true;
    node_config.indexer_db_config.enable_statekeys = true;

    let internal_indexer_db = InternalIndexerDBService::get_indexer_db(&node_config).unwrap();

    let db_indexer = DBIndexer::new(internal_indexer_db.clone(), aptos_db.clone());
    // assert the data matches the expected data
    let version = internal_indexer_db.get_persisted_version().unwrap();
    assert_eq!(version, None);
    let mut start_version = version.map_or(0, |v| v + 1);
    while start_version < total_version {
        start_version = db_indexer.process_a_batch(start_version).unwrap();
    }
    // wait for the commit to finish
    thread::sleep(Duration::from_millis(100));
    // indexer has process all the transactions
    assert_eq!(
        internal_indexer_db.get_persisted_version().unwrap(),
        Some(total_version)
    );

    let txn_iter = internal_indexer_db
        .get_account_transaction_version_iter(core_account.address(), 0, 1000, total_version)
        .unwrap();
    let res: Vec<_> = txn_iter.collect();

    // core account submitted 7 transactions including last reconfig txn, and the first transaction is version 2
    assert!(res.len() == 7);
    assert!(res[0].as_ref().unwrap().1 == 2);

    let x = internal_indexer_db.get_event_by_key_iter().unwrap();
    let res: Vec<_> = x.collect();
    assert_eq!(res.len(), 4);

    let core_kv_iter = db_indexer
        .get_prefixed_state_value_iterator(
            &StateKeyPrefix::from(core_account.address()),
            None,
            total_version,
        )
        .unwrap();
    let core_kv_res: Vec<_> = core_kv_iter.collect();
    assert_eq!(core_kv_res.len(), 4);
    let address_one_kv_iter = db_indexer
        .get_prefixed_state_value_iterator(
            &StateKeyPrefix::from(AccountAddress::from_hex_literal("0x1").unwrap()),
            None,
            total_version,
        )
        .unwrap();
    let address_one_kv_res = address_one_kv_iter.collect::<Result<Vec<_>, _>>().unwrap();

    let (code, resources): (Vec<_>, Vec<_>) = address_one_kv_res
        .into_iter()
        .map(|(s, _)| s)
        .partition(|s| s.is_aptos_code());

    let expected_code = vec![
        ident_str!("acl"),
        ident_str!("any"),
        ident_str!("bcs"),
        ident_str!("dkg"),
        ident_str!("mem"),
        ident_str!("code"),
        ident_str!("coin"),
        ident_str!("guid"),
        ident_str!("hash"),
        ident_str!("jwks"),
        ident_str!("util"),
        ident_str!("block"),
        ident_str!("debug"),
        ident_str!("error"),
        ident_str!("event"),
        ident_str!("stake"),
        ident_str!("table"),
        ident_str!("math64"),
        ident_str!("object"),
        ident_str!("option"),
        ident_str!("signer"),
        ident_str!("string"),
        ident_str!("vector"),
        ident_str!("voting"),
        ident_str!("account"),
        ident_str!("ed25519"),
        ident_str!("genesis"),
        ident_str!("math128"),
        ident_str!("version"),
        ident_str!("vesting"),
        ident_str!("bls12381"),
        ident_str!("chain_id"),
        ident_str!("features"),
        ident_str!("from_bcs"),
        ident_str!("pool_u64"),
        ident_str!("secp256k1"),
        ident_str!("timestamp"),
        ident_str!("type_info"),
        ident_str!("aggregator"),
        ident_str!("aptos_coin"),
        ident_str!("aptos_hash"),
        ident_str!("big_vector"),
        ident_str!("bit_vector"),
        ident_str!("capability"),
        ident_str!("comparator"),
        ident_str!("math_fixed"),
        ident_str!("randomness"),
        ident_str!("simple_map"),
        ident_str!("smart_table"),
        ident_str!("storage_gas"),
        ident_str!("chain_status"),
        ident_str!("copyable_any"),
        ident_str!("gas_schedule"),
        ident_str!("managed_coin"),
        ident_str!("math_fixed64"),
        ident_str!("ristretto255"),
        ident_str!("smart_vector"),
        ident_str!("string_utils"),
        ident_str!("aggregator_v2"),
        ident_str!("aptos_account"),
        ident_str!("bn254_algebra"),
        ident_str!("config_buffer"),
        ident_str!("create_signer"),
        ident_str!("fixed_point32"),
        ident_str!("fixed_point64"),
        ident_str!("function_info"),
        ident_str!("multi_ed25519"),
        ident_str!("staking_proxy"),
        ident_str!("state_storage"),
        ident_str!("crypto_algebra"),
        ident_str!("fungible_asset"),
        ident_str!("staking_config"),
        ident_str!("delegation_pool"),
        ident_str!("keyless_account"),
        ident_str!("reconfiguration"),
        ident_str!("transaction_fee"),
        ident_str!("aptos_governance"),
        ident_str!("bls12381_algebra"),
        ident_str!("consensus_config"),
        ident_str!("execution_config"),
        ident_str!("multisig_account"),
        ident_str!("pool_u64_unbound"),
        ident_str!("resource_account"),
        ident_str!("staking_contract"),
        ident_str!("system_addresses"),
        ident_str!("randomness_config"),
        ident_str!("table_with_length"),
        ident_str!("aggregator_factory"),
        ident_str!("governance_proposal"),
        ident_str!("optional_aggregator"),
        ident_str!("transaction_context"),
        ident_str!("jwk_consensus_config"),
        ident_str!("ristretto255_elgamal"),
        ident_str!("reconfiguration_state"),
        ident_str!("ristretto255_pedersen"),
        ident_str!("object_code_deployment"),
        ident_str!("primary_fungible_store"),
        ident_str!("transaction_validation"),
        ident_str!("randomness_api_v0_config"),
        ident_str!("randomness_config_seqnum"),
        ident_str!("reconfiguration_with_dkg"),
        ident_str!("validator_consensus_info"),
        ident_str!("ristretto255_bulletproofs"),
        ident_str!("dispatchable_fungible_asset"),
    ]
    .into_iter()
    .map(|module| StateKey::module(&AccountAddress::ONE, module))
    .collect::<Vec<_>>();

    assert_vec_eq(&code, &expected_code);

    let expected_resources = vec![
        (false, "0x1::dkg::DKGState"),
        (false, "0x1::jwks::Patches"),
        (false, "0x1::account::Account"),
        (false, "0x1::version::Version"),
        (false, "0x1::jwks::PatchedJWKs"),
        (false, "0x1::chain_id::ChainId"),
        (false, "0x1::jwks::ObservedJWKs"),
        (false, "0x1::features::Features"),
        (false, "0x1::stake::ValidatorSet"),
        (false, "0x1::block::BlockResource"),
        (false, "0x1::block::CommitHistory"),
        (false, "0x1::code::PackageRegistry"),
        (true, "0x1::keyless_account::Group"),
        (false, "0x1::coin::CoinConversionMap"),
        (false, "0x1::storage_gas::StorageGas"),
        (false, "0x1::stake::ValidatorPerformance"),
        (false, "0x1::account::OriginatingAddress"),
        (false, "0x1::gas_schedule::GasScheduleV2"),
        (false, "0x1::jwks::SupportedOIDCProviders"),
        (false, "0x1::stake::AptosCoinCapabilities"),
        (false, "0x1::reconfiguration_state::State"),
        (false, "0x1::version::SetVersionCapability"),
        (false, "0x1::storage_gas::StorageGasConfig"),
        (false, "0x1::config_buffer::PendingConfigs"),
        (false, "0x1::staking_config::StakingConfig"),
        (false, "0x1::randomness::PerBlockRandomness"),
        (false, "0x1::chain_status::GenesisEndMarker"),
        (false, "0x1::reconfiguration::Configuration"),
        (false, "0x1::aptos_governance::VotingRecords"),
        (false, "0x1::state_storage::StateStorageUsage"),
        (false, "0x1::consensus_config::ConsensusConfig"),
        (false, "0x1::execution_config::ExecutionConfig"),
        (false, "0x1::timestamp::CurrentTimeMicroseconds"),
        (false, "0x1::aptos_governance::GovernanceConfig"),
        (false, "0x1::aptos_governance::GovernanceEvents"),
        (false, "0x1::randomness_config::RandomnessConfig"),
        (false, "0x1::aggregator_factory::AggregatorFactory"),
        (false, "0x1::transaction_fee::AptosCoinCapabilities"),
        (false, "0x1::transaction_fee::AptosCoinMintCapability"),
        (false, "0x1::jwk_consensus_config::JWKConsensusConfig"),
        (false, "0x1::aptos_governance::ApprovedExecutionHashes"),
        (false, "0x1::aptos_governance::GovernanceResponsbility"),
        (false, "0x1::randomness_api_v0_config::RequiredGasDeposit"),
        (false, "0x1::transaction_validation::TransactionValidation"),
        (
            false,
            "0x1::randomness_api_v0_config::AllowCustomMaxGasFlag",
        ),
        (
            false,
            "0x1::randomness_config_seqnum::RandomnessConfigSeqNum",
        ),
        (false, "0x1::coin::CoinInfo<0x1::aptos_coin::AptosCoin>"),
        (
            false,
            "0x1::voting::VotingForum<0x1::governance_proposal::GovernanceProposal>",
        ),
    ]
    .into_iter()
    .map(|(rg, struct_tag)| {
        if rg {
            StateKey::resource_group(
                &AccountAddress::ONE,
                &StructTag::from_str(struct_tag).unwrap(),
            )
        } else {
            StateKey::resource(
                &AccountAddress::ONE,
                &StructTag::from_str(struct_tag).unwrap(),
            )
            .unwrap()
        }
    })
    .collect::<Vec<_>>();

    assert_vec_eq(&resources, &expected_resources);
}

fn assert_vec_eq<T: Eq + Debug>(left: &[T], right: &[T]) {
    for i in 0..left.len().min(right.len()) {
        assert_eq!(left[i], right[i], "difference at position {}", i);
    }
    assert_eq!(left.len(), right.len(), "difference at last element");
}
