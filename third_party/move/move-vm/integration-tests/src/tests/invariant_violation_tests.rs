// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::execute_script_for_test;
use move_binary_format::file_format::{
    Bytecode::*, CodeUnit, CompiledScript, Constant, ConstantPoolIndex, Signature, SignatureIndex,
    SignatureToken::*,
};
use move_core_types::vm_status::StatusCode;
use move_vm_test_utils::InMemoryStorage;

#[test]
fn merge_borrow_states_infinite_loop() {
    let cs = CompiledScript {
        version: 6,
        module_handles: vec![],
        struct_handles: vec![],
        function_handles: vec![],
        function_instantiations: vec![],
        signatures: vec![
            Signature(vec![]),
            Signature(vec![
                U64,
                Vector(Box::new(U8)),
                U64,
                Vector(Box::new(U8)),
                MutableReference(Box::new(Vector(Box::new(U8)))),
                MutableReference(Box::new(U64)),
            ]),
        ],
        identifiers: vec![],
        address_identifiers: vec![],
        constant_pool: vec![Constant {
            type_: Vector(Box::new(U8)),
            data: vec![0],
        }],
        metadata: vec![],
        // The bytecode verifier should reject this code but it doesn't.
        code: CodeUnit {
            locals: SignatureIndex(1),
            code: vec![
                LdU64(0),
                StLoc(0), // { 0 => 0 }
                LdConst(ConstantPoolIndex(0)),
                StLoc(1), // { 0 => 0, 1 => [] }
                LdU64(0),
                StLoc(2), // { 0 => 0, 1 => [], 2 => 0 }
                MutBorrowLoc(2),
                StLoc(5), // { 0 => 0, 1 => [], 2 => 0, 5 => &2 }
                LdU64(1),
                CopyLoc(5),
                WriteRef, // { 0 => 0, 1 => [], 2 => 1, 5 => &2 }
                LdConst(ConstantPoolIndex(0)),
                StLoc(3), // { 0 => 0, 1 => [], 2 => 1, 3 => [], 5 => &2 }
                MutBorrowLoc(3),
                StLoc(4), // { 0 => 0, 1 => [], 2 => 1, 3 => [], 4 => &3, 5 => &2 }
                LdConst(ConstantPoolIndex(0)),
                CopyLoc(4),
                WriteRef,
                CopyLoc(5),
                ReadRef,
                LdU64(1),
                Eq,
                BrTrue(11),
                Ret,
            ],
        },
        type_parameters: vec![],
        parameters: SignatureIndex(0),
        access_specifiers: None,
    };

    move_bytecode_verifier::verify_script(&cs).expect("verify failed");

    let mut script_bytes = vec![];
    cs.serialize(&mut script_bytes).unwrap();

    let storage = InMemoryStorage::new();
    let status = execute_script_for_test(&storage, &script_bytes, &[], vec![])
        .unwrap_err()
        .major_status();
    assert_eq!(status, StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR);
}
