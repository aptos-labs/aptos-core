// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::ConditionKind,
    exp_generator::ExpGenerator,
    model::{FunctionEnv, GlobalEnv, StructEnv},
    ty::{Type, BOOL_TYPE},
};
use move_stackless_bytecode::{
    borrow_analysis::{BorrowAnnotation, BorrowInfo, WriteBackAction},
    function_data_builder::FunctionDataBuilder,
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{
        BorrowNode,
        Bytecode::{self, *},
        Operation,
    },
};
use std::collections::BTreeSet;

pub struct MemoryInstrumentationProcessor {}

impl MemoryInstrumentationProcessor {
    pub fn new() -> Box<Self> {
        Box::new(MemoryInstrumentationProcessor {})
    }
}

impl FunctionTargetProcessor for MemoryInstrumentationProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native_or_intrinsic() {
            return data;
        }
        let borrow_annotation = data
            .annotations
            .remove::<BorrowAnnotation>()
            .expect("borrow annotation");
        let mut builder = FunctionDataBuilder::new(func_env, data);
        let code = std::mem::take(&mut builder.data.code);
        let mut instrumenter: Instrumenter<'_> = Instrumenter::new(builder, &borrow_annotation);
        let mut offset = 0;
        while offset < code.len() {
            if let Prop(id, _, exp) = &code[offset] {
                let loop_invariant_attr = if instrumenter.builder.data.loop_invariants.contains(id)
                {
                    Some(id)
                } else {
                    None
                };
                let borrow_annotation_at = instrumenter
                    .borrow_annotation
                    .get_borrow_info_at(offset as CodeOffset)
                    .unwrap();
                let borrow_info = &borrow_annotation_at.before;
                let mut spec_instrs = vec![code[offset].clone()];
                let mut nodes = exp
                    .used_temporaries_with_types(func_env.module_env.env)
                    .iter()
                    .map(|(temp, e)| {
                        if e.is_reference() {
                            BorrowNode::Reference(*temp)
                        } else {
                            BorrowNode::LocalRoot(*temp)
                        }
                    })
                    .filter(|n| borrow_info.has_borrow(n))
                    .collect_vec();
                let mut used_memory = exp
                    .used_memory(func_env.module_env.env)
                    .iter()
                    .map(|(id, _)| BorrowNode::GlobalRoot(id.clone()))
                    .filter(|n| borrow_info.has_borrow(n))
                    .collect_vec();
                nodes.append(&mut used_memory);
                offset += 1;
                while offset < code.len() {
                    if let Prop(_, _, exp) = &code[offset] {
                        let nodes_exp = exp
                            .used_temporaries_with_types(func_env.module_env.env)
                            .iter()
                            .map(|(temp, _)| BorrowNode::LocalRoot(*temp))
                            .filter(|n| borrow_info.has_borrow(n))
                            .collect_vec();
                        for node in nodes_exp {
                            if !nodes.contains(&node) {
                                nodes.push(node);
                            }
                        }
                        let used_memory_exp = exp
                            .used_memory(func_env.module_env.env)
                            .iter()
                            .map(|(id, _)| BorrowNode::GlobalRoot(id.clone()))
                            .filter(|n| borrow_info.has_borrow(n))
                            .collect_vec();
                        nodes.append(&mut used_memory);
                        for node in used_memory_exp {
                            if !nodes.contains(&node) {
                                nodes.push(node);
                            }
                        }
                        spec_instrs.push(code[offset].clone());
                        offset += 1;
                        continue;
                    } else {
                        break;
                    }
                }
                // for node in &nodes {
                //     println!("node:{:?}", node);
                // }
                if loop_invariant_attr.is_some_and(|attr| {
                    !instrumenter
                        .builder
                        .data
                        .loop_invariant_write_back_map
                        .contains_key(attr)
                }) {
                    instrumenter
                        .builder
                        .data
                        .loop_invariant_write_back_map
                        .insert(*loop_invariant_attr.unwrap(), (borrow_info.clone(), nodes));
                } else {
                    instrumenter.instrument_write_back_for_spec(borrow_info, nodes);
                }
                for instr in spec_instrs {
                    //println!("instr:{:?}", instr);
                    instrumenter.builder.emit(instr);
                }
            } else {
                instrumenter.instrument(offset as CodeOffset, code[offset].clone());
                offset += 1;
            }
        }

        // for (code_offset, bytecode) in code.into_iter().enumerate() {
        //     instrumenter.instrument(code_offset as CodeOffset, bytecode);
        // }
        instrumenter.builder.data
    }

    fn name(&self) -> String {
        "memory_instr".to_string()
    }
}

