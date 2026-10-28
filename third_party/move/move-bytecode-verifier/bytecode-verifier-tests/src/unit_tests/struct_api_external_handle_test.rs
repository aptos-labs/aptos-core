// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for borrow field struct API attributes on function handles that
//! `check_function` has not validated: imported handles, script handles, and local
//! handles called before their definition is checked. With
//! `VerifierConfig::invariant_violation_fixes` on, a handle with the wrong signature
//! shape is rejected up front; with it off, the outcome is unchanged.

use move_binary_format::{
    errors::VMResult,
    file_format::{
        empty_module, empty_script_with_dependencies, AddressIdentifierIndex, Bytecode, CodeUnit,
        CompiledScript, FunctionAttribute, FunctionDefinition, FunctionHandle, FunctionHandleIndex,
        IdentifierIndex, ModuleHandle, ModuleHandleIndex, Signature, SignatureIndex,
        SignatureToken, StructHandle, StructHandleIndex, Visibility,
    },
    CompiledModule, IndexKind,
};
use move_bytecode_verifier::{
    verifier::{verify_module_with_config, verify_script_with_config},
    VerifierConfig,
};
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, identifier::Identifier,
    vm_status::StatusCode,
};

fn ident(name: &str) -> Identifier {
    Identifier::new(name).expect("valid identifier")
}

fn borrow_mut_handle(module: ModuleHandleIndex, name: IdentifierIndex) -> FunctionHandle {
    FunctionHandle {
        module,
        name,
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![FunctionAttribute::BorrowFieldMutable(0)],
    }
}

/// Module with an imported handle `Other::ext_fn` (parameters `()`) tagged
/// `BorrowFieldMutable(0)`, and a local `test` whose body is `Call(ext_fn); Ret`.
/// The empty parameter list lets type safety accept the call with an empty stack.
fn module_with_external_borrow_handle() -> CompiledModule {
    let mut m = empty_module();
    // identifiers: [<self>, Other, ext_fn, test]
    m.identifiers
        .extend([ident("Other"), ident("ext_fn"), ident("test")]);
    m.address_identifiers.push(AccountAddress::ONE);
    m.module_handles.push(ModuleHandle {
        address: AddressIdentifierIndex(1),
        name: IdentifierIndex(1),
    });
    m.function_handles = vec![
        borrow_mut_handle(ModuleHandleIndex(1), IdentifierIndex(2)),
        FunctionHandle {
            attributes: vec![],
            ..borrow_mut_handle(ModuleHandleIndex(0), IdentifierIndex(3))
        },
    ];
    m.function_defs = vec![FunctionDefinition {
        function: FunctionHandleIndex(1),
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Call(FunctionHandleIndex(0)), Bytecode::Ret],
        }),
    }];
    m
}

/// Script calling an imported handle `Other::ext_fn` (parameters `()`) tagged
/// `BorrowFieldMutable(0)`.
fn script_with_bad_borrow_handle() -> CompiledScript {
    let mut s = empty_script_with_dependencies(["Other"]);
    // identifiers: [Other, ext_fn]
    s.identifiers.push(ident("ext_fn"));
    s.function_handles = vec![borrow_mut_handle(ModuleHandleIndex(0), IdentifierIndex(1))];
    s.code.code = vec![Bytecode::Call(FunctionHandleIndex(0)), Bytecode::Ret];
    s
}

fn assert_rejected_with(result: VMResult<()>, expected: StatusCode) {
    let err = result.expect_err("must not verify");
    assert_eq!(err.major_status(), expected, "{:?}", err);
}

#[test]
fn external_borrow_handle_with_bad_shape_is_rejected_cleanly() {
    let module = module_with_external_borrow_handle();
    let err = verify_module_with_config(&VerifierConfig::production_testing(), &module)
        .expect_err("must not verify");
    assert_eq!(err.major_status(), StatusCode::INVALID_STRUCT_API_CODE);
    assert_eq!(err.indices(), &vec![(IndexKind::FunctionHandle, 0)]);
}

