//! Detect cases where a variable is being cast to the same type it already has.
//! Such type conversions are redundant and can be omitted for cleaner and more readable code.
use move_model::ast::{ExpData, Operation};
use move_model::model::{FunctionEnv, GlobalEnv, NodeId};
use move_model::ty::{PrimitiveType, Type};

use crate::lint::utils::{add_diagnostic_and_emit, get_var_info_from_func_param, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;

pub struct UnnecessaryTypeConversionVisitor;

impl Default for UnnecessaryTypeConversionVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl UnnecessaryTypeConversionVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn prepare_and_add_diagnostic_and_emit(
        &mut self,
        ty: PrimitiveType,
        var_name: String,
        env: &GlobalEnv,
        node_id: NodeId,
    ) {
        let message = &format!(
            "Unnecessary type conversion detected. '{}' is already of type '{}'. Avoid casting it to its own type.",
            var_name,
            ty
        );
        add_diagnostic_and_emit(
            &env.get_node_loc(node_id),
            message,
            codespan_reporting::diagnostic::Severity::Warning,
            env,
        );
    }

    fn get_variable_info(
        &self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
    ) -> Option<(String, Type)> {
        match exp {
            ExpData::LocalVar(_, symbol) => Some((
                env.symbol_pool().string(*symbol).to_string(),
                env.get_node_type(exp.node_id()),
            )),
            ExpData::Temporary(_, index) => {
                let parameters = func_env.get_parameters();
                get_var_info_from_func_param(*index, &parameters).map(|param| {
                    (
                        env.symbol_pool().string(param.0).to_string(),
                        env.get_node_type(exp.node_id()),
                    )
                })
            },
            _ => None,
        }
    }

    fn check_unnecessary_conversion(
        &mut self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
    ) {
        if let ExpData::Call(node_id, Operation::Cast, args) = exp {
            // Checking if an expression is a type cast operation.
            // If the original type and the target type of the cast operation are the same,
            // a warning is generated because such a cast is redundant.
            if let Some(var_info) = self.get_variable_info(&args[0], func_env, env) {
                let cast_type = env.get_node_type(*node_id);
                if var_info.1 == cast_type {
                    if let Type::Primitive(ty) = cast_type {
                        self.prepare_and_add_diagnostic_and_emit(ty, var_info.0, env, *node_id);
                    }
                }
            }
        }
    }
}

impl ExpressionAnalysisVisitor for UnnecessaryTypeConversionVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
    ) {
        self.check_unnecessary_conversion(exp, func_env, env);
    }
}
