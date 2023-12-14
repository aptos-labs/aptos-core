pub mod context;
pub mod core;
pub mod manager;
pub mod rules;
pub mod visitor;
use self::{
    manager::VisitorManager,
    rules::{
        bool_comparison::BoolComparisonVisitor, borrow_deref_ref::BorrowDerefRefVisitor,
        complex_inline_function::ComplexInlineFunctionVisitor, deep_nesting::DeepNestingVisitor,
        double_bool_comparison::DoubleComparisonsVisitor,
        double_parentheses::DoubleParenthesesVisitor, empty_loop::EmptyLoopVisitor,
        ifs_same_cond::IfsSameCondVisitor,
        multiplication_before_division::MultiplicationBeforeDivisionVisitor,
        redundant_deref_ref::RedundantDerefRefVisitor, shift_overflow::ShiftOverflowVisitor,
        unconditional_exit_loop::UnconditionalExitLoopVisitor,
        unnecessary_type_conversion::UnnecessaryTypeConversionVisitor,
        unused_borrow_global_mut::UnusedBorrowGlobalMutVisitor,
        unused_private_function::UnusedFunctionVisitor,
    },
};
use std::path::PathBuf;

pub fn main(path: PathBuf) {
    let env = core::main(Some(path)).unwrap();
    let mut manager = VisitorManager::new(vec![
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
        RedundantDerefRefVisitor::visitor(),
        ComplexInlineFunctionVisitor::visitor(),
    ]);

    manager.run(env);
}
