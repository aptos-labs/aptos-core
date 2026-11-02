// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The loader rejects Move bodies whose qualified names have registered
//! natives. Lowering would otherwise run the native in place of the body,
//! bypassing the Move frame and its reentrancy checks.
//!
//! This check requires the callee module to be loaded. Cross-module native
//! dispatch does not load the callee and bypasses the check.

use mono_move_core::VMInternalError;
use mono_move_loader::LoaderError;
use mono_move_testsuite::{with_loaded_mono_function, SourceKind};
use move_core_types::{account_address::AccountAddress, ident_str};

/// `0x1::test_natives::u64_add` is a registered toy native; this gives it a
/// Move body.
const SOURCE: &str = r#"
module 0x1::test_natives {
    public fun u64_add(a: u64, b: u64): u64 { a + b }
}
"#;

#[test]
fn module_with_a_registered_native_name_fails_to_load() {
    let err = with_loaded_mono_function(
        SOURCE,
        SourceKind::Move,
        AccountAddress::ONE,
        ident_str!("test_natives"),
        ident_str!("u64_add"),
        |_| (),
    )
    .expect_err("loading the shadowed module must fail");
    let vm_err = err
        .downcast_ref::<VMInternalError>()
        .expect("the load failure carries the VM error");
    assert!(
        matches!(
            vm_err.downcast_ref::<LoaderError>(),
            Some(LoaderError::NativeShadowsMoveFunction { module, name, .. })
                if module == "test_natives" && name == "u64_add"
        ),
        "unexpected error: {vm_err}"
    );
}
