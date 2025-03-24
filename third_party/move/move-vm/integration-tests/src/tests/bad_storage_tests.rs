// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compiler::{as_module, as_script, compile_units},
    tests::{
        execute_function_for_test, execute_function_with_single_storage_for_test,
        execute_script_and_commit_change_set_for_test, execute_script_for_test,
    },
};
use bytes::Bytes;
use claims::assert_ok;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    value::{serialize_values, MoveTypeLayout, MoveValue},
    vm_status::{StatusCode, StatusType},
};
use move_vm_runtime::{AsUnsyncModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{code::ModuleBytesStorage, resolver::ResourceResolver};

const TEST_ADDR: AccountAddress = AccountAddress::new([42; AccountAddress::LENGTH]);

#[test]
fn test_malformed_resource() {
    // Compile the modules and scripts.
    // TODO: find a better way to include the Signer module.
    let code = r#"
        address std {
            module signer {
                native public fun borrow_address(s: &signer): &address;

                public fun address_of(s: &signer): address {
                    *borrow_address(s)
                }
            }
        }

        module {{ADDR}}::M {
            use std::signer;

            struct Foo has key { x: u64, y: bool }

            public fun publish(s: &signer) {
                move_to(s, Foo { x: 123, y : false });
            }

            public fun check(s: &signer) acquires Foo {
                let foo = borrow_global<Foo>(signer::address_of(s));
                assert!(foo.x == 123 && foo.y == false, 42);
            }
        }

        script {
            use {{ADDR}}::M;

            fun main(s: signer) {
                M::publish(&s);
            }
        }

        script {
            use {{ADDR}}::M;

            fun main(s: signer) {
                M::check(&s);
            }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();

    let s2 = as_script(units.pop().unwrap());
    let s1 = as_script(units.pop().unwrap());
    let m = as_module(units.pop().unwrap());
    let ms = as_module(units.pop().unwrap());

    let natives = move_stdlib::natives::all_natives(
        AccountAddress::from_hex_literal("0x1").unwrap(),
        move_stdlib::natives::GasParameters::zeros(),
    );
    let runtime_environment = RuntimeEnvironment::new(natives);
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

    // Publish module Signer and module M.
    let mut blob = vec![];
    ms.serialize(&mut blob).unwrap();
    storage.add_module_bytes(ms.self_addr(), ms.self_name(), blob.into());

    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();
    storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());

    // Execute the first script to publish a resource Foo.
    let mut script_blob = vec![];
    s1.serialize(&mut script_blob).unwrap();

    let args = vec![MoveValue::Signer(TEST_ADDR).simple_serialize().unwrap()];
    let result = execute_script_and_commit_change_set_for_test(
        &mut storage,
        &script_blob,
        &[],
        args.clone(),
    );
    assert_ok!(result);

    // Execute the second script and make sure it succeeds. This script simply checks
    // that the published resource is what we expect it to be. This initial run is to ensure
    // the testing environment is indeed free of errors without external interference.
    let mut script_blob = vec![];
    s2.serialize(&mut script_blob).unwrap();
    {
        let result = execute_script_for_test(&storage, &script_blob, &[], args.clone());
        assert_ok!(result);
    }

    // Corrupt the resource in the storage.
    storage.publish_or_overwrite_resource(
        TEST_ADDR,
        StructTag {
            address: TEST_ADDR,
            module: Identifier::new("M").unwrap(),
            name: Identifier::new("Foo").unwrap(),
            type_args: vec![],
        },
        vec![0x3, 0x4, 0x5],
    );

    // Run the second script again.
    // The test will be successful if it fails with an invariant violation.
    {
        let status_type = execute_script_for_test(&storage, &script_blob, &[], args)
            .unwrap_err()
            .status_type();
        assert_eq!(status_type, StatusType::InvariantViolation);
    }
}

#[test]
fn test_malformed_module() {
    // Compile module M.
    let code = r#"
        module {{ADDR}}::M {
            public fun foo() {}
        }
    "#;

    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();

    let m = as_module(units.pop().unwrap());

    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
    let fun_name = Identifier::new("foo").unwrap();

    // Publish M and call M::foo. No errors should be thrown.
    {
        let mut storage = InMemoryStorage::new();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob.clone().into());

        let result = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        );
        assert_ok!(result);
    }

    // Start over with a fresh storage and publish a corrupted version of M.
    // A fresh VM needs to be used whenever the storage has been modified or otherwise the
    // loader cache gets out of sync.
    //
    // Try to call M::foo again and the module should fail to load, causing an
    // invariant violation error.
    {
        blob[0] = 0xDE;
        blob[1] = 0xAD;
        blob[2] = 0xBE;
        blob[3] = 0xEF;

        let mut storage = InMemoryStorage::new();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());

        let err = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err.status_type(), StatusType::InvariantViolation);
    }
}

