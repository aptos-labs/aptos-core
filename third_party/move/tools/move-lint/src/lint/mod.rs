/// Handles the management and execution of linters.
/// It uses a 'VisitorManager' struct to manage a collection of linters, each implementing the 'ExpressionAnalysisVisitor' trait.
pub mod manager;

/// Implements various traits and utilities for linting and analysis.
/// This module defines traits and methods that allow for visiting different elements of a program,
/// such as modules, functions, and expressions, and performing checks or analysis on them.
pub mod visitor;

/// Contains definitions and implementations of various lint rules.
pub mod rules;

/// Contains helper functions for lint rules.
pub mod utils;

/// Focused on the compiling move code into GlobalEnv
pub mod build;
use std::path::PathBuf;

use self::{
    manager::VisitorManager,
    rules::{
        bool_comparison::BoolComparisonVisitor,
        check_redundant_boolean_expressions::RedundantBooleanExpressions,
        combinable_bool_conditions::CombinableBoolVisitor,
        complex_inline_function::ComplexInlineFunctionVisitor,
        constant_naming::ConstantNamingVisitor, deep_nesting::DeepNestingVisitor,
        empty_loop::EmptyLoopVisitor, explicit_self_assignments::ExplicitSelfAssignmentsVisitor,
        getter_method_field_match::GetterMethodFieldMatchLint, ifs_same_cond::IfsSameCondVisitor,
        infinite_loop_detector::InfiniteLoopDetectorVisitor,
        meaningless_math_operations::MeaninglessMathOperationsVisitor,
        multiplication_before_division::MultiplicationBeforeDivisionVisitor,
        out_of_bounds_array_indexing::OutOfBoundsArrayIndexingVisitor,
        overflow_multiplication_detector::OverflowMultiplicationDetectorVisitor,
        redundant_deref_ref::RedundantDerefRefVisitor,
        redundant_ref_deref::RedundantRefDerefVisitor,
        return_at_end_of_block::ReturnAtEndOfBlockVisitor, shift_overflow::ShiftOverflowVisitor,
        sorted_imports::SortedImportsLint, unconditional_exit_loop::UnconditionalExitLoopVisitor,
        unmodified_mutable_argument_lint::UnmodifiedMutableArgumentLint,
        unnecessary_mutable_reference::UnnecessaryMutableReferenceLint,
        unnecessary_type_conversion::UnnecessaryTypeConversionVisitor,
        unnecessary_while_true::UnnecessaryWhileTrueVisitor,
        unused_borrow_global_mut::UnusedBorrowGlobalMutVisitor, use_mul_div::UseMulDivLint,
    },
    utils::read_config,
};

pub fn main(path: PathBuf) {
    let lint_config = read_config(&path).expect("Failed to read config file.");
    let env = build::build_ast(Some(path))
        .expect("Failed to initialize environment. Expected a valid path with necessary data.");
    let mut manager = VisitorManager::new(vec![
        BoolComparisonVisitor::visitor(),
        RedundantRefDerefVisitor::visitor(),
        ComplexInlineFunctionVisitor::visitor(),
        DeepNestingVisitor::visitor(),
        CombinableBoolVisitor::visitor(),
        EmptyLoopVisitor::visitor(),
        GetterMethodFieldMatchLint::visitor(),
        IfsSameCondVisitor::visitor(),
        MultiplicationBeforeDivisionVisitor::visitor(),
        MultiplicationBeforeDivisionVisitor::visitor(),
        RedundantDerefRefVisitor::visitor(),
        ShiftOverflowVisitor::visitor(),
        ShiftOverflowVisitor::visitor(),
        SortedImportsLint::visitor(),
        UnconditionalExitLoopVisitor::visitor(),
        UnmodifiedMutableArgumentLint::visitor(),
        UnnecessaryMutableReferenceLint::visitor(),
        UnnecessaryTypeConversionVisitor::visitor(),
        UnusedBorrowGlobalMutVisitor::visitor(),
        UseMulDivLint::visitor(),
        ConstantNamingVisitor::visitor(),
        ReturnAtEndOfBlockVisitor::visitor(),
        MeaninglessMathOperationsVisitor::visitor(),
        OutOfBoundsArrayIndexingVisitor::visitor(),
        RedundantBooleanExpressions::visitor(),
        ExplicitSelfAssignmentsVisitor::visitor(),
        UnnecessaryWhileTrueVisitor::visitor(),
        InfiniteLoopDetectorVisitor::visitor(),
        OverflowMultiplicationDetectorVisitor::visitor(),
    ]);

    manager.run(env, &lint_config);
}
