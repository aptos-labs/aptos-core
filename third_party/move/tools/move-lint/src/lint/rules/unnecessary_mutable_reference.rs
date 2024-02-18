//! Check for unnecessary mutable references obtained from data structures such as vector, table,
//! etc. that are created but no data is actually modified.
use crate::lint::{utils::add_diagnostic_and_emit, visitor::ExpressionAnalysisVisitor};
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FunctionEnv, GlobalEnv, ModuleEnv},
    ty::{ReferenceKind, Type},
};

#[derive(Debug)]
pub struct UnnecessaryMutableReferenceLint;

impl Default for UnnecessaryMutableReferenceLint {
    fn default() -> Self {
        Self::new()
    }
}

impl UnnecessaryMutableReferenceLint {
    fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_unnecessary_mutable_references(&self, module: &ModuleEnv, env: &GlobalEnv) {
        for func_env in module.get_functions() {
            if let Some(body) = func_env.get_def().as_ref() {
                body.visit_pre_post(
                    &mut (|up: bool, exp: &ExpData| {
                        if !up {
                            if let ExpData::Call(
                                _,
                                Operation::MoveFunction(module_id, fun_id),
                                args,
                            ) = exp
                            {
                                let func_env = env.get_function(module_id.qualified(*fun_id));
                                self.check_arguments(args, &func_env, env);
                            }
                        }
                    }),
                );
            }
        }
    }

    fn check_arguments(&self, args: &[Exp], func_env: &FunctionEnv, env: &GlobalEnv) {
        for (index, arg) in args.iter().enumerate() {
            if let ExpData::Call(node_id, Operation::Borrow(ReferenceKind::Mutable), _) =
                arg.as_ref()
            {
                if !self.is_mutable_required(index, func_env) {
                    let location = env.get_node_loc(*node_id);
                    let message = "Unnecessary mutable reference detected. Consider using an immutable reference instead.";
                    add_diagnostic_and_emit(
                        &location,
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                    );
                }
            }
        }
    }

    fn is_mutable_required(&self, index: usize, func_env: &FunctionEnv) -> bool {
        matches!(func_env.get_parameter_types().get(index), Some(Type::Reference(ReferenceKind::Mutable, _)))
    }
}

impl ExpressionAnalysisVisitor for UnnecessaryMutableReferenceLint {
    fn visit_module(&mut self, module: &ModuleEnv, env: &GlobalEnv) {
        self.check_unnecessary_mutable_references(module, env);
    }
}