#[test]
fn test_unverifiable_module() {
    // Compile module M.
    let code = r#"
        module {{ADDR}}::M {
            public fun foo() {}
        }
    "#;

    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("M").unwrap());
    let fun_name = Identifier::new("foo").unwrap();

    // Publish M and call M::foo to make sure it works.
    {
        let mut storage = InMemoryStorage::new();

        let mut blob = vec![];
        m.serialize(&mut blob).unwrap();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());

        let result = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        );
        assert_ok!(result);
    }

    // Erase the body of M::foo to make it fail verification.
    // Publish this modified version of M and the VM should fail to load it.
    {
        let mut storage = InMemoryStorage::new();

        let mut m = m;
        m.function_defs[0].code.as_mut().unwrap().code = vec![];
        let mut blob = vec![];
        m.serialize(&mut blob).unwrap();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());

        let err = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err.status_type(), StatusType::InvariantViolation);
    }
}

#[test]
fn test_missing_module_dependency() {
    // Compile two modules M, N where N depends on M.
    let code = r#"
        module {{ADDR}}::M {
            public fun foo() {}
        }

        module {{ADDR}}::N {
            use {{ADDR}}::M;

            public fun bar() { M::foo(); }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();
    let n = as_module(units.pop().unwrap());
    let m = as_module(units.pop().unwrap());

    let mut blob_m = vec![];
    m.serialize(&mut blob_m).unwrap();
    let mut blob_n = vec![];
    n.serialize(&mut blob_n).unwrap();

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("N").unwrap());
    let fun_name = Identifier::new("bar").unwrap();

    // Publish M and N and call N::bar. Everything should work.
    {
        let mut storage = InMemoryStorage::new();

        storage.add_module_bytes(m.self_addr(), m.self_name(), blob_m.into());
        storage.add_module_bytes(n.self_addr(), n.self_name(), blob_n.clone().into());

        let result = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        );
        assert_ok!(result);
    }

    // Publish only N and try to call N::bar. The VM should fail to find M and raise
    // an invariant violation.
    {
        let mut storage = InMemoryStorage::new();
        storage.add_module_bytes(n.self_addr(), n.self_name(), blob_n.into());

        let err = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err.status_type(), StatusType::InvariantViolation);
    }
}

#[test]
fn test_malformed_module_dependency() {
    // Compile two modules M, N where N depends on M.
    let code = r#"
        module {{ADDR}}::M {
            public fun foo() {}
        }

        module {{ADDR}}::N {
            use {{ADDR}}::M;

            public fun bar() { M::foo(); }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();
    let n = as_module(units.pop().unwrap());
    let m = as_module(units.pop().unwrap());

    let mut blob_m = vec![];
    m.serialize(&mut blob_m).unwrap();
    let mut blob_n = vec![];
    n.serialize(&mut blob_n).unwrap();

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("N").unwrap());
    let fun_name = Identifier::new("bar").unwrap();

    // Publish M and N and call N::bar. Everything should work.
    {
        let mut storage = InMemoryStorage::new();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob_m.clone().into());
        storage.add_module_bytes(n.self_addr(), n.self_name(), blob_n.clone().into());

        let result = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        );
        assert_ok!(result);
    }

    // Publish N and a corrupted version of M and try to call N::bar, the VM should fail to load M.
    {
        blob_m[0] = 0xDE;
        blob_m[1] = 0xAD;
        blob_m[2] = 0xBE;
        blob_m[3] = 0xEF;

        let mut storage = InMemoryStorage::new();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob_m.into());
        storage.add_module_bytes(n.self_addr(), n.self_name(), blob_n.into());

        let err = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err.status_type(), StatusType::InvariantViolation);
    }
}