pub struct Instrumenter<'a> {
    builder: FunctionDataBuilder<'a>,
    borrow_annotation: &'a BorrowAnnotation,
}

impl<'a> Instrumenter<'a> {
    pub fn new(builder: FunctionDataBuilder<'a>, borrow_annotation: &'a BorrowAnnotation) -> Self {
        Self {
            builder,
            borrow_annotation,
        }
    }

    fn instrument(&mut self, code_offset: CodeOffset, bytecode: Bytecode) {
        if bytecode.is_branching()
            || matches!(bytecode, Bytecode::Call(_, _, Operation::Drop, _, _))
        {
            // Add memory instrumentation before instruction.
            self.memory_instrumentation(code_offset, &bytecode);
            self.builder.emit(bytecode);
        } else {
            self.builder.emit(bytecode.clone());
            self.memory_instrumentation(code_offset, &bytecode);
        }
    }

    /// Determines whether the type needs a pack ref.
    fn is_pack_ref_ty(&self, ty: &Type) -> bool {
        use Type::*;
        let env = self.builder.global_env();
        match ty.skip_reference() {
            Struct(mid, sid, inst) => {
                self.is_pack_ref_struct(&env.get_struct_qid(mid.qualified(*sid)))
                    || inst.iter().any(|t| self.is_pack_ref_ty(t))
            },
            Vector(et) => self.is_pack_ref_ty(et.as_ref()),
            Primitive(_)
            | Tuple(_)
            | TypeParameter(_)
            | Reference(_, _)
            | Fun(..)
            | TypeDomain(_)
            | ResourceDomain(_, _, _)
            | Error
            | Var(_) => false,
        }
    }

    /// Determines whether the struct needs a pack ref.
    fn is_pack_ref_struct(&self, struct_env: &StructEnv<'_>) -> bool {
        struct_env.get_spec().any_kind(ConditionKind::StructInvariant)
        // If any of the fields has it, it inherits to the struct.
        ||  struct_env
            .get_fields()
            .any(|fe| self.is_pack_ref_ty(&fe.get_type()))
    }

    pub fn is_pack_ref_ty_self(ty: &Type, env: &GlobalEnv) -> bool {
        use Type::*;
        match ty.skip_reference() {
            Struct(mid, sid, inst) => {
                Self::is_pack_ref_struct_self(&env.get_struct_qid(mid.qualified(*sid)))
                    || inst.iter().any(|t| Self::is_pack_ref_ty_self(t, env))
            },
            Vector(et) => Self::is_pack_ref_ty_self(et.as_ref(), env),
            Primitive(_)
            | Tuple(_)
            | TypeParameter(_)
            | Reference(_, _)
            | Fun(..)
            | TypeDomain(_)
            | ResourceDomain(_, _, _)
            | Error
            | Var(_) => false,
        }
    }

    /// Determines whether the struct needs a pack ref.
    fn is_pack_ref_struct_self(struct_env: &StructEnv<'_>) -> bool {
        struct_env.get_spec().any_kind(ConditionKind::StructInvariant)
        // If any of the fields has it, it inherits to the struct.
        ||  struct_env
            .get_fields()
            .any(|fe| Self::is_pack_ref_ty_self(&fe.get_type(), struct_env.module_env.env))
    }

