// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//mod algebra;
//mod algebra_helpers;
mod benchmark;
mod benchmark_helpers;
mod modified_gas_meter;
//use aptos_gas_algebra::GasAdd;
//use aptos_gas_meter::GasAlgebra;
//use aptos_gas_schedule::gas_params::instr;
//use aptos_gas_algebra::Expression;
//use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
//use aptos_native_interface::{Expression, SafeNativeBuilder};
use benchmark::benchmark_calibration_function;
use modified_gas_meter::get_abstract_gas_usage;
//use move_core_types::{account_address::AccountAddress, ident_str};
//use std::sync::{Arc, Mutex};

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
}
fn native_test_add(gas_core: &mut impl GasAlgebra, v: &[u64]) -> anyhow::Result<u64> {
    gas_core.charge_execution(GasAdd {
        left: instr::LD_CONST_BASE,
        right: instr::LD_CONST_BASE,
    })?;
    Ok(v.iter().sum())
}*/

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    /*
     * @notice: Run with Regular Gas Meter to get running time
     * @return: f64 representing the running time
     */
    let running_times = benchmark_calibration_function();
    println!("running times (RHS): {:?}", running_times);

    /*
     * @notice: Run with Modified Gas Meter to get Gas Formula
     * @return: Simplified Map of coefficients and gas parameters
     */
    let abstract_gas_formulae = get_abstract_gas_usage();
    println!("\n\nabstract gas formulae (LHS): {:?}", abstract_gas_formulae);

    /*let mut gas_core = CalibrationAlgebra {
        base: StandardGasAlgebra::new(
            10,
            VMGasParameters::zeros(),
            StorageGasParameters::free_and_unlimited(),
            10000,
        ),
        coeff_buffer: BTreeMap::new(),
    };*/
    //native_test_simple(&mut gas_core, &[1, 2, 3]).unwrap();
    //native_test_mul(&mut gas_core, &[1, 2, 3]).unwrap();
    //native_test_mul_reverse(&mut gas_core, &[1, 2, 3]).unwrap();
    //native_test_add(&mut gas_core, &[1, 2, 3]).unwrap();

    /*
     * Access shared buffer
     */
    //// TODO
}
