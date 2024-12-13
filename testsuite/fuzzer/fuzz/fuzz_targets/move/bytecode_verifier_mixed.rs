// Copyright (c) The Move Contributors
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use move_binary_format::file_format::{
    empty_module, AbilitySet, Bytecode, CodeUnit, Constant, FieldDefinition, FunctionDefinition,
    FunctionHandle, FunctionHandleIndex, IdentifierIndex, ModuleHandleIndex, Signature,
    SignatureIndex, SignatureToken,
    SignatureToken::{Address, Bool},
    StructDefinition, StructFieldInformation, StructHandle, StructHandleIndex, TypeSignature,
    Visibility,
};
use move_core_types::{account_address::AccountAddress, ident_str};
use utils::helpers::compiled_module_serde;
mod utils;

#[derive(Arbitrary, Debug)]
struct Mixed {
    code: Vec<Bytecode>,
    abilities: AbilitySet,
    param_types: Vec<SignatureToken>,
    return_type: Option<SignatureToken>,
}

fuzz_target!(|mix: Mixed| {
    let mut module = empty_module();
    module.version = 5;

    module.struct_handles.push(StructHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        abilities: mix.abilities,
        type_parameters: vec![],
    });

    let fun_handle = FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(2),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(1),
        type_parameters: vec![],
        access_specifiers: None,
    };

    module.function_handles.push(fun_handle);

    module.signatures.pop();
    module.signatures.push(Signature(mix.param_types));
    module.signatures.push(Signature(
        mix.return_type.map(|s| vec![s]).unwrap_or_default(),
    ));
    module
        .signatures
        .push(Signature(vec![Address, Bool, Address]));

    module.identifiers.extend(
        vec![
            ident_str!("zf_hello_world").into(),
            ident_str!("awldFnU18mlDKQfh6qNfBGx8X").into(),
            ident_str!("aQPwJNHyAHpvJ").into(),
            ident_str!("aT7ZphKTrKcYCwCebJySrmrKlckmnL5").into(),
            ident_str!("arYpsFa2fvrpPJ").into(),
        ]
        .into_iter(),
    );
    module.address_identifiers.push(AccountAddress::random());

    module.constant_pool.push(Constant {
        type_: Address,
        data: AccountAddress::ZERO.into_bytes().to_vec(),
    });

    module.struct_defs.push(StructDefinition {
        struct_handle: StructHandleIndex(0),
        field_information: StructFieldInformation::Declared(vec![FieldDefinition {
            name: IdentifierIndex::new(3),
            signature: TypeSignature(Address),
        }]),
    });

    let code_unit = CodeUnit {
        code: mix.code,
        locals: SignatureIndex(0),
    };

    let fun_def = FunctionDefinition {
        code: Some(code_unit),
        function: FunctionHandleIndex(0),
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
    };

    module.function_defs.push(fun_def);
    if compiled_module_serde(&module).is_ok() {
        let _ = move_bytecode_verifier::verify_module(&module);
    }
});