    /// Calculate the differentiating factor for a particular write-back chain (among the tree)
    pub fn get_differentiation_factors(
        tree: &[Vec<WriteBackAction>],
        index: usize,
    ) -> BTreeSet<usize> {
        // utility function to first the first different action among two chains
        fn index_of_first_different_action(
            base: &[WriteBackAction],
            another: &[WriteBackAction],
        ) -> usize {
            for ((i, a1), a2) in base.iter().enumerate().zip(another.iter()) {
                if a1 != a2 {
                    return i;
                }
            }
            unreachable!("Two write-back action chains cannot be exactly the same");
        }

        // derive all the borrow edges that uniquely differentiate this chain
        let base = &tree[index];
        let diffs = tree
            .iter()
            .enumerate()
            .filter_map(|(i, chain)| {
                if i == index {
                    None
                } else {
                    Some(index_of_first_different_action(base, chain))
                }
            })
            .collect();

        // return the indices of the actions that differentiate this borrow chain
        diffs
    }

    pub fn instrument_write_back_for_spec(
        &mut self,
        borrow_info: &BorrowInfo,
        nodes: Vec<BorrowNode>,
    ) {
        for node in nodes {
            let mut ancestors = vec![];
            borrow_info.collect_ancestor_trees_recursive_reverse(&node, vec![], &mut ancestors);

            let is_conditional = ancestors.len() > 1;
            for (chain_index, chain) in ancestors.iter().enumerate() {
                // sanity check: the src node of the first action must be the node itself
                //if !chain.is_empty() {
                assert_eq!(
                    chain
                        .last()
                        .expect("The write-back chain should contain at action")
                        .dst,
                    node.clone()
                );
                //}

                // decide on whether we need IsParent checks and how to instrument the checks
                let skip_label_opt = if is_conditional {
                    let factors = Self::get_differentiation_factors(&ancestors, chain_index);
                    let mut last_is_parent_temp = None;

                    for idx in factors {
                        let action = &chain[idx];
                        let temp = self.builder.new_temp(BOOL_TYPE.clone());
                        self.builder.emit_with(|id| {
                            Bytecode::Call(
                                id,
                                vec![temp],
                                Operation::IsParent(action.dst.clone(), action.edge.clone()),
                                vec![action.src],
                                None,
                            )
                        });

                        let combined_temp = match last_is_parent_temp {
                            None => temp,
                            Some(last_temp) => {
                                let temp_conjunction = self.builder.new_temp(BOOL_TYPE.clone());
                                self.builder.emit_with(|id| {
                                    Bytecode::Call(
                                        id,
                                        vec![temp_conjunction],
                                        Operation::And,
                                        vec![last_temp, temp],
                                        None,
                                    )
                                });
                                temp_conjunction
                            },
                        };
                        last_is_parent_temp = Some(combined_temp);
                    }

                    let update_label = self.builder.new_label();
                    let skip_label = self.builder.new_label();
                    self.builder.emit_with(|id| {
                        Bytecode::Branch(
                            id,
                            update_label,
                            skip_label,
                            last_is_parent_temp
                                .expect("There should be at least one IsParent call for a conditional write-back"),
                        )
                    });
                    self.builder
                        .emit_with(|id| Bytecode::Label(id, update_label));
                    Some(skip_label)
                } else {
                    None
                };

                // issue a chain of write-back actions
                for action in chain {
                    // decide if we need a pre-writeback pack-ref (i.e., data structure invariant checking)
                    let pre_writeback_check_opt = match &action.dst {
                        BorrowNode::LocalRoot(..) | BorrowNode::GlobalRoot(..) => {
                            // On write-back to a root, "pack" the reference, i.e. validate all its invariants.
                            let target = self.builder.get_target();
                            let ty = target.get_local_type(action.src);
                            if self.is_pack_ref_ty(ty) {
                                Some(action.src)
                            } else {
                                None
                            }
                        },
                        BorrowNode::Reference(..) => None,
                        BorrowNode::ReturnPlaceholder(..) => unreachable!("invalid placeholder"),
                    };
                    if let Some(idx) = pre_writeback_check_opt {
                        self.builder.emit_with(|id| {
                            Bytecode::Call(id, vec![], Operation::PackRefDeep, vec![idx], None)
                        });
                    }

                    // emit the write-back
                    self.builder.emit_with(|id| {
                        Bytecode::Call(
                            id,
                            vec![],
                            Operation::WriteBack(action.dst.clone(), action.edge.clone()),
                            vec![action.src],
                            None,
                        )
                    });

                    // add a trace for written back value if it's a user variable.
                    match action.dst {
                        BorrowNode::LocalRoot(temp) | BorrowNode::Reference(temp) => {
                            if temp < self.builder.fun_env.get_local_count().unwrap_or_default() {
                                self.builder.emit_with(|id| {
                                    Bytecode::Call(
                                        id,
                                        vec![],
                                        Operation::TraceLocal(temp),
                                        vec![temp],
                                        None,
                                    )
                                });
                            }
                        },
                        _ => {},
                    }
                }

                // continued from IsParent check
                if let Some(label) = skip_label_opt {
                    self.builder.emit_with(|id| Bytecode::Label(id, label));
                }
            }
        }
    }