/// The verifier interleaves per-definition checking with per-definition analysis, so
/// def[0]'s reference safety runs before `check_function` validates def[1]. A caller
/// can therefore reach the fast path through a local handle before it is validated.
#[test]
fn forward_referenced_borrow_handle_with_bad_shape_is_rejected_cleanly() {
    let mut module = module_with_external_borrow_handle();
    module.function_handles[0].module = ModuleHandleIndex(0);
    module.function_handles[0].name = IdentifierIndex(4);
    module.identifiers.push(ident("bad$S"));
    module.function_defs.push(FunctionDefinition {
        function: FunctionHandleIndex(0),
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: vec![Bytecode::Ret],
        }),
    });
    assert_rejected_with(
        verify_module_with_config(&VerifierConfig::production_testing(), &module),
        StatusCode::INVALID_STRUCT_API_CODE,
    );
}

fn mut_ref(token: SignatureToken) -> SignatureToken {
    SignatureToken::MutableReference(Box::new(token))
}

fn imm_ref(token: SignatureToken) -> SignatureToken {
    SignatureToken::Reference(Box::new(token))
}

const S: SignatureToken = SignatureToken::Struct(StructHandleIndex(0));

/// Returns the index of `sig` in the signature pool, adding it if absent; the pool must
/// not contain duplicates.
fn signature_index(module: &mut CompiledModule, sig: Signature) -> SignatureIndex {
    let idx = match module.signatures.iter().position(|s| *s == sig) {
        Some(idx) => idx,
        None => {
            module.signatures.push(sig);
            module.signatures.len() - 1
        },
    };
    SignatureIndex(idx as u16)
}

/// Module whose imported handle `Other::borrow_mut$S$0` has the given attribute,
/// parameter, and return types, and a local `test(p): r` that forwards its parameter
/// to the handle: `MoveLoc(0); Call(handle); Ret`.
fn module_with_imported_handle(
    attr: FunctionAttribute,
    param: SignatureToken,
    ret: Vec<SignatureToken>,
) -> CompiledModule {
    let mut module = module_with_external_borrow_handle();
    let param_idx = signature_index(&mut module, Signature(vec![param]));
    let ret_idx = signature_index(&mut module, Signature(ret));
    module.identifiers.push(ident("borrow_mut$S$0"));
    module.identifiers.push(ident("S"));
    module.struct_handles = vec![StructHandle {
        module: ModuleHandleIndex(1),
        name: IdentifierIndex(5),
        abilities: AbilitySet::EMPTY,
        type_parameters: vec![],
    }];
    for handle in &mut module.function_handles {
        handle.parameters = param_idx;
        handle.return_ = ret_idx;
    }
    module.function_handles[0].name = IdentifierIndex(4);
    module.function_handles[0].attributes = vec![attr];
    let code = module.function_defs[0]
        .code
        .as_mut()
        .expect("test function has code");
    code.locals = param_idx;
    code.code = vec![
        Bytecode::MoveLoc(0),
        Bytecode::Call(FunctionHandleIndex(0)),
        Bytecode::Ret,
    ];
    module
}

/// An imported handle with the wrapper shape must keep verifying: cross-module struct
/// access compiles to exactly this kind of call.
#[test]
fn external_borrow_handle_with_wrapper_shape_verifies() {
    use FunctionAttribute::{BorrowFieldImmutable, BorrowFieldMutable};
    use SignatureToken::U64;
    for (attr, param, ret) in [
        (BorrowFieldMutable(0), mut_ref(S), mut_ref(U64)),
        (BorrowFieldImmutable(0), imm_ref(S), imm_ref(U64)),
    ] {
        let module = module_with_imported_handle(attr.clone(), param, vec![ret]);
        verify_module_with_config(&VerifierConfig::production_testing(), &module)
            .unwrap_or_else(|e| panic!("well-shaped {:?} handle must verify: {:?}", attr, e));
    }
}

