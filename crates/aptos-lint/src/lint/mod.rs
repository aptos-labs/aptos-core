pub mod manager;
pub mod visitor;
pub mod context;
pub mod rules;
pub mod core;
use std::path::PathBuf;

use self::{manager::VisitorManager, rules::{ifs_same_cond::IfsSameCondVisitor, bool_comparison::BoolComparisonVisitor, double_bool_comparison::DoubleComparisonsVisitor, unnecessary_type_conversion::UnnecessaryTypeConversionVisitor, unused_private_function::UnusedFunctionVisitor, shift_overflow::ShiftOverflowVisitor, multiplication_before_division::MultiplicationBeforeDivisionVisitor}, context::VisitorContext};

pub fn main(path: PathBuf) -> anyhow::Result<VisitorContext> {
    let ast = core::main(path).unwrap();

    let mut context: VisitorContext = VisitorContext::new(ast.clone());
    let mut manager = VisitorManager::new(vec![
        BoolComparisonVisitor::visitor(),
        DoubleComparisonsVisitor::visitor(),
        IfsSameCondVisitor::visitor(),
        MultiplicationBeforeDivisionVisitor::visitor(),
        ShiftOverflowVisitor::visitor(),
        UnnecessaryTypeConversionVisitor::visitor(),
        UnusedFunctionVisitor::visitor(),
    ]);
    
    manager.run(ast, &mut context);
    anyhow::Ok(context)
}