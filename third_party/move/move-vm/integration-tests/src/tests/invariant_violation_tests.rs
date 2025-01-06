// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::{
    Bytecode::*, CodeUnit, CompiledScript, Constant, ConstantPoolIndex, Signature, SignatureIndex,
    SignatureToken::*,
};
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::{
    module_traversal::*, move_vm::MoveVM, AsUnsyncCodeStorage, RuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

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
    };

    move_bytecode_verifier::verify_script(&cs).expect("verify failed");
    let runtime_environment = RuntimeEnvironment::new(vec![]);
    let vm = MoveVM::new_with_runtime_environment(&runtime_environment);

    let storage: InMemoryStorage = InMemoryStorage::new();
    let mut session = vm.new_session(&storage);
    let mut script_bytes = vec![];
    cs.serialize(&mut script_bytes).unwrap();

    let traversal_storage = TraversalStorage::new();
    let code_storage = storage.as_unsync_code_storage(runtime_environment);

    let err = session
        .execute_script(
            script_bytes.as_slice(),
            vec![],
            Vec::<Vec<u8>>::new(),
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &code_storage,
        )
        .unwrap_err();

    assert_eq!(
        err.major_status(),
        StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
    );
}
