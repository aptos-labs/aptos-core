// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test-only natives for the `std::unit_test` module.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Opaque, Vector},
    VMResult,
};
use move_core_types::account_address::AccountAddress;

/// `0x1::unit_test::create_signers_for_testing(num_signers: u64): vector<signer>`
///
/// Test-only. Returns `num_signers` signers whose addresses are the little-endian
/// encoding of `0, 1, ..., num_signers - 1`.
//
// TODO(metering): charge gas and bound `num_signers`.
pub fn native_create_signers_for_testing<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `u64`.
    let num_signers: u64 = unsafe { ctx.arg(0)? };

    // A `signer` has the same 32-byte layout as its `address`, and addresses are
    // pointer-free, so pack the addresses back-to-back and build the vector once.
    let mut data = Vec::with_capacity(num_signers as usize * AccountAddress::LENGTH);
    for i in 0..num_signers {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[..8].copy_from_slice(&i.to_le_bytes());
        data.extend_from_slice(&bytes);
    }
    let signers: Vector<Opaque> =
        ctx.new_vector_no_pointers(AccountAddress::LENGTH as u32, num_signers, &data)?;
    // SAFETY: return 0 is `vector<signer>`, whose element layout is a bare address.
    unsafe { ctx.set_return(0, signers)? };
    Ok(NativeStatus::Success)
}

/// Test-only natives for the `unit_test` module.
pub fn make_all_unit_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![(
        "0x1::unit_test::create_signers_for_testing",
        native_create_signers_for_testing
    )]
}
