// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use move_binary_format::errors::PartialVMResult;
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::InternalGas, identifier::Identifier,
};
use move_vm_runtime::{config::VMConfig, move_vm::MoveVM, native_functions::NativeFunction};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, natives::function::NativeResult};
use std::sync::Arc;

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

fn make_failed_native() -> NativeFunction {
    Arc::new(move |_, _, _| -> PartialVMResult<NativeResult> {
        Ok(NativeResult::Abort {
            cost: InternalGas::new(0),
            abort_code: 12,
        })
    })
}

#[test]
fn test_publish_module_with_nested_loops() {
    // Compile the modules and scripts.
    // TODO: find a better way to include the Signer module.
    let code = r#"
        module {{ADDR}}::M {
            entry fun foo() {
                Self::bar();
            }

            entry fun foo2() {
                Self::foo1();
            }

            fun foo1() {
                Self::bar();
            }

            native fun bar();
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();

    let m = as_module(units.pop().unwrap());
    let mut m_blob = vec![];
    m.serialize(&mut m_blob).unwrap();

    // Should succeed with max_loop_depth = 2
    {
        let storage = InMemoryStorage::new();

        let natives = vec![(
            TEST_ADDR,
            Identifier::new("M").unwrap(),
            Identifier::new("bar").unwrap(),
            make_failed_native(),
        )];
        let vm = MoveVM::new_with_config(natives, VMConfig {
            verifier: VerifierConfig {
                max_loop_depth: Some(2),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();

        let mut sess = vm.new_session(&storage);
        sess.publish_module(m_blob.clone(), TEST_ADDR, &mut UnmeteredGasMeter)
            .unwrap();

        let err1 = sess
            .execute_entry_function(
                &m.self_id(),
                &Identifier::new("foo").unwrap(),
                vec![],
                Vec::<Vec<u8>>::new(),
                &mut UnmeteredGasMeter,
            )
            .unwrap_err();

        assert!(err1.exec_state().unwrap().stack_trace().is_empty());

        let err2 = sess
            .execute_entry_function(
                &m.self_id(),
                &Identifier::new("foo2").unwrap(),
                vec![],
                Vec::<Vec<u8>>::new(),
                &mut UnmeteredGasMeter,
            )
            .unwrap_err();

        assert!(err2.exec_state().unwrap().stack_trace().len() == 1);
    }
}
