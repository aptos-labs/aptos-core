use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use move_model::ast::{Exp, ExpData, Operation, Pattern, Value};
use move_model::model::{FunId, FunctionEnv, GlobalEnv, ModuleId};
use move_model::symbol::Symbol;
use num_bigint::BigInt;

pub struct OutOfBoundsArrayIndexingVisitor;

impl Default for OutOfBoundsArrayIndexingVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl OutOfBoundsArrayIndexingVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_out_of_bounds_indexing(
        &self,
        exp: &ExpData,
        env: &GlobalEnv,
        arr_length: usize,
        assigned_symbol: &Symbol,
    ) {
        if let ExpData::Call(_, Operation::MoveFunction(mid, fid), args) = exp {
            let func_env = self.get_function_env(env, *mid, *fid);
            if self.is_vector_borrow(&func_env, env) {
                self.check_vector_borrow_args(args, env, arr_length, assigned_symbol, exp);
            }
        }
    }

    /// Retrieves the function environment for the given module and function ID.
    fn get_function_env<'a>(
        &self,
        env: &'a GlobalEnv,
        module_id: ModuleId,
        function_id: FunId,
    ) -> FunctionEnv<'a> {
        let module_env = env.get_module(module_id);
        module_env.into_function(function_id)
    }

    /// Checks the arguments of a vector borrow operation for out-of-bounds access.
    fn check_vector_borrow_args(
        &self,
        args: &Vec<Exp>,
        env: &GlobalEnv,
        arr_length: usize,
        assigned_symbol: &Symbol,
        exp: &ExpData,
    ) {
        if args.len() > 1 {
            if let ExpData::Value(_, Value::Number(index)) = args[1].as_ref() {
                if let ExpData::Call(_, Operation::Borrow(_), sub_args) = args[0].as_ref() {
                    if let ExpData::LocalVar(_, symbol) = sub_args[0].as_ref() {
                        if symbol == assigned_symbol
                            && self.is_index_out_of_bounds(index, arr_length)
                        {
                            self.emit_out_of_bounds_warning(exp, env);
                        }
                    }
                }
            }
        }
    }

    /// Checks if the given index is out of bounds for the array length.
    fn is_index_out_of_bounds(&self, index: &BigInt, arr_length: usize) -> bool {
        *index >= BigInt::from(arr_length)
    }

    /// Emits a warning for an out-of-bounds access attempt.
    fn emit_out_of_bounds_warning(&self, exp: &ExpData, env: &GlobalEnv) {
        let message = "Array index out of bounds detected in vector::borrow.";
        add_diagnostic_and_emit(
            &env.get_node_loc(exp.node_id()),
            message,
            codespan_reporting::diagnostic::Severity::Warning,
            env,
        );
    }

    /// Checks if the function call is `vector::borrow`.
    fn is_vector_borrow(&self, func_env: &FunctionEnv, global_env: &GlobalEnv) -> bool {
        // Assuming 'vector' is a standard library module, you would need to adjust these accordingly
        if func_env.module_env.self_address() == &global_env.get_stdlib_address()
            && global_env
                .symbol_pool()
                .string(func_env.module_env.get_name().name())
                .to_string()
                == *"vector".to_string()
        {
            return func_env.get_name_str() == "borrow" || func_env.get_name_str() == "borrow_mut";
        }
        false
    }
}

impl ExpressionAnalysisVisitor for OutOfBoundsArrayIndexingVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
    ) {
        if let ExpData::Block(_, Pattern::Var(_, symbol), some_binding_exp, _) = exp {
            let binding_exp = some_binding_exp.as_ref().expect("binding_exp");
            if let ExpData::Call(_, Operation::Vector, arr_length) = binding_exp.as_ref() {
                exp.visit_pre_post(&mut |is_pre_visit, exp: &ExpData| {
                    if !is_pre_visit {
                        self.check_out_of_bounds_indexing(exp, env, arr_length.len(), symbol);
                    }
                });
            }
        }
    }
}
