// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::{
    empty_module, AbilitySet, Bytecode, CodeUnit, FunctionDefinition, FunctionHandle,
    FunctionHandleIndex, IdentifierIndex, ModuleHandleIndex, Signature, SignatureIndex,
    SignatureToken, Visibility::Public,
};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{identifier::Identifier, vm_status::StatusCode};

fn get_fun_type_bool_to_bool() -> SignatureToken {
    let bool_token = SignatureToken::Bool;
    let abilities = AbilitySet::PUBLIC_FUNCTIONS;
    SignatureToken::Function(
        vec![bool_token.clone()],
        vec![bool_token.clone()],
        abilities,
    )
}

fn get_fun_type_nothing_to_bool() -> SignatureToken {
    let bool_token = SignatureToken::Bool;
    let abilities = AbilitySet::PUBLIC_FUNCTIONS;
    SignatureToken::Function(vec![], vec![bool_token.clone()], abilities)
}

#[test]
fn test_function_value_type() {
    let mut m = empty_module();

    // 0 == no values
    m.signatures.push(Signature(vec![]));
    // 1 == function bool->bool
    m.signatures
        .push(Signature(vec![get_fun_type_bool_to_bool()]));

    // fun f0(x: |bool|bool): |bool|bool { x }
    m.identifiers
        .push(Identifier::new(format!("f{}", 0)).unwrap());
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(1),
        return_: SignatureIndex(1),
        type_parameters: vec![],
        access_specifiers: None,
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            // No locals
            locals: SignatureIndex(0),
            // Just pass through the single function value parameter
            code: vec![Bytecode::Ret],
        }),
    });

    let result = move_bytecode_verifier::verify_module_with_config_for_test(
        "test_function_value_type",
        &VerifierConfig::production(),
        &m,
    );
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::FEATURE_NOT_ENABLED
    );
}

#[test]
fn test_function_ld_function() {
    let mut m = empty_module();

    // 0 == no values
    m.signatures.push(Signature(vec![]));
    // 1 == function bool->bool
    m.signatures
        .push(Signature(vec![get_fun_type_bool_to_bool()]));
    // 2 == bool
    m.signatures.push(Signature(vec![SignatureToken::Bool]));

    // fun f0(x; bool): bool { x }
    m.identifiers
        .push(Identifier::new(format!("f{}", 0)).unwrap());
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(1),
        return_: SignatureIndex(1),
        type_parameters: vec![],
        access_specifiers: None,
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            // No locals
            locals: SignatureIndex(0),
            // Just pass through the single function value parameter
            code: vec![Bytecode::Ret],
        }),
    });

    // fun f1(): |bool|bool { f0 }
    m.identifiers
        .push(Identifier::new(format!("f{}", 1)).unwrap());
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        parameters: SignatureIndex(0),
        return_: SignatureIndex(1),
        type_parameters: vec![],
        access_specifiers: None,
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(1),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            // No locals
            locals: SignatureIndex(0),
            // Return the function value
            code: vec![Bytecode::LdFunction(FunctionHandleIndex(1)), Bytecode::Ret],
        }),
    });

    let result = move_bytecode_verifier::verify_module_with_config_for_test(
        "test_function_ld_function",
        &VerifierConfig::production(),
        &m,
    );
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::FEATURE_NOT_ENABLED,
    );
}

#[test]
fn test_function_early_bind() {
    let mut m = empty_module();

    // 0 == no values
    m.signatures.push(Signature(vec![]));
    // 1 == function bool->bool
    m.signatures
        .push(Signature(vec![get_fun_type_bool_to_bool()]));
    // 2 == bool
    m.signatures.push(Signature(vec![SignatureToken::Bool]));
    // 3 == function ()->bool
    m.signatures
        .push(Signature(vec![get_fun_type_nothing_to_bool()]));

    // fun f0(x; bool): bool { x }
    m.identifiers
        .push(Identifier::new(format!("f{}", 0)).unwrap());
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(1),
        return_: SignatureIndex(1),
        type_parameters: vec![],
        access_specifiers: None,
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            // No locals
            locals: SignatureIndex(0),
            // Just pass through the single function value parameter
            code: vec![Bytecode::Ret],
        }),
    });

    // fun f1(x: bool): ||bool { || f0(x) }
    m.identifiers
        .push(Identifier::new(format!("f{}", 1)).unwrap());
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        parameters: SignatureIndex(1),
        return_: SignatureIndex(2),
        type_parameters: vec![],
        access_specifiers: None,
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(1),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            // No locals
            locals: SignatureIndex(0),
            // Bool is on stack, load the function, early bind 1 param, return result
            code: vec![
                Bytecode::LdFunction(FunctionHandleIndex(0)),
                Bytecode::EarlyBindFunction(SignatureIndex(1), 1u8),
                Bytecode::Ret,
            ],
        }),
    });

    let result = move_bytecode_verifier::verify_module_with_config_for_test(
        "test_function_ld_function",
        &VerifierConfig::production(),
        &m,
    );
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::FEATURE_NOT_ENABLED
    );
}

#[test]
fn test_function_value_call() {
    let mut m = empty_module();

    // 0 == no values
    m.signatures.push(Signature(vec![]));
    // 1 == function bool->bool
    m.signatures
        .push(Signature(vec![get_fun_type_bool_to_bool()]));
    // 2 == bool
    m.signatures.push(Signature(vec![SignatureToken::Bool]));
    // 3 == function ()->bool
    m.signatures
        .push(Signature(vec![get_fun_type_nothing_to_bool()]));

    // fun f0(x; bool): bool { x }
    m.identifiers
        .push(Identifier::new(format!("f{}", 0)).unwrap());
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(0),
        parameters: SignatureIndex(1),
        return_: SignatureIndex(1),
        type_parameters: vec![],
        access_specifiers: None,
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            // No locals
            locals: SignatureIndex(0),
            // Just pass through the single function value parameter
            code: vec![Bytecode::Ret],
        }),
    });

    // fun f1(x: bool): ||bool { (f0)(x) }
    m.identifiers
        .push(Identifier::new(format!("f{}", 1)).unwrap());
    m.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name: IdentifierIndex(1),
        parameters: SignatureIndex(1),
        return_: SignatureIndex(2),
        type_parameters: vec![],
        access_specifiers: None,
    });
    m.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(1),
        visibility: Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            // No locals
            locals: SignatureIndex(0),
            // Bool is on stack, load the function value, Invoke
            code: vec![
                Bytecode::LdFunction(FunctionHandleIndex(0)),
                Bytecode::InvokeFunction(SignatureIndex(1)),
                Bytecode::Ret,
            ],
        }),
    });

    let result = move_bytecode_verifier::verify_module_with_config_for_test(
        "test_function_ld_function",
        &VerifierConfig::production(),
        &m,
    );
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::FEATURE_NOT_ENABLED
    );
}
