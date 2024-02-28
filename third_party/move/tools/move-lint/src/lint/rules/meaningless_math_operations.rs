//! `MeaninglessMathOperationsVisitor` detects and warns about operations in Move programs that have no effect, such as adding zero.
//! It aims to improve code clarity by identifying operations that can be simplified or removed.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::ast::{Exp, ExpData, Operation, Value};
use move_model::model::{FunctionEnv, GlobalEnv};
use num_bigint::BigInt;
pub struct MeaninglessMathOperationsVisitor;

impl Default for MeaninglessMathOperationsVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl MeaninglessMathOperationsVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    /// Checks for meaningless math operations.
    fn check_meaningless_math_operations(
        &self,
        exp: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Call(_, oper, args) = exp {
            if self.is_meaningless_operation(oper, args) {
                let message = "Detected a meaningless mathematical operation.";
                add_diagnostic_and_emit(
                    &env.get_node_loc(exp.node_id()),
                    message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                    diags,
                );
            }
        }
    }

    /// Determines if the operation is meaningless.
    fn is_meaningless_operation(&self, oper: &Operation, args: &[Exp]) -> bool {
        args.iter().any(|arg| match arg.as_ref() {
            ExpData::Value(_, Value::Number(num)) => {
                let big_int_zero = BigInt::from(0);
                let big_int_one = BigInt::from(1);
                match (oper, num) {
                    (Operation::Add, num) => num == &big_int_zero,
                    (Operation::Sub, num) => num == &big_int_zero,
                    (Operation::Mul, num) => num == &big_int_one,
                    (Operation::Div, num) => num == &big_int_one,
                    (Operation::Mod, num) => num == &big_int_one,
                    (Operation::BitAnd, num) => num == &big_int_zero,
                    (Operation::BitOr, num) => num == &big_int_zero,
                    (Operation::Xor, num) => num == &big_int_zero,
                    (Operation::Shl, num) => num == &big_int_zero,
                    (Operation::Shr, num) => num == &big_int_zero,
                    _ => false,
                }
            },
            _ => false,
        })
    }
}

impl ExpressionAnalysisVisitor for MeaninglessMathOperationsVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_meaningless_math_operations(exp, env, diags);
    }
}
