// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{current_function_name, executor::FakeExecutor};
use aptos_types::{
    account_address::AccountAddress,
    account_config,
    transaction::{ExecutionStatus, Script, TransactionStatus},
};
use move_binary_format::file_format::{
    empty_script, AddressIdentifierIndex, Bytecode, FunctionHandle, FunctionHandleIndex,
    FunctionInstantiation, FunctionInstantiationIndex, IdentifierIndex, ModuleHandle,
    ModuleHandleIndex, Signature, SignatureIndex, SignatureToken,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    vm_status::{StatusCode, StatusCode::LINKER_ERROR},
};

#[test]
fn script_code_unverifiable() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // create a bogus script
    let mut script = empty_script();
    script.code.code = vec![Bytecode::LdU8(0), Bytecode::Add, Bytecode::Ret];
    let mut blob = vec![];
    script.serialize(&mut blob).expect("script must serialize");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(blob, vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price(1)
        .sign();
    // execute transaction
    let output = &executor.execute_transaction(txn);
    let status = output.status();
    match status {
        TransactionStatus::Keep(_) => (),
        _ => panic!("TransactionStatus must be Keep"),
    }
    assert_eq!(
        status.status(),
        // StatusCode::NEGATIVE_STACK_SIZE_WITHIN_BLOCK
        Ok(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::NEGATIVE_STACK_SIZE_WITHIN_BLOCK
        )))
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let gas = output.gas_used();
    let balance = 1_000_000 - gas;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_coin_store_resource(sender.account())
        .expect("sender balance must exist");
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());
}

