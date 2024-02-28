//! The `RedundantBooleanExpressions` lint checks for boolean expressions
//! in Move code that are unnecessarily complex and can be simplified.
//! It focuses on identifying patterns where a boolean value (`true` or `false`)
//! is used in conjunction with logical operators (`&&` or `||`) in a way that
//! does not affect the outcome of the expression.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use codespan::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use move_model::ast::{Exp, ExpData, Operation, Value};
use move_model::model::{FunctionEnv, GlobalEnv};

pub struct RedundantBooleanExpressions;

impl Default for RedundantBooleanExpressions {
    fn default() -> Self {
        Self::new()
    }
}

impl RedundantBooleanExpressions {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    /// Checks for redundant boolean expressions.
    fn check_redundant_boolean_expressions(
        &self,
        exp: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Call(_, oper, args) = exp {
            if self.is_redundant_boolean_expression(oper, args) {
                let message = "Redundant boolean expression detected. Consider simplifying it.";
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

    /// Determines if the boolean expression is redundant.
    fn is_redundant_boolean_expression(&self, operation: &Operation, args: &[Exp]) -> bool {
        if args.len() != 2 {
            return false;
        }

        // Extract the underlying ExpData from each argument
        let arg1_data = args[0].as_ref();
        let arg2_data = args[1].as_ref();

        match (arg1_data, arg2_data) {
            // Check for expressions like `x && true` or `true && x`
            (ExpData::Value(_, Value::Bool(true)), _)
            | (_, ExpData::Value(_, Value::Bool(true)))
                if operation == &Operation::And =>
            {
                true
            },
            // Check for expressions like `x || false` or `false || x`
            (ExpData::Value(_, Value::Bool(false)), _)
            | (_, ExpData::Value(_, Value::Bool(false)))
                if operation == &Operation::Or =>
            {
                true
            },
            _ => false,
        }
    }
}

impl ExpressionAnalysisVisitor for RedundantBooleanExpressions {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_redundant_boolean_expressions(exp, env, diags);
    }
}
