//! Check for unsafe x * y / z expression that can lead to over flow if x and y are large enough.
//! Developers should use math64::mul_div or math128::mul_div instead which casts to the next
//! larger type before doing the multiplication to avoid overflow.
use crate::lint::{
    utils::{add_diagnostic_and_emit, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use move_model::{
    ast::{ExpData, Operation},
    model::{FunctionEnv, GlobalEnv},
    ty::{PrimitiveType, Type},
};

#[derive(Debug)]
pub struct UseMulDivLint;

impl Default for UseMulDivLint {
    fn default() -> Self {
        Self::new()
    }
}

impl UseMulDivLint {
    fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_expression(&self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Call(_, Operation::Div, vec_exp_div) = exp {
            if let Some(mul_exp) = vec_exp_div.get(0) {
                if let ExpData::Call(_, Operation::Mul, vec_exp_mul) = mul_exp.as_ref() {
                    if vec_exp_mul.len() == 2 && self.is_u64_or_u128(&vec_exp_mul[0], env) &&
                        self.is_u64_or_u128(&vec_exp_mul[1], env) &&
                        self.is_u64_or_u128(&vec_exp_div[1], env) {
                            let message = "Use math64::mul_div or math128::mul_div instead of mul/div.";
                        add_diagnostic_and_emit(
                            &env.get_node_loc(exp.node_id()),
                            message,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                        );
                    }
                }
            }
        }
    }

    fn is_u64_or_u128(&self, exp: &ExpData, env: &GlobalEnv) -> bool {
        let exp_type = env.get_node_type(exp.node_id());
        matches!(
            exp_type,
            Type::Primitive(PrimitiveType::U64) | Type::Primitive(PrimitiveType::U128)
        )
    }
}

impl ExpressionAnalysisVisitor for UseMulDivLint {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
    ) {
        self.check_expression(exp, env);
    }
}
