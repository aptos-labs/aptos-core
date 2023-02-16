// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_types::{
    access_path::AccessPath,
    account_config::CORE_CODE_ADDRESS,
    chain_id::{ChainId, NamedChain},
    on_chain_config::TimedFeatureFlag,
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, Script, TransactionStatus},
};
use move_binary_format::file_format::{empty_script, Bytecode};
use move_core_types::{identifier::Identifier, language_storage::StructTag, vm_status::StatusCode};

fn offending_script() -> Vec<u8> {
    use Bytecode::*;

    let mut code = vec![];
    for _i in 0..50 {
        code.push(LdU8(0));
        code.push(LdU8(1));
        code.push(Eq);
        code.push(BrTrue(0));
    }
    code.push(Ret);

    let mut script = empty_script();
    script.code.code = code;

    let mut bytes = vec![];
    script.serialize(&mut bytes).expect("script must serialize");

    bytes
}

fn test_script(chain_id: ChainId, time: u64) {
    let mut executor = FakeExecutor::from_head_genesis();

    executor.write_state_value(
        StateKey::access_path(
            AccessPath::resource_access_path(CORE_CODE_ADDRESS, StructTag {
                address: CORE_CODE_ADDRESS,
                module: Identifier::new("chain_id").unwrap(),
                name: Identifier::new("ChainId").unwrap(),
                type_params: vec![],
            })
            .expect("access path in test"),
        ),
        bcs::to_bytes(&chain_id).unwrap(),
    );

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // run script before the stricter rules take effect
    executor.new_block_with_timestamp(time - 1);
    executor.exec("reconfiguration", "reconfigure", vec![], vec![]);

    let txn = sender
        .account()
        .transaction()
        .chain_id(chain_id)
        .script(Script::new(offending_script(), vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price(1)
        .ttl(5_000_000)
        .sign();

    let output = &executor.execute_transaction(txn);
    match output.status() {
        TransactionStatus::Keep(status) => {
            assert!(status.is_success());
        },
        _ => panic!("TransactionStatus must be Keep"),
    }

    // run script after the stricter rules take effect
    executor.new_block_with_timestamp(time);
    executor.exec("reconfiguration", "reconfigure", vec![], vec![]);

    let txn = sender
        .account()
        .transaction()
        .chain_id(chain_id)
        .script(Script::new(offending_script(), vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price(1)
        .ttl(5_000_000)
        .sign();

    let output = &executor.execute_transaction(txn);
    match output.status() {
        TransactionStatus::Keep(status) => {
            assert!(
                matches!(
                    status,
                    ExecutionStatus::MiscellaneousError(Some(StatusCode::TOO_MANY_BACK_EDGES))
                ),
                "{:?}",
                status
            );
        },
        _ => panic!("TransactionStatus must be Keep"),
    }
}

#[test]
fn script_too_many_back_edges_testnet() {
    test_script(
        ChainId::testnet(),
        TimedFeatureFlag::VerifierLimitBackEdges.activation_time_on(&NamedChain::TESTNET),
    )
}
