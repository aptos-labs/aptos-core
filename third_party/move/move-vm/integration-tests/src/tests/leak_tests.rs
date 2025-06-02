// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::execute_script_for_test;
use claims::assert_ok;
use move_binary_format::file_format::{
    Bytecode::*, CodeUnit, CompiledScript, Signature, SignatureIndex, SignatureToken::*,
};
use move_vm_test_utils::InMemoryStorage;

#[ignore] // TODO: figure whether to reactive this test
#[test]
fn leak_with_abort() {
    let mut locals = vec![U128, MutableReference(Box::new(U128))];
    // Make locals bigger so each leak is bigger
    // 128 is limit for aptos
    for _ in 0..100 {
        locals.push(U128);
    }
    let cs = CompiledScript {
        version: 6,
        module_handles: vec![],
        struct_handles: vec![],
        function_handles: vec![],
        function_instantiations: vec![],
        signatures: vec![Signature(vec![]), Signature(locals)],
        identifiers: vec![],
        address_identifiers: vec![],
        constant_pool: vec![],
        metadata: vec![],
        code: CodeUnit {
            locals: SignatureIndex(1),
            code: vec![
                // leak
                LdU128(0),
                StLoc(0),
                MutBorrowLoc(0),
                StLoc(1),
                // abort
                LdU64(0),
                Abort,
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
    for _ in 0..100_000 {
        let result = execute_script_for_test(&storage, &script_bytes, &[], vec![]);
        assert_ok!(result);
    }

    let mem_stats = memory_stats::memory_stats().unwrap();
    assert!(
        mem_stats.virtual_mem < 200000000,
        "actual is {}",
        mem_stats.virtual_mem
    );
}
