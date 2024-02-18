//! Detect borrow_global_mut variables that are not actually used to modify any data.
use crate::lint::utils::add_diagnostic_and_emit;
use crate::lint::visitor::ExpressionAnalysisVisitor;
use move_model::model::{FunctionEnv, GlobalEnv};
use move_stackless_bytecode::function_target::FunctionTarget;
use move_stackless_bytecode::stackless_bytecode::{AttrId, Bytecode, Operation};
use move_stackless_bytecode::stackless_bytecode_generator::StacklessBytecodeGenerator;
use std::collections::{BTreeMap, HashSet};

// /Struct representing the visitor for detecting unused mutable variables.
#[derive(Debug)]
pub struct UnusedBorrowGlobalMutVisitor {}

impl Default for UnusedBorrowGlobalMutVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl UnusedBorrowGlobalMutVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn process_bytecode(
        &mut self,
        bytecode: &Bytecode,
        func_target: &FunctionTarget,
    ) -> (
        BTreeMap<usize, AttrId>,
        BTreeMap<usize, usize>,
        HashSet<usize>,
    ) {
        let mut borrow_mut_refs = BTreeMap::new();
        let mut modified = HashSet::new();
        let mut borrow_fields: BTreeMap<usize, usize> = BTreeMap::new();

        match bytecode {
            Bytecode::Call(att_id, _, Operation::BorrowGlobal(_, _, _), _, _) => {
                let (_, mut_mods) = bytecode.modifies(func_target);
                for (idx, is_mutable_reference) in mut_mods {
                    if is_mutable_reference {
                        borrow_mut_refs.insert(idx, *att_id);
                    }
                }
            },
            Bytecode::Call(_, dest, Operation::BorrowField(_, _, _, _), srcs, _) => {
                for (des, src) in dest.iter().zip(srcs.iter()) {
                    borrow_fields.insert(*src, *des);
                }
            },
            Bytecode::Call(_, _, Operation::WriteRef, _, _) => {
                let (_, mut_mods) = bytecode.modifies(func_target);
                for (idx, _) in mut_mods {
                    modified.insert(idx);
                }
            },
            _ => {},
        }

        (borrow_mut_refs, borrow_fields, modified)
    }

    fn find_unused_borrow_mut_refs(
        &self,
        all_borrow_mut_refs: &BTreeMap<usize, AttrId>,
        borrow_fields: BTreeMap<usize, usize>,
        modified: HashSet<usize>,
    ) -> Vec<usize> {
        all_borrow_mut_refs
            .iter()
            .filter_map(|(&ref_id, _)| {
                let is_modified = if let Some(&mapped_ref) = borrow_fields.get(&ref_id) {
                    modified.contains(&mapped_ref)
                } else {
                    modified.contains(&ref_id)
                };

                if !is_modified {
                    Some(ref_id)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl ExpressionAnalysisVisitor for UnusedBorrowGlobalMutVisitor {
    fn requires_bytecode_inspection(&self) -> bool {
        true
    }
    fn visit_function_with_bytecode(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        if func_env.is_inline() {
            return;
        }
        let data = StacklessBytecodeGenerator::new(func_env).generate_function();
        // Handle to work with stackless functions -- function targets.
        let target = FunctionTarget::new(&func_env, &data);
        let byte_codes = target.get_bytecode();

        let mut all_borrow_mut_refs = BTreeMap::new();
        let mut borrow_fields_refs = BTreeMap::new();
        let mut modified = HashSet::new();

        for bytecode in byte_codes {
            let (borrow_mut_refs, borrow_fields, mods) = self.process_bytecode(bytecode, &target);
            all_borrow_mut_refs.extend(borrow_mut_refs);
            modified.extend(mods);
            borrow_fields_refs.extend(borrow_fields);
        }

        let unused_borrow_mut_refs =
            self.find_unused_borrow_mut_refs(&all_borrow_mut_refs, borrow_fields_refs, modified);
        for idx in unused_borrow_mut_refs {
            let message = "Unused borrowed mutable variable. Consider normal borrow (borrow_global, vector::borrow, etc.) instead";
            add_diagnostic_and_emit(
                &target.get_bytecode_loc(*all_borrow_mut_refs.get(&idx).unwrap()),
                message,
                codespan_reporting::diagnostic::Severity::Warning,
                env,
            );
        }
    }
}