/// Each way a borrow field handle's signature can deviate from the wrapper shape is
/// rejected, including when only the return or only the mutability is wrong.
#[test]
fn borrow_handle_shape_mismatches_are_rejected_cleanly() {
    use FunctionAttribute::{BorrowFieldImmutable, BorrowFieldMutable};
    use SignatureToken::{Bool, U64};
    for (case, attr, param, ret) in [
        ("param not a reference", BorrowFieldMutable(0), S, vec![
            mut_ref(U64),
        ]),
        (
            "param not a struct",
            BorrowFieldMutable(0),
            mut_ref(U64),
            vec![mut_ref(U64)],
        ),
        (
            "immutable param for mutable attr",
            BorrowFieldMutable(0),
            imm_ref(S),
            vec![mut_ref(U64)],
        ),
        (
            "mutable param for immutable attr",
            BorrowFieldImmutable(0),
            mut_ref(S),
            vec![imm_ref(U64)],
        ),
        (
            "return not a reference",
            BorrowFieldMutable(0),
            mut_ref(S),
            vec![Bool],
        ),
        (
            "immutable return for mutable attr",
            BorrowFieldMutable(0),
            mut_ref(S),
            vec![imm_ref(U64)],
        ),
        ("no return", BorrowFieldMutable(0), mut_ref(S), vec![]),
        ("two returns", BorrowFieldMutable(0), mut_ref(S), vec![
            mut_ref(U64),
            mut_ref(U64),
        ]),
    ] {
        let module = module_with_imported_handle(attr, param, ret);
        let result = verify_module_with_config(&VerifierConfig::production_testing(), &module);
        let err = result.expect_err(case);
        assert_eq!(
            err.major_status(),
            StatusCode::INVALID_STRUCT_API_CODE,
            "{}: {:?}",
            case,
            err
        );
    }
}

/// Without the return check, a `&mut S -> bool` handle still crashes reference safety:
/// the fast path pushes a reference where type safety saw a bool, so the following
/// `BrTrue` hits an invariant violation.
#[test]
fn borrow_handle_with_value_return_before_branch() {
    let mut module =
        module_with_imported_handle(FunctionAttribute::BorrowFieldMutable(0), mut_ref(S), vec![
            SignatureToken::Bool,
        ]);
    // test(s: &mut S) { MoveLoc(0); Call(h); BrTrue(4); Ret; Ret }
    module.function_handles[1].return_ = SignatureIndex(0);
    let code = module.function_defs[0]
        .code
        .as_mut()
        .expect("test function has code");
    code.code = vec![
        Bytecode::MoveLoc(0),
        Bytecode::Call(FunctionHandleIndex(0)),
        Bytecode::BrTrue(4),
        Bytecode::Ret,
        Bytecode::Ret,
    ];
    assert_rejected_with(
        verify_module_with_config(&VerifierConfig::production_testing(), &module),
        StatusCode::INVALID_STRUCT_API_CODE,
    );
    let flag_off = VerifierConfig {
        invariant_violation_fixes: false,
        ..VerifierConfig::production_testing()
    };
    assert_rejected_with(
        verify_module_with_config(&flag_off, &module),
        StatusCode::VERIFIER_INVARIANT_VIOLATION,
    );
}

#[test]
fn script_borrow_handle_with_bad_shape_is_rejected_cleanly() {
    let script = script_with_bad_borrow_handle();
    assert_rejected_with(
        verify_script_with_config(&VerifierConfig::production_testing(), &script),
        StatusCode::INVALID_STRUCT_API_CODE,
    );
}

/// With the feature flag off, verification must behave exactly as before, so
/// upgraded and non-upgraded validators agree until the flag is enabled.
#[test]
fn flag_off_preserves_previous_behavior() {
    let config = VerifierConfig {
        invariant_violation_fixes: false,
        ..VerifierConfig::production_testing()
    };
    assert_rejected_with(
        verify_module_with_config(&config, &module_with_external_borrow_handle()),
        StatusCode::VERIFIER_INVARIANT_VIOLATION,
    );
    assert_rejected_with(
        verify_script_with_config(&config, &script_with_bad_borrow_handle()),
        StatusCode::VERIFIER_INVARIANT_VIOLATION,
    );
}
