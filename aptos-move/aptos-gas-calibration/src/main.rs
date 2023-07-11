// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod algebra;
mod algebra_helpers;
mod types;
mod visitor;
use algebra::CalibrationAlgebra;
use aptos_gas_algebra::GasAdd;
use aptos_gas_meter::{GasAlgebra, StandardGasAlgebra};
use aptos_gas_schedule::{gas_params::instr, VMGasParameters};
use aptos_vm_types::storage::StorageGasParameters;
use std::collections::BTreeMap;

/*fn native_test_simple(gas_core: &mut impl GasAlgebra, v: &[u64]) -> anyhow::Result<u64> {
    gas_core.charge_execution(
        instr::LD_CONST_BASE + instr::LD_CONST_PER_BYTE * NumBytes::new(v.len() as u64),
    )?;
    Ok(v.iter().sum())
}

fn native_test_mul(gas_core: &mut impl GasAlgebra, v: &[u64]) -> anyhow::Result<u64> {
    gas_core.charge_execution(instr::LD_CONST_PER_BYTE * NumBytes::new(v.len() as u64))?;
    Ok(v.iter().sum())
}

fn native_test_mul_reverse(gas_core: &mut impl GasAlgebra, v: &[u64]) -> anyhow::Result<u64> {
    gas_core.charge_execution(GasMul {
        left: NumBytes::new(v.len() as u64),
        right: instr::LD_CONST_PER_BYTE,
    })?;
    Ok(v.iter().sum())
}*/

fn native_test_add(gas_core: &mut impl GasAlgebra, v: &[u64]) -> anyhow::Result<u64> {
    gas_core.charge_execution(GasAdd {
        left: instr::LD_CONST_BASE,
        right: instr::LD_CONST_BASE,
    })?;
    Ok(v.iter().sum())
}

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let mut gas_core = CalibrationAlgebra {
        base: StandardGasAlgebra::new(
            10,
            VMGasParameters::zeros(),
            StorageGasParameters::free_and_unlimited(),
            10000,
        ),
        coeff_buffer: BTreeMap::new(),
    };

    //native_test_simple(&mut gas_core, &[1, 2, 3]).unwrap();

    //native_test_mul(&mut gas_core, &[1, 2, 3]).unwrap();
    //native_test_mul_reverse(&mut gas_core, &[1, 2, 3]).unwrap();

    // native_test_add(&mut gas_core, &[1, 2, 3]).unwrap();
}
