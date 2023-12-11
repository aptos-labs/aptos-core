pub mod manager;
pub mod visitor;
pub mod context;
pub mod rules;
pub mod core;
use std::path::PathBuf;

use self::{
    manager::VisitorManager,
    rules::{
        bool_comparison::BoolComparisonVisitor,
        ifs_same_cond::IfsSameCondVisitor,
        double_bool_comparison::DoubleComparisonsVisitor,
        unnecessary_type_conversion::UnnecessaryTypeConversionVisitor,
        unused_private_function::UnusedFunctionVisitor,
        shift_overflow::ShiftOverflowVisitor,
        multiplication_before_division::MultiplicationBeforeDivisionVisitor,
        unused_borrow_global_mut::UnusedBorrowGlobalMutVisitor,
        deep_nesting::DeepNestingVisitor,
        empty_loop::EmptyLoopVisitor,
        unconditional_exit_loop::UnconditionalExitLoopVisitor,
        double_parentheses::DoubleParenthesesVisitor,
        borrow_deref_ref::BorrowDerefRefVisitor,
        redundant_deref_ref::RedundantDerefRefVisitor,
    },
};

pub fn main(path: PathBuf) {
    let env = core::main(Some(path)).unwrap();
    let mut manager = VisitorManager::new(
        vec![
            BoolComparisonVisitor::visitor(),
            DoubleComparisonsVisitor::visitor(),
            IfsSameCondVisitor::visitor(),
            MultiplicationBeforeDivisionVisitor::visitor(),
            ShiftOverflowVisitor::visitor(),
            UnnecessaryTypeConversionVisitor::visitor(),
            UnusedFunctionVisitor::visitor(),
            ShiftOverflowVisitor::visitor(),
            MultiplicationBeforeDivisionVisitor::visitor(),
            UnusedBorrowGlobalMutVisitor::visitor(),
            DeepNestingVisitor::visitor(),
            EmptyLoopVisitor::visitor(),
            UnconditionalExitLoopVisitor::visitor(),
            DoubleParenthesesVisitor::visitor(),
            BorrowDerefRefVisitor::visitor(),
            RedundantDerefRefVisitor::visitor()
        ]
    );

    manager.run(env);
}
