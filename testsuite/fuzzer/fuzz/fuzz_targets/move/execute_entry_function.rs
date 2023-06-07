// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use move_binary_format::file_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::TypeTag,
};
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::{gas_schedule::GasStatus, InMemoryStorage};

#[derive(Arbitrary, Debug)]
struct FuzzData {
    cm: CompiledModule,
    ident: String,
    ty_arg: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
    account_address: AccountAddress,
}

fuzz_target!(|fuzz_data: FuzzData| {
    let mut cm_serialized = Vec::with_capacity(65536);
    if fuzz_data.cm.serialize(&mut cm_serialized).is_err() {
        return;
    }

    if move_bytecode_verifier::verify_module(&fuzz_data.cm).is_err() {
        return;
    }

    let vm = MoveVM::new(vec![]).unwrap();
    let storage = InMemoryStorage::new();
    let mut session = vm.new_session(&storage);
    let mut gas = GasStatus::new_unmetered();

    if session
        .publish_module(cm_serialized, fuzz_data.account_address, &mut gas)
        .is_err()
    {
        return;
    }

    let ident =
        IdentStr::new(fuzz_data.ident.as_str()).unwrap_or_else(|_| IdentStr::new("f").unwrap());
    let _ = session.execute_entry_function(
        &fuzz_data.cm.self_id(),
        ident,
        fuzz_data.ty_arg,
        fuzz_data.args,
        &mut gas,
    );
});
