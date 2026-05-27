// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{ConditionKind, Exp},
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
            // When seeing a spec clause, processing all consecutive ones
            if let Prop(..) = &code[offset] {
                process_spec_clauses(&code, func_env, &mut instrumenter, &mut offset);
            } else {
                instrumenter.instrument(offset as CodeOffset, code[offset].clone());
                offset += 1;
            }
        }
        instrumenter.builder.data
    }

    fn name(&self) -> String {
        "memory_instr".to_string()
    }
}

fn process_spec_clauses(
    code: &[Bytecode],
    func_env: &FunctionEnv,
    instrumenter: &mut Instrumenter<'_>,
    offset: &mut usize,
) {
    let first_attr_id = match &code[*offset] {
        Prop(attr_id, _, _) => attr_id,
        _ => unreachable!("Expected a Prop"),
    };

    // Get the borrow info before the first spec clause.
    let borrow_annotation_at = instrumenter
        .borrow_annotation
        .get_borrow_info_at(*offset as CodeOffset)
        .unwrap();
    let borrow_info = &borrow_annotation_at.before;

    let mut spec_instrs = vec![];
    let mut nodes = BTreeSet::new();

    // Process all clauses
    while *offset < code.len() {
        if let Prop(_, _, exp) = &code[*offset] {
            collect_borrow_nodes_from_exp(&mut nodes, func_env, borrow_info, exp);
            spec_instrs.push(code[*offset].clone());
            *offset += 1;
        } else {
            break;
        }
    }

    // We only need to check the attribute of the first clause
    // because it can represent a set of loop invariants in the same loop
    // and write-back actions will be inserted before it.
    let loop_invariant_attr = if instrumenter
        .builder
        .data
        .loop_invariants
        .contains(first_attr_id)
    {
        Some(first_attr_id)
    } else {
        None
    };

    // Either record write-back actions for loop invariants, or instrument them directly
    match loop_invariant_attr {
        Some(attr)
            if !instrumenter
                .builder
                .data
                .loop_invariant_write_back_map
                .contains_key(attr) =>
        {
            instrumenter
                .builder
                .data
                .loop_invariant_write_back_map
                .insert(*attr, (borrow_info.clone(), nodes));
        },
        _ => {
            Instrumenter::instrument_write_back_for_spec(
                &mut instrumenter.builder,
                borrow_info,
                nodes,
            );
        },
    }

    // Emit all the spec instructions.
    for instr in spec_instrs {
        instrumenter.builder.emit(instr);
    }
}

fn collect_borrow_nodes_from_exp(
    nodes: &mut BTreeSet<BorrowNode>,
    func_env: &FunctionEnv,
    borrow_info: &BorrowInfo,
    exp: &Exp,
) {
    let env = func_env.env();
    exp.used_temporaries_with_types(env)
        .iter()
        .map(|(temp, e)| {
            if e.is_reference() {
                BorrowNode::Reference(*temp)
            } else {
                BorrowNode::LocalRoot(*temp)
            }
        })
        .chain(
            exp.used_memory(env)
                .iter()
                .map(|(id, _)| BorrowNode::GlobalRoot(id.clone())),
        )
        .filter(|node| borrow_info.has_borrow(node))
        .for_each(|node| {
            nodes.insert(node);
        });
}

pub struct Instrumenter<'a> {
    pub builder: FunctionDataBuilder<'a>,
    borrow_annotation: &'a BorrowAnnotation,
}

impl<'a> Instrumenter<'a> {
    fn new(builder: FunctionDataBuilder<'a>, borrow_annotation: &'a BorrowAnnotation) -> Self {
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
        Self::is_pack_ref_ty_(ty, self.builder.global_env())
    }

