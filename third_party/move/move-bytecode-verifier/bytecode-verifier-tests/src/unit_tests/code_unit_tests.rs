// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::support::{
    dummy_procedure_module, module_with_enum, module_with_struct, module_with_struct_api_attributes,
};
use move_binary_format::file_format::{
    Bytecode, FunctionAttribute, Signature, SignatureToken, StructDefinitionIndex,
    StructHandleIndex, StructVariantHandleIndex, VariantFieldHandleIndex,
};
use move_bytecode_verifier::{CodeUnitVerifier, VerifierConfig};
use move_core_types::vm_status::StatusCode;

#[test]
fn test_struct_api_attributes_more_than_one() {
    let mut module = module_with_struct();
    let struct_def_index = StructDefinitionIndex(0);
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::Pack(struct_def_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::Pack, FunctionAttribute::Unpack],
        Signature(vec![SignatureToken::Bool]),
        Signature(vec![SignatureToken::Struct(StructHandleIndex(0))]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_STRUCT_API_CODE
    );
}

#[test]
fn test_struct_api_attributes_invalid_pack() {
    let mut module = module_with_struct();
    let struct_def_index = StructDefinitionIndex(0);
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::LdFalse,
            Bytecode::Pop,
            Bytecode::Pack(struct_def_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::Pack, FunctionAttribute::Unpack],
        Signature(vec![SignatureToken::Bool]),
        Signature(vec![SignatureToken::Struct(StructHandleIndex(0))]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_STRUCT_API_CODE
    );
}

