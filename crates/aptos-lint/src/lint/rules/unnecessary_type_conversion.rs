use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect cases where a variable is being cast to the same type it already has.
/// Such type conversions are redundant and can be omitted for cleaner and more readable code.
use move_model::ast::{ExpData, Operation};
use move_model::{
    model::{FunctionEnv, GlobalEnv, NodeId},
    ty::{PrimitiveType, Type},
};

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

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
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
        self.add_diagnostic_and_emit(
            &env.get_node_loc(node_id),
            message,
            codespan_reporting::diagnostic::Severity::Warning,
            env,
        );
    }

    fn check_unnecessary_conversion(
        &mut self,
        exp: &ExpData,

        func_env: &FunctionEnv,
        env: &GlobalEnv,
    ) {
        if let ExpData::Call(node_id, Operation::Cast, vec_exp) = exp {
            // Checking if an expression is a type cast operation.
            // If the original type and the target type of the cast operation are the same,
            // a warning is generated because such a cast is redundant.
            match vec_exp[0].as_ref() {
                ExpData::LocalVar(_, symbol) => {
                    let var_name = env.symbol_pool().string(*symbol);
                    let cast_type = env.get_node_type(*node_id);
                    let arg_type = env.get_node_type(vec_exp[0].node_id());
                    if arg_type == cast_type {
                        if let Type::Primitive(ty) = cast_type {
                            self.prepare_and_add_diagnostic_and_emit(
                                ty,
                                var_name.to_string(),
                                env,
                                *node_id,
                            );
                        }
                    };
                },
                ExpData::Temporary(_, index) => {
                    let params = self
                        .get_var_info_from_func_param(index, func_env.get_parameters())
                        .unwrap();
                    let var_name = env.symbol_pool().string(params.0);
                    let cast_type = env.get_node_type(*node_id);
                    let arg_type = env.get_node_type(vec_exp[0].node_id());
                    if arg_type == cast_type {
                        if let Type::Primitive(ty) = cast_type {
                            self.prepare_and_add_diagnostic_and_emit(
                                ty,
                                var_name.to_string(),
                                env,
                                *node_id,
                            );
                        }
                    }
                },
                _ => (),
            }
        }
    }
}

impl ExpDataVisitor for UnnecessaryTypeConversionVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        let mut visitor = |exp: &ExpData| {
            self.check_unnecessary_conversion(exp, func_env, env);
        };
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit(&mut visitor);
        };
    }
}

impl LintUtilities for UnnecessaryTypeConversionVisitor {}