#[test]
fn test_unverifiable_module_dependency() {
    // Compile two modules M, N where N depends on M.
    let code = r#"
        module {{ADDR}}::M {
            public fun foo() {}
        }

        module {{ADDR}}::N {
            use {{ADDR}}::M;

            public fun bar() { M::foo(); }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));
    let mut units = compile_units(&code).unwrap();
    let n = as_module(units.pop().unwrap());
    let m = as_module(units.pop().unwrap());

    let mut blob_n = vec![];
    n.serialize(&mut blob_n).unwrap();

    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("N").unwrap());
    let fun_name = Identifier::new("bar").unwrap();

    // Publish M and N and call N::bar. Everything should work.
    {
        let mut blob_m = vec![];
        m.serialize(&mut blob_m).unwrap();

        let mut storage = InMemoryStorage::new();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob_m.into());
        storage.add_module_bytes(n.self_addr(), n.self_name(), blob_n.clone().into());

        let result = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        );
        assert_ok!(result);
    }

    // Publish N and an unverifiable version of M and try to call N::bar, the VM should fail to load M.
    {
        let mut m = m;
        m.function_defs[0].code.as_mut().unwrap().code = vec![];
        let mut blob_m = vec![];
        m.serialize(&mut blob_m).unwrap();

        let mut storage = InMemoryStorage::new();
        storage.add_module_bytes(m.self_addr(), m.self_name(), blob_m.into());
        storage.add_module_bytes(n.self_addr(), n.self_name(), blob_n.into());

        let err = execute_function_with_single_storage_for_test(
            &storage,
            &module_id,
            &fun_name,
            &[],
            vec![],
        )
        .unwrap_err();
        assert_eq!(err.status_type(), StatusType::InvariantViolation);
    }
}

struct BogusModuleStorage {
    runtime_environment: RuntimeEnvironment,
    bad_status_code: StatusCode,
}

impl WithRuntimeEnvironment for BogusModuleStorage {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        &self.runtime_environment
    }
}

impl ModuleBytesStorage for BogusModuleStorage {
    fn fetch_module_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        Err(PartialVMError::new(self.bad_status_code).finish(Location::Undefined))
    }
}

impl ResourceResolver for BogusModuleStorage {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        _address: &AccountAddress,
        _tag: &StructTag,
        _metadata: &[Metadata],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        unreachable!()
    }
}

// Need another bogus storage implementation to allow querying modules but not resources.
struct BogusResourceStorage {
    module_storage: InMemoryStorage,
    bad_status_code: StatusCode,
}

impl ResourceResolver for BogusResourceStorage {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        _address: &AccountAddress,
        _tag: &StructTag,
        _metadata: &[Metadata],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        Err(PartialVMError::new(self.bad_status_code))
    }
}

const LIST_OF_ERROR_CODES: &[StatusCode] = &[
    StatusCode::UNKNOWN_VALIDATION_STATUS,
    StatusCode::INVALID_SIGNATURE,
    StatusCode::UNKNOWN_VERIFICATION_ERROR,
    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
    StatusCode::UNKNOWN_BINARY_ERROR,
    StatusCode::UNKNOWN_RUNTIME_STATUS,
    StatusCode::UNKNOWN_STATUS,
];

