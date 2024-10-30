// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use aptos_language_e2e_tests::{
    account::AccountData, compile::compile_script, current_function_name, executor::FakeExecutor,
    feature_flags_for_orderless,
};
use aptos_transaction_simulation::SimulationStateStore;
use aptos_types::transaction::{
    ExecutionStatus, SignedTransaction, Transaction, TransactionStatus,
};
use claims::assert_matches;
use move_binary_format::CompiledModule;
use move_bytecode_verifier::verify_module;
use move_ir_compiler::Compiler;
use rstest::rstest;

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn move_from_across_blocks(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    // publish module with add and remove resource
    let module = add_module(executor.state_store(), &sender);

    // remove resource fails given no resource were published
    let rem_txn = remove_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(0)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource
    let add_txn = add_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(1)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(add_txn);

    // borrow resource
    let borrow_txn = borrow_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(2)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(borrow_txn);

    // remove resource
    let rem_txn = remove_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(3)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(rem_txn);

    // remove resource fails given it was removed already
    let rem_txn = remove_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(4)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // borrow resource fail given it was removed
    let borrow_txn = borrow_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(5)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let output = executor.execute_transaction(borrow_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource again
    let add_txn = add_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(6)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(add_txn);

    // create 2 remove resource transaction over the same resource in one block
    let txns = vec![
        Transaction::UserTransaction(remove_resource_txn(
            &sender,
            if use_orderless_transactions {
                None
            } else {
                Some(7)
            },
            vec![module.clone()],
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )),
        Transaction::UserTransaction(remove_resource_txn(
            &sender,
            if use_orderless_transactions {
                None
            } else {
                Some(8)
            },
            vec![module],
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )),
    ];
    let output = executor
        .execute_transaction_block(txns)
        .expect("Must execute transactions");
    assert_eq!(
        output[0].status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    assert_matches!(
        output[1].status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    for out in output {
        executor.apply_write_set(out.write_set());
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn borrow_after_move(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    // publish module with add and remove resource
    let module = add_module(executor.state_store(), &sender);

    // remove resource fails given no resource were published
    let rem_txn = remove_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(0)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource
    let add_txn = add_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(1)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(add_txn);

    // borrow resource
    let borrow_txn = borrow_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(2)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(borrow_txn);

    // create a remove and a borrow resource transaction over the same resource in one block
    let txns = vec![
        Transaction::UserTransaction(remove_resource_txn(
            &sender,
            if use_orderless_transactions {
                None
            } else {
                Some(3)
            },
            vec![module.clone()],
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )),
        Transaction::UserTransaction(borrow_resource_txn(
            &sender,
            if use_orderless_transactions {
                None
            } else {
                Some(4)
            },
            vec![module],
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )),
    ];
    let output = executor
        .execute_transaction_block(txns)
        .expect("Must execute transactions");
    assert_eq!(
        output[0].status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    assert_matches!(
        output[1].status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    for out in output {
        executor.apply_write_set(out.write_set());
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn change_after_move(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.enable_features(
        feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
        vec![],
    );
    executor.set_golden_file(current_function_name!());
    let sender =
        executor.create_raw_account_data(1_000_000, if stateless_account { None } else { Some(0) });
    executor.add_account_data(&sender);

    // publish module with add and remove resource
    let module = add_module(executor.state_store(), &sender);

    // remove resource fails given no resource were published
    let rem_txn = remove_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(0)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource
    let add_txn = add_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(1)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(add_txn);

    // borrow resource
    let borrow_txn = borrow_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(2)
        },
        vec![module.clone()],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    executor.execute_and_apply(borrow_txn);

    // create a remove and a change resource transaction over the same resource in one block
    let txns = vec![
        Transaction::UserTransaction(remove_resource_txn(
            &sender,
            if use_orderless_transactions {
                None
            } else {
                Some(3)
            },
            vec![module.clone()],
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )),
        Transaction::UserTransaction(change_resource_txn(
            &sender,
            if use_orderless_transactions {
                None
            } else {
                Some(4)
            },
            vec![module.clone()],
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )),
    ];
    let output = executor
        .execute_transaction_block(txns)
        .expect("Must execute transactions");
    assert_eq!(
        output[0].status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
    assert_matches!(
        output[1].status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    for out in output {
        executor.apply_write_set(out.write_set());
    }

    // borrow resource
    let borrow_txn = borrow_resource_txn(
        &sender,
        if use_orderless_transactions {
            None
        } else {
            Some(5)
        },
        vec![module],
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let output = executor.execute_transaction(borrow_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());
}

fn add_module(state_store: &impl SimulationStateStore, sender: &AccountData) -> CompiledModule {
    let code = format!(
        "
        module 0x{}.M {{
            import 0x1.signer;
            struct T1 has key {{ v: u64 }}

            public borrow_t1(account: &signer) acquires T1 {{
                let t1: &Self.T1;
            label b0:
                t1 = borrow_global<T1>(signer.address_of(move(account)));
                return;
            }}

            public change_t1(account: &signer, v: u64) acquires T1 {{
                let t1: &mut Self.T1;
            label b0:
                t1 = borrow_global_mut<T1>(signer.address_of(move(account)));
                *&mut move(t1).T1::v = move(v);
                return;
            }}

            public remove_t1(account: &signer) acquires T1 {{
                let v: u64;
            label b0:
                T1 {{ v }} = move_from<T1>(signer.address_of(move(account)));
                return;
            }}

            public publish_t1(account: &signer) {{
            label b0:
                move_to<T1>(move(account), T1 {{ v: 3 }});
                return;
            }}
        }}
        ",
        sender.address().to_hex(),
    );

    let framework_modules = aptos_cached_packages::head_release_bundle().compiled_modules();
    let compiler = Compiler {
        deps: framework_modules.iter().collect(),
    };
    let module = compiler
        .into_compiled_module(code.as_str())
        .expect("Module compilation failed");
    verify_module(&module).expect("Module must verify");

    let mut module_bytes = vec![];
    module
        .serialize(&mut module_bytes)
        .expect("Module must serialize");

    state_store
        .add_module_blob(&module.self_id(), module_bytes)
        .expect("should be able to add module");
    module
}

fn add_resource_txn(
    sender: &AccountData,
    seq_num: Option<u64>,
    extra_deps: Vec<CompiledModule>,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    let program = format!(
        "
            import 0x{}.M;

            main(account: signer) {{
            label b0:
                M.publish_t1(&account);
                return;
            }}
        ",
        sender.address().to_hex(),
    );

    let script = compile_script(&program, extra_deps);
    let seq_num = if use_orderless_transactions {
        u64::MAX
    } else {
        seq_num.unwrap()
    };
    sender
        .account()
        .transaction()
        .script(script)
        .sequence_number(seq_num)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign()
}

fn remove_resource_txn(
    sender: &AccountData,
    seq_num: Option<u64>,
    extra_deps: Vec<CompiledModule>,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    let program = format!(
        "
            import 0x{}.M;

            main(account: signer) {{
            label b0:
                M.remove_t1(&account);
                return;
            }}
        ",
        sender.address().to_hex(),
    );

    let module = compile_script(&program, extra_deps);
    let seq_num = if use_orderless_transactions {
        u64::MAX
    } else {
        seq_num.unwrap()
    };
    sender
        .account()
        .transaction()
        .script(module)
        .sequence_number(seq_num)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign()
}

fn borrow_resource_txn(
    sender: &AccountData,
    seq_num: Option<u64>,
    extra_deps: Vec<CompiledModule>,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    let program = format!(
        "
            import 0x{}.M;

            main(account: signer) {{
            label b0:
                M.borrow_t1(&account);
                return;
            }}
        ",
        sender.address().to_hex(),
    );

    let module = compile_script(&program, extra_deps);
    let seq_num = if use_orderless_transactions {
        u64::MAX
    } else {
        seq_num.unwrap()
    };
    sender
        .account()
        .transaction()
        .script(module)
        .sequence_number(seq_num)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign()
}

fn change_resource_txn(
    sender: &AccountData,
    seq_num: Option<u64>,
    extra_deps: Vec<CompiledModule>,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    let program = format!(
        "
            import 0x{}.M;

            main(account: signer) {{
            label b0:
                M.change_t1(&account, 20);
                return;
            }}
        ",
        sender.address().to_hex(),
    );

    let module = compile_script(&program, extra_deps);
    let seq_num = if use_orderless_transactions {
        u64::MAX
    } else {
        seq_num.unwrap()
    };
    sender
        .account()
        .transaction()
        .script(module)
        .sequence_number(seq_num)
        .upgrade_payload(use_txn_payload_v2_format, use_orderless_transactions)
        .sign()
}