#[test]
fn script_none_existing_module_dep() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // create a bogus script
    let mut script = empty_script();

    // make a non existent external module
    script
        .address_identifiers
        .push(AccountAddress::new([2u8; AccountAddress::LENGTH]));
    script.identifiers.push(Identifier::new("module").unwrap());
    let module_handle = ModuleHandle {
        address: AddressIdentifierIndex((script.address_identifiers.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
    };
    script.module_handles.push(module_handle);
    // make a non existent function on the non existent external module
    script.identifiers.push(Identifier::new("foo").unwrap());
    let fun_handle = FunctionHandle {
        module: ModuleHandleIndex((script.module_handles.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
    };
    script.function_handles.push(fun_handle);

    script.code.code = vec![
        Bytecode::Call(FunctionHandleIndex(
            (script.function_handles.len() - 1) as u16,
        )),
        Bytecode::Ret,
    ];
    let mut blob = vec![];
    script.serialize(&mut blob).expect("script must serialize");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(blob, vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price(1)
        .sign();

    // execute transaction
    let output = &executor.execute_transaction(txn);
    let status = output.status();
    match status {
        TransactionStatus::Keep(_) => (),
        _ => panic!("TransactionStatus must be Keep"),
    }
    assert_eq!(
        status.status(),
        Ok(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::LINKER_ERROR
        ))),
        "Linker Error: Transaction executed at a non-existent external module"
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let gas = output.gas_used();
    let balance = 1_000_000 - gas;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_coin_store_resource(sender.account())
        .expect("sender balance must exist");
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());
}

#[test]
fn script_non_existing_function_dep() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // create a bogus script
    let mut script = empty_script();

    // BCS module
    script
        .address_identifiers
        .push(account_config::CORE_CODE_ADDRESS);
    script.identifiers.push(Identifier::new("bcs").unwrap());
    let module_handle = ModuleHandle {
        address: AddressIdentifierIndex((script.address_identifiers.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
    };
    script.module_handles.push(module_handle);
    // make a non existent function on BCS
    script.identifiers.push(Identifier::new("foo").unwrap());
    let fun_handle = FunctionHandle {
        module: ModuleHandleIndex((script.module_handles.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
    };
    script.function_handles.push(fun_handle);

    script.code.code = vec![
        Bytecode::Call(FunctionHandleIndex(
            (script.function_handles.len() - 1) as u16,
        )),
        Bytecode::Ret,
    ];
    let mut blob = vec![];
    script.serialize(&mut blob).expect("script must serialize");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(blob, vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price(1)
        .sign();

    // execute transaction
    let output = &executor.execute_transaction(txn);
    let status = output.status();
    match status {
        TransactionStatus::Keep(_) => (),
        _ => panic!("TransactionStatus must be Keep"),
    }
    assert_eq!(
        status.status(),
        // StatusCode::LOOKUP_FAILED
        Ok(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::LOOKUP_FAILED
        )))
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let gas = output.gas_used();
    let balance = 1_000_000 - gas;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_coin_store_resource(sender.account())
        .expect("sender balance must exist");
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());
}

#[test]
fn script_bad_sig_function_dep() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // create a bogus script
    let mut script = empty_script();

    // BCS module
    script
        .address_identifiers
        .push(account_config::CORE_CODE_ADDRESS);
    script.identifiers.push(Identifier::new("bcs").unwrap());
    let module_handle = ModuleHandle {
        address: AddressIdentifierIndex((script.address_identifiers.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
    };
    script.module_handles.push(module_handle);
    // BCS::to_bytes with bad sig
    script
        .identifiers
        .push(Identifier::new("to_bytes").unwrap());
    let fun_handle = FunctionHandle {
        module: ModuleHandleIndex((script.module_handles.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
    };
    script.function_handles.push(fun_handle);

    script.code.code = vec![
        Bytecode::Call(FunctionHandleIndex(
            (script.function_handles.len() - 1) as u16,
        )),
        Bytecode::Ret,
    ];
    let mut blob = vec![];
    script.serialize(&mut blob).expect("script must serialize");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(blob, vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price(1)
        .sign();
    // execute transaction
    let output = &executor.execute_transaction(txn);
    let status = output.status();
    match status {
        TransactionStatus::Keep(_) => (),
        _ => panic!("TransactionStatus must be Keep"),
    }
    assert_eq!(
        status.status(),
        // StatusCode::TYPE_MISMATCH
        Ok(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::TYPE_MISMATCH
        )))
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let gas = output.gas_used();
    let balance = 1_000_000 - gas;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_coin_store_resource(sender.account())
        .expect("sender balance must exist");
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());
}

#[test]
fn script_type_argument_module_does_not_exist() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // create a bogus script
    let mut script = empty_script();

    // make a non existent external module
    let address = AccountAddress::new([2u8; AccountAddress::LENGTH]);
    let module = Identifier::new("module").unwrap();
    script.address_identifiers.push(address);
    script.identifiers.push(module.clone());
    let module_handle = ModuleHandle {
        address: AddressIdentifierIndex((script.address_identifiers.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
    };
    script.module_handles.push(module_handle);
    script.code.code = vec![Bytecode::Ret];
    script.type_parameters = vec![AbilitySet::EMPTY];
    let mut blob = vec![];
    script.serialize(&mut blob).expect("script must serialize");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(
            blob,
            vec![TypeTag::Struct(Box::new(StructTag {
                address,
                module,
                name: Identifier::new("fake").unwrap(),
                type_args: vec![],
            }))],
            vec![],
        ))
        .sequence_number(10)
        .gas_unit_price(1)
        .sign();

    // execute transaction
    let output = &executor.execute_transaction(txn);
    let status = output.status();
    assert_eq!(
        status,
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(LINKER_ERROR))),
        "Linker Error: Transaction executed at a non-existent external module"
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let gas = output.gas_used();
    let balance = 1_000_000 - gas;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_coin_store_resource(sender.account())
        .expect("sender balance must exist");
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());
}

#[test]
fn script_nested_type_argument_module_does_not_exist() {
    let mut executor = FakeExecutor::from_head_genesis();
    executor.set_golden_file(current_function_name!());

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // create a bogus script
    let mut script = empty_script();

    // make a non existent external module
    let address = AccountAddress::new([2u8; AccountAddress::LENGTH]);
    let module = Identifier::new("module").unwrap();
    script.address_identifiers.push(address);
    script.identifiers.push(module.clone());
    let module_handle = ModuleHandle {
        address: AddressIdentifierIndex((script.address_identifiers.len() - 1) as u16),
        name: IdentifierIndex((script.identifiers.len() - 1) as u16),
    };
    script.module_handles.push(module_handle);
    script.code.code = vec![Bytecode::Ret];
    script.type_parameters = vec![AbilitySet::EMPTY];
    let mut blob = vec![];
    script.serialize(&mut blob).expect("script must serialize");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(
            blob,
            vec![TypeTag::Vector(Box::new(TypeTag::Struct(Box::new(
                StructTag {
                    address,
                    module,
                    name: Identifier::new("fake").unwrap(),
                    type_args: vec![],
                },
            ))))],
            vec![],
        ))
        .sequence_number(10)
        .gas_unit_price(1)
        .sign();

    // execute transaction
    let output = &executor.execute_transaction(txn);
    let status = output.status();
    assert_eq!(
        status,
        &TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(LINKER_ERROR))),
        "Linker Error: Transaction executed at a non-existent external module"
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let gas = output.gas_used();
    let balance = 1_000_000 - gas;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_coin_store_resource(sender.account())
        .expect("sender balance must exist");
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());
}

#[test]
fn forbid_script_emitting_events() {
    let mut executor = FakeExecutor::from_head_genesis();

    // create and publish sender
    let sender = executor.create_raw_account_data(1_000_000, 10);
    executor.add_account_data(&sender);

    // create an event-emitting script
    let mut script = empty_script();
    script.code.code = vec![
        Bytecode::LdTrue,
        Bytecode::CallGeneric(FunctionInstantiationIndex(0)),
        Bytecode::Ret,
    ];
    script.function_instantiations.push(FunctionInstantiation {
        handle: FunctionHandleIndex(0),
        type_parameters: SignatureIndex(2),
    });
    script.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        parameters: SignatureIndex(1),
        return_: SignatureIndex(0),
        type_parameters: vec![
            AbilitySet::singleton(Ability::Store) | AbilitySet::singleton(Ability::Drop),
        ],
        access_specifiers: None,
    });
    script.module_handles.push(ModuleHandle {
        address: AddressIdentifierIndex(0),
        name: IdentifierIndex(0),
    });
    script.address_identifiers.push(AccountAddress::ONE);
    script.identifiers = vec![
        Identifier::new("event").unwrap(),
        Identifier::new("emit").unwrap(),
    ];
    // dummy signatures
    script.signatures = vec![
        Signature(vec![]),
        Signature(vec![SignatureToken::TypeParameter(0)]),
        Signature(vec![SignatureToken::Bool]),
    ];
    let mut blob = vec![];
    script.serialize(&mut blob).expect("script must serialize");
    let txn = sender
        .account()
        .transaction()
        .script(Script::new(blob, vec![], vec![]))
        .sequence_number(10)
        .gas_unit_price(1)
        .sign();
    // execute transaction
    let output = &executor.execute_transaction(txn);
    let status = output.status();
    match status {
        TransactionStatus::Keep(_) => (),
        _ => panic!("TransactionStatus must be Keep"),
    }
    assert_eq!(
        status.status(),
        Ok(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::INVALID_OPERATION_IN_SCRIPT
        )))
    );
    executor.apply_write_set(output.write_set());

    // Check that numbers in store are correct.
    let gas = output.gas_used();
    let balance = 1_000_000 - gas;
    let updated_sender = executor
        .read_account_resource(sender.account())
        .expect("sender must exist");
    let updated_sender_balance = executor
        .read_apt_coin_store_resource(sender.account())
        .expect("sender balance must exist");
    assert_eq!(balance, updated_sender_balance.coin());
    assert_eq!(11, updated_sender.sequence_number());
}