#[test]
fn test_storage_returns_bogus_error_when_loading_module() {
    let module_id = ModuleId::new(TEST_ADDR, Identifier::new("N").unwrap());

    for error_code in LIST_OF_ERROR_CODES {
        let data_storage = BogusModuleStorage {
            runtime_environment: RuntimeEnvironment::new(vec![]),
            bad_status_code: *error_code,
        };
        let module_storage = data_storage.as_unsync_module_storage();

        let err = execute_function_for_test(
            &data_storage,
            &module_storage,
            &module_id,
            ident_str!("bar"),
            &[],
            vec![],
        )
        .unwrap_err();

        // TODO(loader_v2):
        //   Loader V2 remaps all deserialization and verification errors. Loader V1 does not
        //   remap them when module resolver is accessed, and only on verification steps.
        //   Strictly speaking, the storage would never return such an error so V2 behaviour is
        //   ok. Moreover, the fact that V1 still returns UNKNOWN_BINARY_ERROR and does not
        //   remap it is weird.
        if *error_code == StatusCode::UNKNOWN_VERIFICATION_ERROR {
            assert_eq!(err.major_status(), StatusCode::UNEXPECTED_VERIFIER_ERROR);
        } else if *error_code == StatusCode::UNKNOWN_BINARY_ERROR {
            assert_eq!(
                err.major_status(),
                StatusCode::UNEXPECTED_DESERIALIZATION_ERROR
            );
        } else {
            assert_eq!(err.major_status(), *error_code);
        }
    }
}

#[test]
fn test_storage_returns_bogus_error_when_loading_resource() {
    let code = r#"
        address std {
            module signer {
                native public fun borrow_address(s: &signer): &address;

                public fun address_of(s: &signer): address {
                    *borrow_address(s)
                }
            }
        }

        module {{ADDR}}::M {
            use std::signer;

            struct R has key {}

            public fun foo() {}

            public fun bar(sender: &signer) acquires R {
                _ = borrow_global<R>(signer::address_of(sender));
            }
        }
    "#;
    let code = code.replace("{{ADDR}}", &format!("0x{}", TEST_ADDR.to_hex()));

    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let s = as_module(units.pop().unwrap());
    let mut m_blob = vec![];
    let mut s_blob = vec![];
    m.serialize(&mut m_blob).unwrap();
    s.serialize(&mut s_blob).unwrap();

    let m_id = m.self_id();
    let foo_name = Identifier::new("foo").unwrap();
    let bar_name = Identifier::new("bar").unwrap();

    for error_code in LIST_OF_ERROR_CODES {
        let natives = move_stdlib::natives::all_natives(
            AccountAddress::from_hex_literal("0x1").unwrap(),
            move_stdlib::natives::GasParameters::zeros(),
        );
        let runtime_environment = RuntimeEnvironment::new(natives);

        let mut module_storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);
        module_storage.add_module_bytes(m.self_addr(), m.self_name(), m_blob.clone().into());
        module_storage.add_module_bytes(s.self_addr(), s.self_name(), s_blob.clone().into());

        let storage = BogusResourceStorage {
            module_storage,
            bad_status_code: *error_code,
        };
        let module_storage = storage.module_storage.as_unsync_module_storage();

        let result =
            execute_function_for_test(&storage, &module_storage, &m_id, &foo_name, &[], vec![]);
        assert_ok!(result);

        let err = execute_function_for_test(
            &storage,
            &module_storage,
            &m_id,
            &bar_name,
            &[],
            serialize_values(&vec![MoveValue::Signer(TEST_ADDR)]),
        )
        .unwrap_err();

        if *error_code == StatusCode::UNKNOWN_VERIFICATION_ERROR {
            // MoveVM maps `UNKNOWN_VERIFICATION_ERROR` to `VERIFICATION_ERROR`.
            assert_eq!(err.major_status(), StatusCode::VERIFICATION_ERROR);
        } else {
            assert_eq!(err.major_status(), *error_code);
        }
    }
}
