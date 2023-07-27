// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod gas_meter;
mod gas_meter_helpers;
mod math;
mod math_interface;
mod solve;
use aptos_abstract_gas_usage::{collect_terms, normalize};
use aptos_gas_algebra::DynamicExpression;
use gas_meter::{compile_and_run_samples, compile_and_run_samples_ir};
use math_interface::{convert_to_matrix_format, total_num_of_cols, total_num_rows};
use solve::{build_coefficient_matrix, build_constant_matrix, solve};
use std::collections::BTreeMap;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    println!("Calibrating Gas Parameters ...\n");

    let samples = compile_and_run_samples();
    let samples_ir = compile_and_run_samples_ir();

    let mut equation_names: Vec<String> = Vec::new();
    equation_names.extend(samples.equation_names);
    equation_names.extend(samples_ir.equation_names);

    let mut system_of_equations: Vec<Vec<DynamicExpression>> = Vec::new();
    for formula in samples.abstract_meter {
        let mut terms: Vec<DynamicExpression> = Vec::new();
        for term in formula {
            let normal = normalize(term);
            terms.extend(normal);
        }
        system_of_equations.push(terms);
    }
    for formula in samples_ir.abstract_meter {
        let mut terms: Vec<DynamicExpression> = Vec::new();
        for term in formula {
            let normal = normalize(term);
            terms.extend(normal);
        }
        system_of_equations.push(terms);
    }

    // Collect like terms
    let mut mappings: Vec<BTreeMap<String, u64>> = Vec::new();
    for equation in system_of_equations {
        let map = collect_terms(equation)
            .expect("Failed: Should not have concrete quantities in gas formulae.");
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
    solve(
        mappings,
        &mut coeff_matrix,
        &mut const_matrix,
        equation_names,
    );
}
