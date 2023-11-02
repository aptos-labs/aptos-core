// The visitor identifies functions that are declared but not used 
// (i.e., functions that are defined but not called anywhere within the module).

use std::collections::BTreeMap;

use move_compiler::typing::ast::{ Function, ModuleDefinition };
use move_compiler::typing::ast as AST;
use move_compiler::expansion::ast as AST2;
use move_ir_types::location::Loc;
use crate::lint::context::VisitorContext;
use crate::lint::visitor::{ LintVisitor, LintUtilities };

pub struct UnusedFunctionVisitor {
    declared_functions: BTreeMap<String, move_ir_types::location::Loc>,
    called_functions: BTreeMap<String, move_ir_types::location::Loc>,
}

impl UnusedFunctionVisitor {
    pub fn new() -> Self {
        Self {
            declared_functions: BTreeMap::new(),
            called_functions: BTreeMap::new(),
        }
    }

    pub fn visitor() -> Box<dyn LintVisitor> {
        Box::new(Self::new())
    }

    fn register_function_declaration(&mut self, func_name: &str, loc: Loc) {
        self.declared_functions.insert(func_name.to_string(), loc);
    }

    fn register_function_call(&mut self, func_name: &str, loc: Loc) {
        self.called_functions.insert(func_name.to_string(), loc);
    }

    fn check_unused_functions(&mut self, context: &mut VisitorContext) {
        let unused_function_names: Vec<_> = self.declared_functions
            .iter()
            .filter(|(fname, _)| !self.called_functions.contains_key(*fname))
            .map(|(fname, loc)| (fname.clone(), loc.clone()))
            .collect();
        for (fname, loc) in unused_function_names {
            let message = format!("Function '{}' is unused.", fname);
            self.add_warning(context, &loc, &message);
        }
    }
    fn check_function_calls(&mut self, function: &Function) {
        match &function.body.value {
            AST::FunctionBody_::Defined(block) => {
                for seq in block {
                    match &seq.value {
                        AST::SequenceItem_::Seq(exp) => {
                            if let AST::UnannotatedExp_::ModuleCall(call) = &exp.exp.value {
                                let func_key = format!("{}", &call.name.0.value);
                                self.register_function_call(&func_key, exp.exp.loc);
                            }
                        }
                        AST::SequenceItem_::Bind(_, _, exp) => {
                            if let AST::UnannotatedExp_::ModuleCall(call) = &exp.exp.value {
                                let func_key = format!("{}", &call.name.0.value);
                                self.register_function_call(&func_key, exp.exp.loc);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => (),
        }
    }
}

impl LintVisitor for UnusedFunctionVisitor {
    fn visit_module(&mut self, module: &ModuleDefinition, context: &mut VisitorContext) {
        for (loc, fname, func) in &module.functions {
            if func.visibility == AST2::Visibility::Internal && !func.entry.is_some() {
                self.register_function_declaration(fname, loc);
            }
        }

        for (_, _, func) in &module.functions {
            self.check_function_calls(&func);
        }
        self.check_unused_functions(context);
    }
}

impl LintUtilities for UnusedFunctionVisitor {}
