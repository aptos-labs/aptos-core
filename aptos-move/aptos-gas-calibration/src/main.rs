// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod gas_meter;
mod gas_meter_helpers;
mod math;
mod math_interface;
mod solve;
use aptos_abstract_gas_usage::{collect_terms, normalize};
use aptos_gas_algebra::Expression;
use gas_meter::compile_and_run_samples_ir;
use math_interface::{convert_to_matrix_format, total_num_of_cols, total_num_rows};
use solve::{build_coefficient_matrix, build_constant_matrix, solve};
use std::collections::BTreeMap;

/*
 * Error types:
 *
 * - Impercise gas models (i.e., inject a dummy term into abstract usage)
 * - Measurement errors (i.e., loading of module, loading of vm, etc.)
 * - Samples not running long enough (i.e., run the simple ones in loops)
 */

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");

    let samples_ir = compile_and_run_samples_ir();
    //let samples_ir = compile_and_run_samples();

    println!("\n\nabstract gas formulae (LHS): ");
    let mut system_of_equations: Vec<Vec<Expression>> = Vec::new();
    for formula in samples_ir.abstract_meter {
        let mut terms: Vec<Expression> = Vec::new();
        for term in formula {
            let normal = normalize(term);
            terms.extend(normal);
        }
        system_of_equations.push(terms);
    }

    // Collect like terms
    let mut mappings: Vec<BTreeMap<String, u64>> = Vec::new();
    for equation in system_of_equations {
        let map = collect_terms(equation);
        mappings.push(map);
    }

    // Convert simplified map to a math friendly interface
    let vec_format: Vec<Vec<f64>> = convert_to_matrix_format(mappings.clone());

    // Build the system of linear equations using the math library
    let nrows = total_num_rows(mappings.clone());
    let ncols = total_num_of_cols(mappings.clone());
    let vec_col: usize = 1;

    let mut coeff_matrix = build_coefficient_matrix(vec_format, nrows, ncols);
    let mut const_matrix = build_constant_matrix(samples_ir.regular_meter, nrows, vec_col);

    // Solve the system of linear equations
    solve(mappings, &mut coeff_matrix, &mut const_matrix);
}
