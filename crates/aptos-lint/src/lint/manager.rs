use move_compiler::FullyCompiledProgram;

use super::{visitor::LintVisitor, context::VisitorContext};

pub struct VisitorManager<'a> {
    linters: Vec<Box<dyn LintVisitor + 'a>>,
}

impl<'a> VisitorManager<'a> {
    pub fn new(linters: Vec<Box<dyn LintVisitor + 'a>>) -> Self {
        Self { linters }
    }

    pub fn run(&mut self, custom_ast: FullyCompiledProgram, context: &mut VisitorContext) {
        for (_, _, module) in &custom_ast.typing.modules {
            
            for linter in &mut self.linters {
                linter.visit_module(module, context);
                for (_, _, function) in &module.functions {
                    linter.visit_function(function, context);
                }
            }
        }
    }
}
