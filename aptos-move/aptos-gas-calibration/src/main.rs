// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//mod algebra;
//mod algebra_helpers;
mod benchmark;
mod benchmark_helpers;
mod math;
mod math_interface;
mod modified_gas_meter;
mod solve;
//use aptos_gas_algebra::GasAdd;
//use aptos_gas_meter::GasAlgebra;
//use aptos_gas_schedule::gas_params::instr;
//use aptos_gas_algebra::Expression;
//use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
//use aptos_native_interface::{Expression, SafeNativeBuilder};
use aptos_abstract_gas_usage::{collect_terms, normalize};
use aptos_gas_algebra::Expression;
use benchmark::benchmark_calibration_function;
use math_interface::{convert_to_matrix_format, total_num_of_cols, total_num_rows};
use modified_gas_meter::get_abstract_gas_usage;
use solve::{build_coefficient_matrix, build_constant_matrix, solve};
use std::collections::BTreeMap;
//use move_core_types::{account_address::AccountAddress, ident_str};
//use std::sync::{Arc, Mutex};

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

    /*
     * @notice: Normalize terms into addition of Vec<Expression>
     * @return: A vector holding the system of linear equations
     */
    let mut system_of_equations: Vec<Vec<Expression>> = Vec::new();
    println!("\n\nabstract gas formulae (LHS): ");
    for formula in abstract_gas_formulae {
        let mut terms: Vec<Expression> = Vec::new();
        for term in formula {
            let normal = normalize(term);
            terms.extend(normal);
        }
        system_of_equations.push(terms);
    }

    /*
     * @notice: Collect like terms
     * @return: Simple mapping to interface with math helpers
     */
    let mut mappings: Vec<BTreeMap<String, u64>> = Vec::new();
    for equation in system_of_equations {
        let map = collect_terms(equation);
        mappings.push(map);
    }

    /*
     * @notice: Convert simplified map to a math friendly interface
     * @return vec_format: A format used to easily call the math functions
     */
    let vec_format: Vec<Vec<f64>> = convert_to_matrix_format(mappings.clone());

    /*
     * @notice: Build the system of linear equations using the math library
     */
    let nrows = total_num_rows(mappings.clone());
    let ncols = total_num_of_cols(mappings.clone());
    let vec_col: usize = 1;

    let mut coeff_matrix = build_coefficient_matrix(vec_format, nrows, ncols);
    let mut const_matrix = build_constant_matrix(running_times, nrows, vec_col);

    /*
     * @notice: Solve the system of linear equations
     */
    solve(mappings, &mut coeff_matrix, &mut const_matrix);
}
