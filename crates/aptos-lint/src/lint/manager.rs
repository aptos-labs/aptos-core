use move_model::model::GlobalEnv;

use super::visitor::ExpDataVisitor;

pub struct VisitorManager {
    linters: Vec<Box<dyn ExpDataVisitor>>,
}

impl VisitorManager {
    pub fn new(linters: Vec<Box<dyn ExpDataVisitor>>) -> Self {
        Self { linters }
    }

    pub fn run(&mut self, env: GlobalEnv) {
        for module_env in &env.get_target_modules() {
            for linter in &mut self.linters {
                linter.visit_module(&module_env, &env);
                for func_env in module_env.get_functions() {
                    linter.visit(&func_env, &env);
                }
            }
        }
    }
}
