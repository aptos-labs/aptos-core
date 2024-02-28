//! Comparisons where a value is compared exactly twice with different relational operators
//! inside a logical OR operation. For example, expressions like `a == b || a < b` or `x != y || x > y`
//! can potentially be combined to simplify the code.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::ast::{ExpData, Operation, Value};
use move_model::model::{FunctionEnv, GlobalEnv};

pub struct CombinableBoolVisitor;

impl Default for CombinableBoolVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CombinableBoolVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    // Check if the given expression contains two comparisons that can be combined.
    fn find_combinable_comparison(
        &mut self,
        cond: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Call(_, Operation::Or, args) = cond {
            if let (ExpData::Call(_, op1, args1), ExpData::Call(_, op2, args2)) =
                (&args[0].as_ref(), &args[1].as_ref())
            {
                if args1.len() == 2 && args2.len() == 2 {
                    let right_operand_1 = &args1[1];
                    let right_operand_2 = &args2[1];
                    if let (
                        ExpData::Value(_, Value::Number(num1)),
                        ExpData::Value(_, Value::Number(num2)),
                    ) = (right_operand_1.as_ref(), right_operand_2.as_ref())
                    {
                        if num1 != num2 {
                            return;
                        }
                    }
                }
                let left = &mut args[0].used_temporaries(env);
                let right = &mut args[1].used_temporaries(env);
                left.sort();
                right.sort();
                if left == right {
                    let operation_pairs = [
                        (
                            (&Operation::Eq, &Operation::Lt),
                            "Simplify comparison by using <= instead.",
                        ),
                        (
                            (&Operation::Lt, &Operation::Eq),
                            "Simplify comparison by using <= instead.",
                        ),
                        (
                            (&Operation::Eq, &Operation::Gt),
                            "Simplify comparison by using >= instead.",
                        ),
                        (
                            (&Operation::Gt, &Operation::Eq),
                            "Simplify comparison by using >= instead.",
                        ),
                        (
                            (&Operation::Neq, &Operation::Lt),
                            "Unequal (!=) condition is unnecessary and can be removed",
                        ),
                        (
                            (&Operation::Lt, &Operation::Neq),
                            "Unequal (!=) condition is unnecessary and can be removed",
                        ),
                        (
                            (&Operation::Neq, &Operation::Gt),
                            "Unequal (!=) condition is unnecessary and can be removed",
                        ),
                        (
                            (&Operation::Gt, &Operation::Neq),
                            "Unequal (!=) condition is unnecessary and can be removed",
                        ),
                    ];
                    if let Some(message) = operation_pairs
                        .iter()
                        .find(|(pair, _)| pair == &(op1, op2))
                        .map(|&(_, message)| message)
                    {
                        add_diagnostic_and_emit(
                            &env.get_node_loc(cond.node_id()),
                            message,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                            diags,
                        );
                    }
                }
            }
        }
    }
}

impl ExpressionAnalysisVisitor for CombinableBoolVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::IfElse(_, cond, _, _) = exp {
            self.find_combinable_comparison(cond.as_ref(), env, diags);
        }
    }
}
