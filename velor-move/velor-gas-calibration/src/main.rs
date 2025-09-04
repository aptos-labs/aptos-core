// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod math;
mod math_interface;
mod measurements;
mod measurements_helpers;
mod solve;
use velor_abstract_gas_usage::{aggregate_terms, expand_terms};
use velor_gas_algebra::DynamicExpression;
use clap::Parser;
use math_interface::{convert_to_matrix_format, total_num_of_cols, total_num_rows};
use measurements::compile_and_run;
use solve::{build_coefficient_matrix, build_constant_matrix, least_squares};
use std::collections::BTreeMap;

/// Automated Gas Calibration to calibrate Move bytecode and Native Functions
#[derive(Parser, Debug)]
struct Args {
    /// Specific Calibration Function tests to run that match a given pattern
    #[clap(short, long, default_value = "")]
    pattern: String,

    /// Number of iterations to run each Calibration Function
    #[clap(short, long, default_value_t = 20)]
    iterations: u64,

    /// Maximum execution time in milliseconds
    #[clap(short, long, default_value_t = 300)]
    max_execution_time: u64,
}

fn main() {
    // Implement CLI
    let args = Args::parse();
    let pattern = &args.pattern;
    let iterations = args.iterations;
    let max_execution_time = args.max_execution_time;

    println!(
        "Running each Calibration Function for {} iterations\n",
        iterations
    );

    println!("Calibrating Gas Parameters ...\n");

    let measurements = compile_and_run(iterations, pattern);

    let mut system_of_equations: Vec<Vec<DynamicExpression>> = Vec::new();
    for formula in measurements.abstract_meter {
        let mut terms: Vec<DynamicExpression> = Vec::new();
        for term in formula {
            let normal = expand_terms(term);
            terms.extend(normal);
        }
        system_of_equations.push(terms);
    }

    // Collect like terms
    let mut mappings: Vec<BTreeMap<String, u64>> = Vec::new();
    for equation in system_of_equations {
        let map = aggregate_terms(equation)
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
    let mut const_matrix = build_constant_matrix(measurements.regular_meter, nrows, vec_col);

    // Solve the system of linear equations
    least_squares(
        mappings,
        &mut coeff_matrix,
        &mut const_matrix,
        measurements.equation_names,
        max_execution_time,
    );
}
