use move_model::{ model::{ FunctionEnv, GlobalEnv }, ast::{ ExpData, Operation } };
use crate::lint::visitor::{ ExpDataVisitor, LintUtilities };

#[derive(Debug)]
pub struct BorrowDerefRefVisitor;

impl BorrowDerefRefVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_borrow_deref_ref(&self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Call(_, Operation::Borrow(_), exp_vec) = exp {
            if let Some(ExpData::Call(_, Operation::Deref, inner_exp_vec)) = exp_vec.get(0).map(|e| e.as_ref()) {
                if let Some(ExpData::Call(_, Operation::Borrow(_), _)) = inner_exp_vec.get(0).map(|e| e.as_ref()) {
                    let message = "Redundant borrow detected. Consider removing the borrow operation and using the variable directly.";
                    self.add_diagnostic_and_emit(
                        &env.get_node_loc(exp.node_id()),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env
                    );
                }
            }
        }
    }
}

impl ExpDataVisitor for BorrowDerefRefVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        func_env.get_def().map(|func| {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.check_borrow_deref_ref(exp, env);
                    }
                })
            );
        });
    }
}

impl LintUtilities for BorrowDerefRefVisitor {}