#[test]
fn test_struct_api_attributes_valid_pack() {
    let mut module = module_with_struct();
    let struct_def_index = StructDefinitionIndex(0);
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::Pack(struct_def_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::Pack],
        Signature(vec![SignatureToken::Bool]),
        Signature(vec![SignatureToken::Struct(StructHandleIndex(0))]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn test_struct_api_attributes_valid_unpack() {
    let mut module = module_with_struct();
    let struct_def_index = StructDefinitionIndex(0);
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::Unpack(struct_def_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::Unpack],
        Signature(vec![SignatureToken::Struct(StructHandleIndex(0))]),
        Signature(vec![SignatureToken::Bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn test_struct_api_attributes_invalid_unpack() {
    let mut module = module_with_struct();
    let struct_def_index = StructDefinitionIndex(0);
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::LdFalse,
            Bytecode::Pop,
            Bytecode::Unpack(struct_def_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::Unpack],
        Signature(vec![SignatureToken::Struct(StructHandleIndex(0))]),
        Signature(vec![SignatureToken::Bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_STRUCT_API_CODE
    );
}

#[test]
fn test_struct_api_attributes_invalid_test_variant() {
    let mut module = module_with_enum();
    let struct_variant_index = StructVariantHandleIndex(0);
    let ref_type =
        SignatureToken::Reference(Box::new(SignatureToken::Struct(StructHandleIndex(0))));
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::LdFalse,
            Bytecode::Pop,
            Bytecode::TestVariant(struct_variant_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::TestVariant(0)],
        Signature(vec![ref_type]),
        Signature(vec![SignatureToken::Bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_STRUCT_API_CODE
    );
}

#[test]
fn test_struct_api_attributes_valid_test_variant() {
    let mut module = module_with_enum();
    let struct_variant_index = StructVariantHandleIndex(0);
    let ref_type =
        SignatureToken::Reference(Box::new(SignatureToken::Struct(StructHandleIndex(0))));
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::TestVariant(struct_variant_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::TestVariant(0)],
        Signature(vec![ref_type]),
        Signature(vec![SignatureToken::Bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn test_struct_api_attributes_invalid_borrow_field_immutable() {
    let mut module = module_with_enum();
    let variant_field_index = VariantFieldHandleIndex(0);
    let ref_type =
        SignatureToken::Reference(Box::new(SignatureToken::Struct(StructHandleIndex(0))));
    let ref_bool = SignatureToken::Reference(Box::new(SignatureToken::Bool));
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::LdFalse,
            Bytecode::Pop,
            Bytecode::ImmBorrowVariantField(variant_field_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::BorrowFieldImmutable(0)],
        Signature(vec![ref_type]),
        Signature(vec![ref_bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_STRUCT_API_CODE
    );
}

#[test]
fn test_struct_api_attributes_valid_borrow_field_immutable() {
    let mut module = module_with_enum();
    let variant_field_index = VariantFieldHandleIndex(0);
    let ref_type =
        SignatureToken::Reference(Box::new(SignatureToken::Struct(StructHandleIndex(0))));
    let ref_bool = SignatureToken::Reference(Box::new(SignatureToken::Bool));
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::ImmBorrowVariantField(variant_field_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::BorrowFieldImmutable(0)],
        Signature(vec![ref_type]),
        Signature(vec![ref_bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn test_struct_api_attributes_invalid_borrow_field_mutable() {
    let mut module = module_with_enum();
    let variant_field_index = VariantFieldHandleIndex(0);
    let ref_type =
        SignatureToken::MutableReference(Box::new(SignatureToken::Struct(StructHandleIndex(0))));
    let ref_bool = SignatureToken::MutableReference(Box::new(SignatureToken::Bool));
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::LdFalse,
            Bytecode::Pop,
            Bytecode::MutBorrowVariantField(variant_field_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::BorrowFieldMutable(0)],
        Signature(vec![ref_type]),
        Signature(vec![ref_bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_STRUCT_API_CODE
    );
}

#[test]
fn test_struct_api_attributes_valid_borrow_field_mutable() {
    let mut module = module_with_enum();
    let variant_field_index = VariantFieldHandleIndex(0);
    let ref_type =
        SignatureToken::MutableReference(Box::new(SignatureToken::Struct(StructHandleIndex(0))));
    let ref_bool = SignatureToken::MutableReference(Box::new(SignatureToken::Bool));
    module_with_struct_api_attributes(
        &mut module,
        vec![
            Bytecode::MoveLoc(0),
            Bytecode::MutBorrowVariantField(variant_field_index),
            Bytecode::Ret,
        ],
        vec![FunctionAttribute::BorrowFieldMutable(0)],
        Signature(vec![ref_type]),
        Signature(vec![ref_bool]),
    );
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn invalid_fallthrough_br_true() {
    let module = dummy_procedure_module(vec![Bytecode::LdFalse, Bytecode::BrTrue(1)]);
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_FALL_THROUGH
    );
}

#[test]
fn invalid_fallthrough_br_false() {
    let module = dummy_procedure_module(vec![Bytecode::LdTrue, Bytecode::BrFalse(1)]);
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_FALL_THROUGH
    );
}

// all non-branch instructions should trigger invalid fallthrough; just check one of them
#[test]
fn invalid_fallthrough_non_branch() {
    let module = dummy_procedure_module(vec![Bytecode::LdTrue, Bytecode::Pop]);
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::INVALID_FALL_THROUGH
    );
}

#[test]
fn valid_fallthrough_branch() {
    let module = dummy_procedure_module(vec![Bytecode::Branch(0)]);
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn valid_fallthrough_ret() {
    let module = dummy_procedure_module(vec![Bytecode::Ret]);
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn valid_fallthrough_abort() {
    let module = dummy_procedure_module(vec![Bytecode::LdU64(7), Bytecode::Abort]);
    let result = CodeUnitVerifier::verify_module(&Default::default(), &module);
    assert!(result.is_ok());
}

#[test]
fn test_max_number_of_bytecode() {
    let mut nops = vec![];
    for _ in 0..u16::MAX - 1 {
        nops.push(Bytecode::Nop);
    }
    nops.push(Bytecode::Ret);
    let module = dummy_procedure_module(nops);

    let result = CodeUnitVerifier::verify_module(&VerifierConfig::unbounded(), &module);
    assert!(result.is_ok());
}

#[test]
fn test_max_basic_blocks() {
    let mut code = (0..17)
        .map(|idx| Bytecode::Branch(idx + 1))
        .collect::<Vec<_>>();
    code.push(Bytecode::Ret);
    let module = dummy_procedure_module(code);

    let result = CodeUnitVerifier::verify_module(
        &VerifierConfig {
            max_basic_blocks: Some(16),
            ..Default::default()
        },
        &module,
    );
    assert_eq!(
        result.unwrap_err().major_status(),
        StatusCode::TOO_MANY_BASIC_BLOCKS
    );
}
