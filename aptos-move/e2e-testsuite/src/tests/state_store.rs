// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_language_e2e_tests::{
    account::AccountData, compile::compile_script, current_function_name, executor::FakeExecutor,
};
use aptos_transaction_simulation::SimulationStateStore;
use aptos_types::transaction::{
    ExecutionStatus, SignedTransaction, Transaction, TransactionStatus,
};
use claims::assert_matches;
use indoc::formatdoc;
use move_asm::assembler;
use move_binary_format::CompiledModule;
use move_bytecode_verifier::verify_module;

#[test]
fn move_from_across_blocks() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());
    let sender = executor.create_raw_account_data(1_000_000, 11);
    executor.add_account_data(&sender);

    // publish module with add and remove resource
    let module = add_module(executor.state_store(), &sender);

    // remove resource fails given no resource were published
    let rem_txn = remove_resource_txn(&sender, 11, vec![module.clone()]);
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource
    let add_txn = add_resource_txn(&sender, 12, vec![module.clone()]);
    executor.execute_and_apply(add_txn);

    // borrow resource
    let borrow_txn = borrow_resource_txn(&sender, 13, vec![module.clone()]);
    executor.execute_and_apply(borrow_txn);

    // remove resource
    let rem_txn = remove_resource_txn(&sender, 14, vec![module.clone()]);
    executor.execute_and_apply(rem_txn);

    // remove resource fails given it was removed already
    let rem_txn = remove_resource_txn(&sender, 15, vec![module.clone()]);
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // borrow resource fail given it was removed
    let borrow_txn = borrow_resource_txn(&sender, 16, vec![module.clone()]);
    let output = executor.execute_transaction(borrow_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource again
    let add_txn = add_resource_txn(&sender, 17, vec![module.clone()]);
    executor.execute_and_apply(add_txn);

    // create 2 remove resource transaction over the same resource in one block
    let txns = vec![
        Transaction::UserTransaction(remove_resource_txn(&sender, 18, vec![module.clone()])),
        Transaction::UserTransaction(remove_resource_txn(&sender, 19, vec![module])),
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

#[test]
fn borrow_after_move() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());
    let sender = executor.create_raw_account_data(1_000_000, 11);
    executor.add_account_data(&sender);

    // publish module with add and remove resource
    let module = add_module(executor.state_store(), &sender);

    // remove resource fails given no resource were published
    let rem_txn = remove_resource_txn(&sender, 11, vec![module.clone()]);
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource
    let add_txn = add_resource_txn(&sender, 12, vec![module.clone()]);
    executor.execute_and_apply(add_txn);

    // borrow resource
    let borrow_txn = borrow_resource_txn(&sender, 13, vec![module.clone()]);
    executor.execute_and_apply(borrow_txn);

    // create a remove and a borrow resource transaction over the same resource in one block
    let txns = vec![
        Transaction::UserTransaction(remove_resource_txn(&sender, 14, vec![module.clone()])),
        Transaction::UserTransaction(borrow_resource_txn(&sender, 15, vec![module])),
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

#[test]
fn change_after_move() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());
    let sender = executor.create_raw_account_data(1_000_000, 11);
    executor.add_account_data(&sender);

    // publish module with add and remove resource
    let module = add_module(executor.state_store(), &sender);

    // remove resource fails given no resource were published
    let rem_txn = remove_resource_txn(&sender, 11, vec![module.clone()]);
    let output = executor.execute_transaction(rem_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());

    // publish resource
    let add_txn = add_resource_txn(&sender, 12, vec![module.clone()]);
    executor.execute_and_apply(add_txn);

    // borrow resource
    let borrow_txn = borrow_resource_txn(&sender, 13, vec![module.clone()]);
    executor.execute_and_apply(borrow_txn);

    // create a remove and a change resource transaction over the same resource in one block
    let txns = vec![
        Transaction::UserTransaction(remove_resource_txn(&sender, 14, vec![module.clone()])),
        Transaction::UserTransaction(change_resource_txn(&sender, 15, vec![module.clone()])),
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
    let borrow_txn = borrow_resource_txn(&sender, 16, vec![module]);
    let output = executor.execute_transaction(borrow_txn);
    assert_matches!(
        output.status().status(),
        // StatusCode::MISSING_DATA
        Ok(ExecutionStatus::ExecutionFailure { .. })
    );
    executor.apply_write_set(output.write_set());
}

fn add_module(state_store: &impl SimulationStateStore, sender: &AccountData) -> CompiledModule {
    let code = formatdoc!(
        "
        module 0x{}::M
        use 0x1::signer
        struct T1 has key
          v: u64

        public fun borrow_t1(l0: &signer) acquires T1
            local l1: &T1
            move_loc l0
            call signer::address_of
            borrow_global T1
            st_loc l1
            ret

        public fun change_t1(l0: &signer, l1: u64) acquires T1
            local l2: &mut T1
            move_loc l0
            call signer::address_of
            mut_borrow_global T1
            st_loc l2
            move_loc l1
            move_loc l2
            mut_borrow_field T1, v
            write_ref
            ret

        public fun remove_t1(l0: &signer) acquires T1
            local l1: u64
            move_loc l0
            call signer::address_of
            move_from T1
            unpack T1
            st_loc l1
            ret

        public fun publish_t1(l0: &signer)
            move_loc l0
            ld_u64 3
            pack T1
            move_to T1
            ret
        ",
        sender.address().to_hex(),
    );

    let framework_modules = aptos_cached_packages::head_release_bundle().compiled_modules();
    let options = assembler::Options::default();
    let module = assembler::assemble(&options, &code, framework_modules.iter())
        .expect("Module assembly failed")
        .left()
        .expect("Expected module, got script");
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
    seq_num: u64,
    extra_deps: Vec<CompiledModule>,
) -> SignedTransaction {
    let program = formatdoc!(
        "
        script
        use 0x{}::M
        entry public fun main(l0: signer)
            borrow_loc l0
            call M::publish_t1
            ret
        ",
        sender.address().to_hex(),
    );

    let script = compile_script(&program, extra_deps);
    sender
        .account()
        .transaction()
        .script(script)
        .sequence_number(seq_num)
        .sign()
}

fn remove_resource_txn(
    sender: &AccountData,
    seq_num: u64,
    extra_deps: Vec<CompiledModule>,
) -> SignedTransaction {
    let program = formatdoc!(
        "
        script
        use 0x{}::M
        entry public fun main(l0: signer)
            borrow_loc l0
            call M::remove_t1
            ret
        ",
        sender.address().to_hex(),
    );

    let script = compile_script(&program, extra_deps);
    sender
        .account()
        .transaction()
        .script(script)
        .sequence_number(seq_num)
        .sign()
}

fn borrow_resource_txn(
    sender: &AccountData,
    seq_num: u64,
    extra_deps: Vec<CompiledModule>,
) -> SignedTransaction {
    let program = formatdoc!(
        "
        script
        use 0x{}::M
        entry public fun main(l0: signer)
            borrow_loc l0
            call M::borrow_t1
            ret
        ",
        sender.address().to_hex(),
    );

    let script = compile_script(&program, extra_deps);
    sender
        .account()
        .transaction()
        .script(script)
        .sequence_number(seq_num)
        .sign()
}

fn change_resource_txn(
    sender: &AccountData,
    seq_num: u64,
    extra_deps: Vec<CompiledModule>,
) -> SignedTransaction {
    let program = formatdoc!(
        "
        script
        use 0x{}::M
        entry public fun main(l0: signer)
            borrow_loc l0
            ld_u64 20
            call M::change_t1
            ret
        ",
        sender.address().to_hex(),
    );

    let script = compile_script(&program, extra_deps);
    sender
        .account()
        .transaction()
        .script(script)
        .sequence_number(seq_num)
        .sign()
}
