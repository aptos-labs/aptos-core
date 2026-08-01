// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Regression test: the transaction status must not depend on struct-name
//! interning order.

use crate::{assert_success, tests::common, MoveHarness};
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManager, txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_language_e2e_tests::account::Account;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::StateView,
    transaction::{
        signature_verified_transaction::into_signature_verified_block, ExecutionStatus,
        SignedTransaction, Transaction, TransactionOutput, TransactionStatus,
    },
    vm_status::VMStatus,
};
use aptos_vm::block_executor::AptosVMBlockExecutorWrapper;
use move_core_types::vm_status::StatusCode;

fn agg_source(addr: &str) -> String {
    // There are at most 10 aggregators per resource, so use 11 to force the
    // serialization to fail.
    const NUM_AGGS: usize = 11;

    let mut s = String::new();
    s.push_str(&format!("module {}::agg {{\n", addr));
    s.push_str("    use aptos_framework::aggregator_v2::{Self, Aggregator};\n");
    s.push_str("    struct A has key {\n");
    for i in 0..NUM_AGGS {
        s.push_str(&format!("        a{}: Aggregator<u64>,\n", i));
    }
    s.push_str("    }\n");
    s.push_str("    public fun make(s: &signer) {\n        move_to(s, A {\n");
    for i in 0..NUM_AGGS {
        s.push_str(&format!(
            "            a{}: aggregator_v2::create_unbounded_aggregator<u64>(),\n",
            i
        ));
    }
    s.push_str("        })\n    }\n");
    s.push_str("}\n");
    s
}

fn clo_source(addr: &str) -> String {
    // Using 11 here to trigger serialization failure because layout of the struct
    // is too large.
    const N: usize = 11;

    let mut s = String::new();
    s.push_str(&format!("module {}::clo {{\n", addr));
    s.push_str("    struct S0 has copy, drop, store { v: u8 }\n");
    for i in 0..N {
        s.push_str(&format!(
            "    struct S{} has copy, drop, store {{ l: S{}, r: S{} }}\n",
            i + 1,
            i,
            i
        ));
    }
    s.push_str("    struct Work has copy, drop, store, key { bar: || }\n");
    s.push_str(&format!(
        "    #[persistent] fun consumer(_o: vector<S{}>) {{}}\n",
        N
    ));
    s.push_str(&format!(
        "    public fun make(s: &signer) {{\n        \
                 let o: vector<S{}> = vector[];\n        \
                 move_to(s, Work {{ bar: || consumer(o) }})\n    \
             }}\n",
        N
    ));
    s.push_str("    public entry fun touch(_s: &signer) {}\n");
    s.push_str("}\n");
    s
}

fn setup() -> (MoveHarness, Account) {
    let addr = "0xcafe";
    let mut builder = PackageBuilder::new("P");
    builder.add_source("agg.move", &agg_source(addr));
    builder.add_source("clo.move", &clo_source(addr));

    let driver_source = format!(
        "module {addr}::drv {{\n    \
             use {addr}::agg;\n    \
             use {addr}::clo;\n    \
             public entry fun agg_then_clo(s: &signer) {{ agg::make(s); clo::make(s) }}\n\
         }}\n",
    );
    builder.add_source("drv.move", &driver_source);

    for (name, path) in [
        ("MoveStdlib", "move-stdlib"),
        ("AptosStdlib", "aptos-stdlib"),
        ("AptosFramework", "aptos-framework"),
    ] {
        builder.add_local_dep(name, &common::framework_dir_path(path).to_string_lossy());
    }
    let path = builder.write_to_temp().unwrap();

    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal(addr).unwrap());
    assert_success!(h.publish_package(&acc, path.path()));
    (h, acc)
}

fn execute_block(
    txn: SignedTransaction,
    state_view: &(impl StateView + Sync),
    manager: &AptosModuleCacheManager,
    parent: u64,
    child: u64,
) -> TransactionOutput {
    let block = into_signature_verified_block(vec![Transaction::UserTransaction(txn)]);
    let txn_provider = DefaultTxnProvider::new_without_info(block);
    AptosVMBlockExecutorWrapper::execute_block::<_, NoOpTransactionCommitHook<VMStatus>, _>(
        &txn_provider,
        state_view,
        manager,
        BlockExecutorConfig::new_no_block_limit(1),
        TransactionSliceMetadata::block(
            aptos_crypto::HashValue::from_u64(parent),
            aptos_crypto::HashValue::from_u64(child),
        ),
        None,
    )
    .unwrap()
    .into_transaction_outputs_forced()
    .first()
    .unwrap()
    .clone()
}

/// Same state, same block, only interner warmth differs. Every output field must
/// agree, and the status must be the `StructTag`-first failure. Before the fix
/// the statuses diverged.
#[test]
fn cold_vs_warm_status_matches() {
    let (mut h, acc) = setup();
    let manager = AptosModuleCacheManager::new();

    // Block 1: Runs a transaction to warm up the cache.
    let sender = h.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
    let warming_txn = h.create_entry_function(
        &sender,
        str::parse("0xcafe::clo::touch").unwrap(),
        vec![],
        vec![],
    );
    let outs = execute_block(warming_txn, h.executor.get_state_view(), &manager, 1, 2);
    assert_success!(outs.status().clone());
    h.executor.apply_write_set(outs.write_set());

    // Transaction that touches and serializes resources in both modules.
    let target = h.create_entry_function(
        &acc,
        str::parse("0xcafe::drv::agg_then_clo").unwrap(),
        vec![],
        vec![],
    );

    // Warm: still has `clo` data in the cache from the previous block.
    let warm = execute_block(target.clone(), h.executor.get_state_view(), &manager, 2, 3);

    // Cold: run with no caches.
    let cold = execute_block(
        target,
        h.executor.get_state_view(),
        &AptosModuleCacheManager::new(),
        2,
        3,
    );

    assert_eq!(cold.status(), warm.status());
    assert_eq!(
        cold.status().clone(),
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::TOO_MANY_DELAYED_FIELDS,
        ))),
    );
}