    fn memory_instrumentation(&mut self, code_offset: CodeOffset, bytecode: &Bytecode) {
        let param_count = self.builder.get_target().get_parameter_count();

        let borrow_annotation_at = self
            .borrow_annotation
            .get_borrow_info_at(code_offset)
            .unwrap();
        let before = &borrow_annotation_at.before;
        let after = &borrow_annotation_at.after;

        // if let SpecBlock(id, spec) = bytecode {
        //     println!("spec block:{:?}", spec);
        // }

        // if let Prop(id, kind, exp) = bytecode {
        //     println!("propkind:{:?}, exp:{}", kind, exp.display(self.builder.fun_env.module_env.env));
        //     println!("borrow info before:{:?}", before);
        //     println!("borrow info after:{:?}", after);
        // }

        // Generate UnpackRef from Borrow instructions.
        if let Call(attr_id, dests, op, _, _) = bytecode {
            use Operation::*;
            match op {
                BorrowLoc | BorrowField(..) | BorrowGlobal(..) | BorrowVariantField(..) => {
                    let ty = &self
                        .builder
                        .get_target()
                        .get_local_type(dests[0])
                        .to_owned();
                    let node = BorrowNode::Reference(dests[0]);
                    if self.is_pack_ref_ty(ty) && after.is_in_use(&node) {
                        self.builder.set_loc_from_attr(*attr_id);
                        self.builder.emit_with(|id| {
                            Bytecode::Call(id, vec![], Operation::UnpackRef, vec![dests[0]], None)
                        });
                    }
                },
                _ => {},
            }
        }

        // Generate PackRef for nodes which go out of scope, as well as WriteBack.
        let attr_id = bytecode.get_attr_id();
        self.builder.set_loc_from_attr(attr_id);

        for (node, ancestors) in before.dying_nodes(after) {
            // we only care about references that occurs in the function body
            let node_idx = match node {
                BorrowNode::LocalRoot(..) | BorrowNode::GlobalRoot(..) => {
                    continue;
                },
                BorrowNode::Reference(idx) => {
                    if idx < param_count {
                        // NOTE: we have an entry-point assumption where a &mut parameter must
                        // have its data invariants hold. As a result, when we write-back the
                        // references, we should assert that the data invariant still hold.
                        //
                        // This, however, does not apply to &mut references we obtained in the
                        // function body, i.e., by borrow local or borrow global. These cases
                        // are handled by the `pre_writeback_check_opt` (see below).
                        let target = self.builder.get_target();
                        let ty = target.get_local_type(idx);
                        if self.is_pack_ref_ty(ty) {
                            self.builder.emit_with(|id| {
                                Bytecode::Call(id, vec![], Operation::PackRefDeep, vec![idx], None)
                            });
                        }
                        continue;
                    }
                    idx
                },
                BorrowNode::ReturnPlaceholder(..) => {
                    unreachable!("Unexpected placeholder borrow node");
                },
            };

            // Generate write_back for this reference.
            let is_conditional = ancestors.len() > 1;
            for (chain_index, chain) in ancestors.iter().enumerate() {
                // sanity check: the src node of the first action must be the node itself
                assert_eq!(
                    chain
                        .first()
                        .expect("The write-back chain should contain at action")
                        .src,
                    node_idx
                );

                // decide on whether we need IsParent checks and how to instrument the checks
                let skip_label_opt = if is_conditional {
                    let factors = Self::get_differentiation_factors(&ancestors, chain_index);
                    let mut last_is_parent_temp = None;

                    for idx in factors {
                        let action = &chain[idx];
                        let temp = self.builder.new_temp(BOOL_TYPE.clone());
                        self.builder.emit_with(|id| {
                            Bytecode::Call(
                                id,
                                vec![temp],
                                Operation::IsParent(action.dst.clone(), action.edge.clone()),
                                vec![action.src],
                                None,
                            )
                        });

                        let combined_temp = match last_is_parent_temp {
                            None => temp,
                            Some(last_temp) => {
                                let temp_conjunction = self.builder.new_temp(BOOL_TYPE.clone());
                                self.builder.emit_with(|id| {
                                    Bytecode::Call(
                                        id,
                                        vec![temp_conjunction],
                                        Operation::And,
                                        vec![last_temp, temp],
                                        None,
                                    )
                                });
                                temp_conjunction
                            },
                        };
                        last_is_parent_temp = Some(combined_temp);
                    }

                    let update_label = self.builder.new_label();
                    let skip_label = self.builder.new_label();
                    self.builder.emit_with(|id| {
                        Bytecode::Branch(
                            id,
                            update_label,
                            skip_label,
                            last_is_parent_temp
                                .expect("There should be at least one IsParent call for a conditional write-back"),
                        )
                    });
                    self.builder
                        .emit_with(|id| Bytecode::Label(id, update_label));
                    Some(skip_label)
                } else {
                    None
                };

                // issue a chain of write-back actions
                for action in chain {
                    // decide if we need a pre-writeback pack-ref (i.e., data structure invariant checking)
                    let pre_writeback_check_opt = match &action.dst {
                        BorrowNode::LocalRoot(..) | BorrowNode::GlobalRoot(..) => {
                            // On write-back to a root, "pack" the reference, i.e. validate all its invariants.
                            let target = self.builder.get_target();
                            let ty = target.get_local_type(action.src);
                            if self.is_pack_ref_ty(ty) {
                                Some(action.src)
                            } else {
                                None
                            }
                        },
                        BorrowNode::Reference(..) => None,
                        BorrowNode::ReturnPlaceholder(..) => unreachable!("invalid placeholder"),
                    };
                    if let Some(idx) = pre_writeback_check_opt {
                        self.builder.emit_with(|id| {
                            Bytecode::Call(id, vec![], Operation::PackRefDeep, vec![idx], None)
                        });
                    }

                    // emit the write-back
                    self.builder.emit_with(|id| {
                        Bytecode::Call(
                            id,
                            vec![],
                            Operation::WriteBack(action.dst.clone(), action.edge.clone()),
                            vec![action.src],
                            None,
                        )
                    });

                    // add a trace for written back value if it's a user variable.
                    match action.dst {
                        BorrowNode::LocalRoot(temp) | BorrowNode::Reference(temp) => {
                            if temp < self.builder.fun_env.get_local_count().unwrap_or_default() {
                                self.builder.emit_with(|id| {
                                    Bytecode::Call(
                                        id,
                                        vec![],
                                        Operation::TraceLocal(temp),
                                        vec![temp],
                                        None,
                                    )
                                });
                            }
                        },
                        _ => {},
                    }
                }

                // continued from IsParent check
                if let Some(label) = skip_label_opt {
                    self.builder.emit_with(|id| Bytecode::Label(id, label));
                }
            }
        }
    }
}