    fn is_pack_ref_ty_(ty: &Type, env: &GlobalEnv) -> bool {
        use Type::*;
        match ty.skip_reference() {
            Struct(mid, sid, inst) => {
                Self::is_pack_ref_struct_(&env.get_struct_qid(mid.qualified(*sid)))
                    || inst.iter().any(|t| Self::is_pack_ref_ty_(t, env))
            },
            Vector(et) => Self::is_pack_ref_ty_(et.as_ref(), env),
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
    fn is_pack_ref_struct_(struct_env: &StructEnv<'_>) -> bool {
        struct_env.get_spec().any_kind(ConditionKind::StructInvariant)
        // If any of the fields has it, it inherits to the struct.
        ||  struct_env
            .get_fields()
            .any(|fe| Self::is_pack_ref_ty_(&fe.get_type(), struct_env.env()))
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
        // return the indices of the actions that differentiate this borrow chain
        tree.iter()
            .enumerate()
            .filter_map(|(i, chain)| {
                if i == index {
                    None
                } else {
                    Some(index_of_first_different_action(base, chain))
                }
            })
            .collect()
    }

    fn write_back_chain(
        builder: &mut FunctionDataBuilder,
        ancestors: &[Vec<WriteBackAction>],
        chain_index: usize,
        is_conditional: bool,
    ) {
        let chain = &ancestors[chain_index];
        // decide on whether we need IsParent checks and how to instrument the checks
        let skip_label_opt = if is_conditional {
            let factors = Self::get_differentiation_factors(ancestors, chain_index);
            let mut last_is_parent_temp = None;

            for idx in factors {
                let action = &chain[idx];
                let temp = builder.new_temp(BOOL_TYPE.clone());
                builder.emit_with(|id| {
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
                        let temp_conjunction = builder.new_temp(BOOL_TYPE.clone());
                        builder.emit_with(|id| {
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

            let update_label = builder.new_label();
            let skip_label = builder.new_label();
            builder.emit_with(|id| {
                Bytecode::Branch(
                    id,
                    update_label,
                    skip_label,
                    last_is_parent_temp.expect(
                        "There should be at least one IsParent call for a conditional write-back",
                    ),
                )
            });
            builder.emit_with(|id| Bytecode::Label(id, update_label));
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
                    let target = builder.get_target();
                    let ty = target.get_local_type(action.src);
                    if Instrumenter::is_pack_ref_ty_(ty, target.global_env()) {
                        Some(action.src)
                    } else {
                        None
                    }
                },
                BorrowNode::Reference(..) => None,
                BorrowNode::ReturnPlaceholder(..) => unreachable!("invalid placeholder"),
            };
            if let Some(idx) = pre_writeback_check_opt {
                builder.emit_with(|id| {
                    Bytecode::Call(id, vec![], Operation::PackRefDeep, vec![idx], None)
                });
            }

            // emit the write-back
            builder.emit_with(|id| {
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
                    if temp < builder.fun_env.get_local_count().unwrap_or_default() {
                        builder.emit_with(|id| {
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
            builder.emit_with(|id| Bytecode::Label(id, label));
        }
    }

    /// Instrument write-back action for spec blocks
    pub fn instrument_write_back_for_spec(
        builder: &mut FunctionDataBuilder,
        borrow_info: &BorrowInfo,
        nodes: BTreeSet<BorrowNode>,
    ) {
        for node in nodes {
            let mut ancestors = vec![];
            borrow_info.collect_ancestor_trees_recursive_reverse(&node, vec![], &mut ancestors);

            let is_conditional = ancestors.len() > 1;
            for (chain_index, chain) in ancestors.iter().enumerate() {
                // sanity check: the src node of the first action must be the node itself
                assert_eq!(
                    chain
                        .last()
                        .expect("The write-back chain should contain at action")
                        .dst,
                    node.clone()
                );

                Self::write_back_chain(builder, &ancestors, chain_index, is_conditional);
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

                Instrumenter::write_back_chain(
                    &mut self.builder,
                    &ancestors,
                    chain_index,
                    is_conditional,
                );
            }
        }
    }
}
