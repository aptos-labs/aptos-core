// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    experiments::Experiment,
    file_format_generator::{
        module_generator::{ModuleContext, ModuleGenerator, SOURCE_MAP_OK},
        peephole_optimizer, Options, MAX_FUNCTION_DEF_COUNT, MAX_LOCAL_COUNT,
    },
    pipeline::{
        flush_writes_processor::FlushWritesAnnotation,
        livevar_analysis_processor::LiveVarAnnotation,
    },
};
use move_binary_format::{
    file_format as FF,
    file_format::{CodeOffset, FunctionDefinitionIndex},
};
use move_core_types::{
    ability, function::ClosureMask, language_storage::PARAM_NAME_FOR_STRUCT_API,
};
use move_model::{
    ast::{ExpData, Spec, SpecBlockTarget, TempIndex},
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    model::{
        FunId, FunctionEnv, Loc, NodeId, Parameter, QualifiedId, StructEnv, StructId, TypeParameter,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
    well_known,
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    function_target_pipeline::FunctionVariant,
    stackless_bytecode::{AssignKind, AttrId, Bytecode, Constant, Label, Operation},
};
use std::collections::{BTreeMap, BTreeSet};

pub struct FunctionGenerator<'a> {
    /// The underlying module generator.
    genr: &'a mut ModuleGenerator,
    /// The set of temporaries which need to be pinned to locals because references are taken for
    /// them, or they are used in specs.
    pinned: BTreeSet<TempIndex>,
    /// A map from a temporary to information associated with it.
    temps: BTreeMap<TempIndex, TempInfo>,
    /// The value stack, represented by the temporaries which are located on it.
    /// Each temporary is paired with a bool, indicating whether the value was copied onto
    /// the stack from a local.
    /// If a value was copied onto the stack, then we don't have to save it back to the local.
    stack: Vec<(TempIndex, bool)>,
    /// The locals which have been used so far. This contains the parameters of the function.
    locals: Vec<Type>,
    /// A map from branching labels to information about them.
    label_info: BTreeMap<Label, LabelInfo>,
    /// A map from code offset to spec blocks associated with them
    spec_blocks: BTreeMap<CodeOffset, Spec>,
    /// The generated code
    code: Vec<FF::Bytecode>,
}

/// Immutable context for a function, separated from the mutable generator state, to reduce
/// borrow conflicts.
#[derive(Clone)]
pub struct FunctionContext<'env> {
    /// The module context
    pub module: ModuleContext<'env>,
    /// Function target we are generating code for.
    pub fun: FunctionTarget<'env>,
    /// Location of the function for error messages.
    pub loc: Loc,
    /// Type parameters, cached here.
    type_parameters: Vec<TypeParameter>,
    /// Function definition index.
    def_idx: FunctionDefinitionIndex,
}

/// Immutable context for processing a bytecode instruction.
#[derive(Clone)]
struct BytecodeContext<'env> {
    fun_ctx: &'env FunctionContext<'env>,
    code_offset: FF::CodeOffset,
    attr_id: AttrId,
}

#[derive(Debug, Copy, Clone)]
/// Represents the location of a temporary if it is not only on the stack.
struct TempInfo {
    /// The temp is stored in a local of given index.
    local: FF::LocalIndex,
}

impl TempInfo {
    fn new(local: FF::LocalIndex) -> Self {
        Self { local }
    }
}

/// Represents information about a label.
#[derive(Debug, Default)]
struct LabelInfo {
    /// The references to this label, as seen so far, in terms of the code offset of the
    /// instruction. The instruction pointed to will be any of `Branch`, `BrTrue`, or `BrFalse`.
    references: BTreeSet<FF::CodeOffset>,
    /// The resolution of linking the label to a code offset.
    resolution: Option<FF::CodeOffset>,
}

impl<'a> FunctionGenerator<'a> {
    /// Runs the function generator for the given function.
    pub fn run<'b>(
        genr: &'a mut ModuleGenerator,
        ctx: &'b ModuleContext,
        fun_env: FunctionEnv<'b>,
        acquires_list: &BTreeSet<StructId>,
    ) {
        let loc = fun_env.get_id_loc();
        let function = genr.function_index(ctx, &loc, &fun_env);
        let visibility = fun_env.visibility();
        let fun_count = genr.module.function_defs.len();
        let def_idx = FunctionDefinitionIndex::new(ctx.checked_bound(
            &loc,
            fun_count,
            MAX_FUNCTION_DEF_COUNT,
            "defined function",
        ));
        genr.source_map
            .add_top_level_function_mapping(def_idx, ctx.env.to_ir_loc(&loc), fun_env.is_native())
            .expect(SOURCE_MAP_OK);
        for TypeParameter(name, _, loc) in fun_env.get_type_parameters() {
            genr.source_map
                .add_function_type_parameter_mapping(def_idx, ctx.source_name(name, loc))
                .expect(SOURCE_MAP_OK)
        }
        for Parameter(name, _, loc) in fun_env.get_parameters() {
            genr.source_map
                .add_parameter_mapping(def_idx, ctx.source_name(name, loc))
                .expect(SOURCE_MAP_OK)
        }
        let (genr, code) = if !fun_env.is_native() {
            let mut fun_gen = Self {
                genr,
                pinned: Default::default(),
                temps: Default::default(),
                stack: vec![],
                locals: vec![],
                label_info: Default::default(),
                spec_blocks: BTreeMap::new(),
                code: vec![],
            };
            let target = ctx.targets.get_target(&fun_env, &FunctionVariant::Baseline);
            let mut code = fun_gen.gen_code(&FunctionContext {
                module: ctx.clone(),
                fun: target,
                loc: loc.clone(),
                type_parameters: fun_env.get_type_parameters(),
                def_idx,
            });
            let options = ctx
                .env
                .get_extension::<Options>()
                .expect("Options is available");
            if options.experiment_on(Experiment::PEEPHOLE_OPTIMIZATION) {
                let transformed_code_chunk = peephole_optimizer::optimize(&code.code);
                // Fix the source map for the optimized code.
                fun_gen
                    .genr
                    .source_map
                    .remap_code_map(def_idx, &transformed_code_chunk.original_offsets)
                    .expect(SOURCE_MAP_OK);
                // Replace the code with the optimized one.
                code.code = transformed_code_chunk.code;
                // Remap the spec blocks to the new code offsets.
                fun_gen.remap_spec_blocks(&transformed_code_chunk.original_offsets);
            }
            if !fun_gen.spec_blocks.is_empty() {
                fun_env.get_mut_spec().on_impl = fun_gen.spec_blocks;
            }
            (fun_gen.genr, Some(code))
        } else {
            (genr, None)
        };
        let acquires_global_resources = acquires_list
            .iter()
            .map(|id| {
                let struct_env = fun_env.module_env.get_struct(*id);
                genr.struct_def_index(ctx, &struct_env.get_loc(), &struct_env)
            })
            .collect();
        let def = FF::FunctionDefinition {
            function,
            visibility,
            is_entry: fun_env.is_entry(),
            acquires_global_resources,
            code,
        };

        genr.module.function_defs.push(def)
    }

    /// Common builder for generating function definitions for struct APIs
    fn gen_struct_api_common<'b, F>(
        genr: &'a mut ModuleGenerator,
        ctx: &'b ModuleContext,
        struct_env: &'b StructEnv<'b>,
        visibility: FF::Visibility,
        base_loc: &'b Loc,
        type_parameters: &[Type],
        function_indices: BTreeMap<Option<Symbol>, FF::FunctionHandleIndex>,
        mut mk_body: F,
    ) where
        F: FnMut(
            &mut ModuleGenerator,
            &ModuleContext,
            &StructEnv<'_>,
            Option<Symbol>,
            FunctionDefinitionIndex,
            &Loc,
            &[Type],
        ) -> FF::CodeUnit,
    {
        assert!(visibility.is_public_or_friend());

        for (variant, function_index) in function_indices {
            let def_idx = FunctionDefinitionIndex::new(ctx.checked_bound(
                base_loc,
                genr.module.function_defs.len(),
                MAX_FUNCTION_DEF_COUNT,
                "defined function",
            ));

            let loc = if let Some(variant_id) = variant {
                struct_env.get_variant_loc(variant_id)
            } else {
                base_loc
            };

            genr.source_map
                .add_top_level_function_mapping(def_idx, ctx.env.to_ir_loc(loc), false)
                .expect(SOURCE_MAP_OK);

            for TypeParameter(name, _, tp_loc) in struct_env.get_type_parameters() {
                genr.source_map
                    .add_function_type_parameter_mapping(
                        def_idx,
                        ctx.source_name(name, tp_loc.clone()),
                    )
                    .expect(SOURCE_MAP_OK);
            }

            // Let the caller build the actual body.
            let code = mk_body(
                genr,
                ctx,
                struct_env,
                variant,
                def_idx,
                loc,
                type_parameters,
            );

            let def = FF::FunctionDefinition {
                function: function_index,
                visibility,
                is_entry: false,
                acquires_global_resources: vec![],
                code: Some(code),
            };

            genr.module.function_defs.push(def);
        }
    }

    /// Generates test variant APIs for all variants of an enum
    pub fn gen_struct_test_variant_api<'b>(
        genr: &'a mut ModuleGenerator,
        ctx: &'b ModuleContext,
        struct_env: &'b StructEnv<'b>,
    ) {
        if !struct_env.has_variants() {
            return;
        }

        let visibility = struct_env.get_visibility();
        assert!(visibility.is_public_or_friend());

        let type_parameters = TypeParameter::vec_to_formals(struct_env.get_type_parameters());
        let struct_ty = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            type_parameters.clone(),
        );
        let ref_struct_ty = Type::Reference(ReferenceKind::Immutable, Box::new(struct_ty));
        let loc = struct_env.get_loc();

        let raw_function_indices = genr.struct_api_test_variant_index(ctx, &loc, struct_env);
        let function_indices = raw_function_indices
            .into_iter()
            .map(|(variant, fh_idx)| (Some(variant), fh_idx))
            .collect();

        Self::gen_struct_api_common(
            genr,
            ctx,
            struct_env,
            visibility,
            &loc,
            &type_parameters,
            function_indices,
            |genr, ctx, struct_env, variant_opt, def_idx, variant_loc, type_parameters| {
                let variant = variant_opt.expect("test_variant API only generated for variants");

                let input_param_name = struct_env
                    .env()
                    .symbol_pool()
                    .make(PARAM_NAME_FOR_STRUCT_API);
                genr.source_map
                    .add_parameter_mapping(
                        def_idx,
                        ctx.source_name(input_param_name, variant_loc.clone()),
                    )
                    .expect(SOURCE_MAP_OK);

                // Function body:
                // MoveLoc(0)
                // TestVariant/TestVariantGeneric
                // Ret
                let locals = genr.signature(ctx, &loc, vec![ref_struct_ty.clone()]);

                let mut code = vec![FF::Bytecode::MoveLoc(0 as FF::LocalIndex)];

                let test_variant_bc = if type_parameters.is_empty() {
                    let idx = genr.struct_variant_index(ctx, &loc, struct_env, variant);
                    FF::Bytecode::TestVariant(idx)
                } else {
                    let idx = genr.struct_variant_inst_index(
                        ctx,
                        &loc,
                        struct_env,
                        variant,
                        type_parameters.to_vec(),
                    );
                    FF::Bytecode::TestVariantGeneric(idx)
                };

                code.push(test_variant_bc);
                code.push(FF::Bytecode::Ret);

                FF::CodeUnit { locals, code }
            },
        );
    }

    /// Generates pack APIs
    pub fn gen_struct_pack_api<'b>(
        genr: &'a mut ModuleGenerator,
        ctx: &'b ModuleContext,
        struct_env: &'b StructEnv<'b>,
    ) {
        let visibility = struct_env.get_visibility();
        let type_parameters = TypeParameter::vec_to_formals(struct_env.get_type_parameters());
        let loc = struct_env.get_loc();
        let function_indices = genr.struct_api_pack_index(ctx, &loc, struct_env);

        Self::gen_struct_api_common(
            genr,
            ctx,
            struct_env,
            visibility,
            &loc,
            type_parameters.as_slice(),
            function_indices,
            |genr, ctx, struct_env, variant, def_idx, loc, type_parameters| {
                // Function body:
                // MoveLoc(0)
                // MoveLoc(1)
                // ...
                // MoveLoc(parameter_count - 1)
                // Pack/PackGeneric/PackVariant/PackVariantGeneric
                // Ret
                let mut tys = vec![];
                let mut code = vec![];

                for (i, field) in struct_env.get_fields_optional_variant(variant).enumerate() {
                    let field_symbol = struct_env.env().symbol_pool().make(&format!(
                        "_{}",
                        field.get_name().display(struct_env.env().symbol_pool())
                    ));
                    genr.source_map
                        .add_parameter_mapping(def_idx, ctx.source_name(field_symbol, loc.clone()))
                        .expect(SOURCE_MAP_OK);

                    // number of fields will not exceed 255, so we can safely cast it
                    code.push(FF::Bytecode::MoveLoc(i as FF::LocalIndex));
                    tys.push(field.get_type());
                }

                let locals = genr.signature(ctx, loc, tys);

                let pack_bc = match (type_parameters.is_empty(), variant) {
                    (true, Some(variant_id)) => {
                        let idx = genr.struct_variant_index(ctx, loc, struct_env, variant_id);
                        FF::Bytecode::PackVariant(idx)
                    },
                    (true, None) => {
                        let idx = genr.struct_def_index(ctx, loc, struct_env);
                        FF::Bytecode::Pack(idx)
                    },
                    (false, Some(variant_id)) => {
                        let idx = genr.struct_variant_inst_index(
                            ctx,
                            loc,
                            struct_env,
                            variant_id,
                            type_parameters.to_vec(),
                        );
                        FF::Bytecode::PackVariantGeneric(idx)
                    },
                    (false, None) => {
                        let idx = genr.struct_def_instantiation_index(
                            ctx,
                            loc,
                            struct_env,
                            type_parameters.to_vec(),
                        );
                        FF::Bytecode::PackGeneric(idx)
                    },
                };

                code.push(pack_bc);
                code.push(FF::Bytecode::Ret);

                FF::CodeUnit { locals, code }
            },
        );
    }

    /// Generates unpack api
    pub fn gen_struct_unpack_api<'b>(
        genr: &'a mut ModuleGenerator,
        ctx: &'b ModuleContext,
        struct_env: &'b StructEnv<'b>,
    ) {
        let visibility = struct_env.get_visibility();
        let type_parameters = TypeParameter::vec_to_formals(struct_env.get_type_parameters());
        let loc = struct_env.get_loc();
        let function_indices = genr.struct_api_unpack_index(ctx, &loc, struct_env);

        let struct_ty = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            type_parameters.clone(),
        );

        Self::gen_struct_api_common(
            genr,
            ctx,
            struct_env,
            visibility,
            &loc,
            &type_parameters,
            function_indices,
            |genr, ctx, struct_env, variant, def_idx, loc, type_parameters| {
                let input_param_name = struct_env
                    .env()
                    .symbol_pool()
                    .make(PARAM_NAME_FOR_STRUCT_API);

                genr.source_map
                    .add_parameter_mapping(def_idx, ctx.source_name(input_param_name, loc.clone()))
                    .expect(SOURCE_MAP_OK);

                // Function body:
                // MoveLoc(0)
                // Unpack/UnpackGeneric/UnpackVariant/UnpackVariantGeneric
                // Ret
                let locals = genr.signature(ctx, loc, vec![struct_ty.clone()]);
                let mut code = vec![FF::Bytecode::MoveLoc(0 as FF::LocalIndex)];

                let unpack_bc = match (type_parameters.is_empty(), variant) {
                    (true, Some(variant_id)) => {
                        let idx = genr.struct_variant_index(ctx, loc, struct_env, variant_id);
                        FF::Bytecode::UnpackVariant(idx)
                    },
                    (true, None) => {
                        let idx = genr.struct_def_index(ctx, loc, struct_env);
                        FF::Bytecode::Unpack(idx)
                    },
                    (false, Some(variant_id)) => {
                        let idx = genr.struct_variant_inst_index(
                            ctx,
                            loc,
                            struct_env,
                            variant_id,
                            type_parameters.to_vec(),
                        );
                        FF::Bytecode::UnpackVariantGeneric(idx)
                    },
                    (false, None) => {
                        let idx = genr.struct_def_instantiation_index(
                            ctx,
                            loc,
                            struct_env,
                            type_parameters.to_vec(),
                        );
                        FF::Bytecode::UnpackGeneric(idx)
                    },
                };

                code.push(unpack_bc);
                code.push(FF::Bytecode::Ret);

                FF::CodeUnit { locals, code }
            },
        );
    }

    /// Generates borrow field api
    pub fn gen_struct_borrow_field_api<'b>(
        genr: &'a mut ModuleGenerator,
        ctx: &'b ModuleContext,
        struct_env: &'b StructEnv<'b>,
        is_imm: bool,
    ) {
        let visibility = struct_env.get_visibility();
        assert!(visibility.is_public_or_friend());
        // Early return if struct is empty (has no fields)
        if struct_env.is_empty_struct() {
            return;
        }
        let type_parameters = TypeParameter::vec_to_formals(struct_env.get_type_parameters());
        let loc = struct_env.get_loc();
        let function_indices = genr.struct_api_borrow_index(ctx, &loc, struct_env, is_imm);
        let struct_ty = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            type_parameters.clone(),
        );
        let ref_struct_ty = Type::Reference(
            if is_imm {
                ReferenceKind::Immutable
            } else {
                ReferenceKind::Mutable
            },
            Box::new(struct_ty),
        );
        for ((variant_opt, offset, _), function_index) in function_indices {
            let def_idx = FunctionDefinitionIndex::new(ctx.checked_bound(
                &loc,
                genr.module.function_defs.len(),
                MAX_FUNCTION_DEF_COUNT,
                "defined function",
            ));
            genr.source_map
                .add_top_level_function_mapping(def_idx, ctx.env.to_ir_loc(&loc), false)
                .expect(SOURCE_MAP_OK);
            for TypeParameter(name, _, loc) in struct_env.get_type_parameters() {
                genr.source_map
                    .add_function_type_parameter_mapping(
                        def_idx,
                        ctx.source_name(name, loc.clone()),
                    )
                    .expect(SOURCE_MAP_OK)
            }
            let input_param_name = struct_env
                .env()
                .symbol_pool()
                .make(PARAM_NAME_FOR_STRUCT_API);
            genr.source_map
                .add_parameter_mapping(def_idx, ctx.source_name(input_param_name, loc.clone()))
                .expect(SOURCE_MAP_OK);

            // Function body:
            // MoveLoc(0)
            // ImmBorrowField/MutBorrowField/ImmBorrowFieldGeneric/MutBorrowFieldGeneric/
            // ImmBorrowVariantField/MutBorrowVariantField/ImmBorrowVariantFieldGeneric/MutBorrowVariantFieldGeneric
            // Ret
            let locals = genr.signature(ctx, &loc, vec![ref_struct_ty.clone()]);
            let mut code = vec![];
            code.push(FF::Bytecode::MoveLoc(0 as FF::LocalIndex));
            let field_env = &struct_env.get_field_by_offset_optional_variant(
                variant_opt.clone().map(|variants| variants[0]),
                offset,
            );
            let borrow_field_bc = match (type_parameters.is_empty(), variant_opt, is_imm) {
                (true, Some(variants), true) => {
                    let idx = genr.variant_field_index(ctx, &loc, &variants, field_env);
                    FF::Bytecode::ImmBorrowVariantField(idx)
                },
                (true, Some(variants), false) => {
                    let idx = genr.variant_field_index(ctx, &loc, &variants, field_env);
                    FF::Bytecode::MutBorrowVariantField(idx)
                },
                (true, None, true) => {
                    let idx = genr.field_index(ctx, &loc, field_env);
                    FF::Bytecode::ImmBorrowField(idx)
                },
                (true, None, false) => {
                    let idx = genr.field_index(ctx, &loc, field_env);
                    FF::Bytecode::MutBorrowField(idx)
                },
                (false, Some(variants), true) => {
                    let idx = genr.variant_field_inst_index(
                        ctx,
                        &loc,
                        &variants,
                        field_env,
                        type_parameters.to_vec(),
                    );
                    FF::Bytecode::ImmBorrowVariantFieldGeneric(idx)
                },
                (false, Some(variants), false) => {
                    let idx = genr.variant_field_inst_index(
                        ctx,
                        &loc,
                        &variants,
                        field_env,
                        type_parameters.to_vec(),
                    );
                    FF::Bytecode::MutBorrowVariantFieldGeneric(idx)
                },
                (false, None, true) => {
                    let idx = genr.field_inst_index(ctx, &loc, field_env, type_parameters.to_vec());
                    FF::Bytecode::ImmBorrowFieldGeneric(idx)
                },
                (false, None, false) => {
                    let idx = genr.field_inst_index(ctx, &loc, field_env, type_parameters.to_vec());
                    FF::Bytecode::MutBorrowFieldGeneric(idx)
                },
            };
            code.push(borrow_field_bc);
            code.push(FF::Bytecode::Ret);
            let code = FF::CodeUnit { locals, code };

            let def = FF::FunctionDefinition {
                function: function_index,
                visibility,
                is_entry: false,
                acquires_global_resources: vec![],
                code: Some(code),
            };

            genr.module.function_defs.push(def);
        }
    }

    /// Generates code for a function.
    fn gen_code(&mut self, ctx: &FunctionContext<'_>) -> FF::CodeUnit {
        // Initialize the abstract virtual machine
        // TODO: right now we pin temps which are parameter of the drop instruction.
        //   This is needed since we cannot determine whether the local has been already moved on
        //   the stack and is not longer available in the associated local. This needs to be reworked
        //   to avoid this.
        self.pinned = ctx.fun.get_pinned_temps(/*include_drop*/ true);
        self.temps = (0..ctx.fun.get_parameter_count())
            .map(|temp| (temp, TempInfo::new(self.temp_to_local(ctx, None, temp))))
            .collect();
        self.locals = (0..ctx.fun.get_parameter_count())
            .map(|temp| ctx.temp_type(temp).to_owned())
            .collect();

        // Walk the bytecode
        let bytecode = ctx.fun.get_bytecode();
        for i in 0..bytecode.len() {
            let code_offset = i as FF::CodeOffset;
            let bc = &bytecode[i];
            let bytecode_ctx = BytecodeContext {
                fun_ctx: ctx,
                code_offset,
                attr_id: bc.get_attr_id(),
            };
            if i + 1 < bytecode.len() {
                let next_bc = &bytecode[i + 1];
                self.gen_bytecode(&bytecode_ctx, bc, Some(next_bc));
                if !bc.is_branching() && matches!(next_bc, Bytecode::Label(..)) {
                    // At block boundaries without a preceding branch, need to flush stack
                    // TODO: to avoid this, we should use the CFG for code generation.
                    self.abstract_flush_stack_after(&bytecode_ctx, 0);
                }
            } else {
                self.gen_bytecode(&bytecode_ctx, bc, None)
            }
        }

        // At this point, all labels should be resolved, so link them.
        for info in self.label_info.values() {
            if let Some(label_offs) = info.resolution {
                for ref_offs in &info.references {
                    let ref_offs = *ref_offs;
                    let code_ref = &mut self.code[ref_offs as usize];
                    match code_ref {
                        FF::Bytecode::Branch(_) => *code_ref = FF::Bytecode::Branch(label_offs),
                        FF::Bytecode::BrTrue(_) => *code_ref = FF::Bytecode::BrTrue(label_offs),
                        FF::Bytecode::BrFalse(_) => *code_ref = FF::Bytecode::BrFalse(label_offs),
                        _ => {},
                    }
                }
            } else {
                ctx.internal_error("inconsistent bytecode label info")
            }
        }

        // Deliver result
        let locals = self.genr.signature(
            &ctx.module,
            &ctx.loc,
            self.locals[ctx.fun.get_parameter_count()..].to_vec(),
        );
        FF::CodeUnit {
            locals,
            code: std::mem::take(&mut self.code),
        }
    }

    /// Generate file-format bytecode from a stackless bytecode and an optional next bytecode
    /// for peephole optimizations.
    fn gen_bytecode(&mut self, ctx: &BytecodeContext, bc: &Bytecode, next_bc: Option<&Bytecode>) {
        self.genr
            .source_map
            .add_code_mapping(
                ctx.fun_ctx.def_idx,
                self.code.len() as FF::CodeOffset,
                ctx.fun_ctx
                    .module
                    .env
                    .to_ir_loc(&ctx.fun_ctx.fun.get_bytecode_loc(ctx.attr_id)),
            )
            .expect(SOURCE_MAP_OK);
        match bc {
            Bytecode::Assign(_, dest, source, mode) => {
                self.flush_any_conflicts(
                    ctx,
                    std::slice::from_ref(dest),
                    std::slice::from_ref(source),
                );
                self.abstract_push_args(ctx, vec![*source], Some(mode));
                self.abstract_pop(ctx);
                self.abstract_push_result(ctx, std::slice::from_ref(dest));
            },
            Bytecode::Ret(_, result) => {
                self.balance_stack_end_of_block(ctx, result);
                self.emit(FF::Bytecode::Ret);
                self.abstract_pop_n(ctx, result.len());
            },
            Bytecode::Call(_, dest, oper, source, None) => {
                self.gen_operation(ctx, dest, oper, source)
            },
            Bytecode::Load(_, dest, cons) => self.gen_load(ctx, dest, cons),
            Bytecode::Label(_, label) => self.define_label(*label),
            Bytecode::Branch(_, if_true, if_false, cond) => {
                // Ensure only `cond` is on the stack before branch.
                self.balance_stack_end_of_block(ctx, vec![*cond]);
                // Attempt to detect fallthrough, such that for
                // ```
                //   branch l1, l2, cond
                //   l1: ...
                // ```
                // .. we generate adequate code.
                let successor_label_opt = next_bc.and_then(|bc| {
                    if let Bytecode::Label(_, l) = bc {
                        Some(*l)
                    } else {
                        None
                    }
                });
                if successor_label_opt == Some(*if_true) {
                    self.add_label_reference(*if_false);
                    self.emit(FF::Bytecode::BrFalse(0))
                } else if successor_label_opt == Some(*if_false) {
                    self.add_label_reference(*if_true);
                    self.emit(FF::Bytecode::BrTrue(0))
                } else {
                    // No fallthrough
                    self.add_label_reference(*if_false);
                    self.emit(FF::Bytecode::BrFalse(0));
                    self.add_label_reference(*if_true);
                    self.emit(FF::Bytecode::Branch(0))
                }
                self.abstract_pop(ctx);
            },
            Bytecode::Jump(_, label) => {
                self.abstract_flush_stack_before(ctx, 0);
                self.add_label_reference(*label);
                self.emit(FF::Bytecode::Branch(0));
            },
            Bytecode::Abort(_, temp, None) => {
                self.balance_stack_end_of_block(ctx, [*temp]);
                self.emit(FF::Bytecode::Abort);
                self.abstract_pop(ctx)
            },
            Bytecode::Abort(_, temp0, Some(temp1)) => {
                self.balance_stack_end_of_block(ctx, [*temp0, *temp1]);
                self.emit(FF::Bytecode::AbortMsg);
                self.abstract_pop_n(ctx, 2);
            },
            Bytecode::Nop(_) => {
                // do nothing -- labels are relative
            },
            Bytecode::SpecBlock(_, spec) => self.gen_spec_block(ctx, spec),
            Bytecode::SaveMem(_, _, _)
            | Bytecode::Call(_, _, _, _, Some(_))
            | Bytecode::SaveSpecVar(_, _, _)
            | Bytecode::Prop(_, _, _) => {
                // do nothing -- skip specification ops
            },
        }
    }

    /// Balance the stack such that it exactly contains the `result` temps and nothing else. This
    /// is used for instructions like `branch`, `return` or `abort` which terminate a block
    /// and must leave the stack empty at end.
    fn balance_stack_end_of_block(
        &mut self,
        ctx: &BytecodeContext,
        result: impl AsRef<[TempIndex]>,
    ) {
        let result = result.as_ref();
        let stack = self.stack.iter().map(|(t, _)| *t).collect::<Vec<_>>();
        // If the stack already contains exactly the result and none of the temps are used after
        // or were copied from locals, nothing to do.
        // [TODO]: we could further optimize this when:
        //   1. `stack == result` and
        //   2. some stack temps are alive after and were not copied from locals;
        // then we can flush only until the lowest such temp on the stack, and then push them back,
        // instead of always fully flushing the stack.
        let stack_ready = stack == result
            && self
                .stack
                .iter()
                .all(|(temp, copied)| *copied || !ctx.is_alive_after(*temp, &[], false));
        if !stack_ready {
            // Flush the stack and push the result.
            self.abstract_flush_stack_before(ctx, 0);
            self.abstract_push_args(ctx, result, None);
        }
    }

    /// Adds a reference to a label to the LabelInfo. This is used to link the labels final
    /// value at the current code offset once it is resolved.
    fn add_label_reference(&mut self, label: Label) {
        let offset = self.code.len() as FF::CodeOffset;
        self.label_info
            .entry(label)
            .or_default()
            .references
            .insert(offset);
    }

    /// Sets the resolution of a label to the current code offset.
    fn define_label(&mut self, label: Label) {
        let offset = self.code.len() as FF::CodeOffset;
        self.label_info.entry(label).or_default().resolution = Some(offset)
    }

    /// Generates code for an operation.
    fn gen_operation(
        &mut self,
        ctx: &BytecodeContext,
        dest: &[TempIndex],
        oper: &Operation,
        source: &[TempIndex],
    ) {
        self.flush_any_conflicts(ctx, dest, source);
        let fun_ctx = ctx.fun_ctx;
        match oper {
            Operation::Function(mid, fid, inst) => {
                self.gen_call(ctx, dest, mid.qualified(*fid), inst, source);
            },
            Operation::Closure(mid, fid, inst, mask) => {
                self.gen_closure(ctx, dest, mid.qualified(*fid), inst, *mask, source);
            },
            Operation::Invoke => {
                self.gen_invoke(ctx, dest, source);
            },
            Operation::Pack(mid, sid, inst) => {
                let struct_env = &fun_ctx.module.env.get_struct(mid.qualified(*sid));
                let fun_mid = ctx.fun_ctx.fun.func_env.module_env.get_id();
                if mid != &fun_mid {
                    if !struct_env
                        .env()
                        .language_version()
                        .language_version_for_public_struct()
                    {
                        fun_ctx.internal_error(format!(
                            "cross module struct access is not supported by language version {}",
                            struct_env.env().language_version()
                        ));
                        return;
                    }
                    self.gen_pack_unpack_api_call::<true>(
                        ctx, dest, source, fun_ctx, &None, inst, struct_env,
                    );
                } else {
                    self.gen_struct_oper(
                        ctx,
                        dest,
                        mid.qualified(*sid),
                        inst,
                        source,
                        FF::Bytecode::Pack,
                        FF::Bytecode::PackGeneric,
                    );
                }
            },
            Operation::PackVariant(mid, sid, variant, inst) => {
                let struct_env = &fun_ctx.module.env.get_struct(mid.qualified(*sid));
                let fun_mid = ctx.fun_ctx.fun.func_env.module_env.get_id();
                if mid != &fun_mid {
                    if !struct_env
                        .env()
                        .language_version()
                        .language_version_for_public_struct()
                    {
                        fun_ctx.internal_error(format!(
                            "cross module struct access is not supported by language version {}",
                            struct_env.env().language_version()
                        ));
                        return;
                    }
                    self.gen_pack_unpack_api_call::<true>(
                        ctx,
                        dest,
                        source,
                        fun_ctx,
                        &Some(*variant),
                        inst,
                        struct_env,
                    );
                } else {
                    self.gen_struct_variant_oper(
                        ctx,
                        dest,
                        mid.qualified(*sid),
                        *variant,
                        inst,
                        source,
                        FF::Bytecode::PackVariant,
                        FF::Bytecode::PackVariantGeneric,
                    );
                }
            },
            Operation::Unpack(mid, sid, inst) => {
                let struct_env = &fun_ctx.module.env.get_struct(mid.qualified(*sid));
                let fun_mid = ctx.fun_ctx.fun.func_env.module_env.get_id();
                if mid != &fun_mid {
                    if !struct_env
                        .env()
                        .language_version()
                        .language_version_for_public_struct()
                    {
                        fun_ctx.internal_error(format!(
                            "cross module struct access is not supported by language version {}",
                            struct_env.env().language_version()
                        ));
                        return;
                    }
                    self.gen_pack_unpack_api_call::<false>(
                        ctx, dest, source, fun_ctx, &None, inst, struct_env,
                    );
                } else {
                    self.gen_struct_oper(
                        ctx,
                        dest,
                        mid.qualified(*sid),
                        inst,
                        source,
                        FF::Bytecode::Unpack,
                        FF::Bytecode::UnpackGeneric,
                    );
                }
            },
            Operation::UnpackVariant(mid, sid, variant, inst) => {
                let struct_env = &fun_ctx.module.env.get_struct(mid.qualified(*sid));
                let fun_mid = ctx.fun_ctx.fun.func_env.module_env.get_id();
                if mid != &fun_mid {
                    if !struct_env
                        .env()
                        .language_version()
                        .language_version_for_public_struct()
                    {
                        fun_ctx.internal_error(format!(
                            "cross module struct access is not supported by language version {}",
                            struct_env.env().language_version()
                        ));
                        return;
                    }
                    self.gen_pack_unpack_api_call::<false>(
                        ctx,
                        dest,
                        source,
                        fun_ctx,
                        &Some(*variant),
                        inst,
                        struct_env,
                    );
                } else {
                    self.gen_struct_variant_oper(
                        ctx,
                        dest,
                        mid.qualified(*sid),
                        *variant,
                        inst,
                        source,
                        FF::Bytecode::UnpackVariant,
                        FF::Bytecode::UnpackVariantGeneric,
                    );
                }
            },
            Operation::TestVariant(mid, sid, variant, inst) => {
                let struct_env = &fun_ctx.module.env.get_struct(mid.qualified(*sid));
                let fun_mid = ctx.fun_ctx.fun.func_env.module_env.get_id();
                let is_mut_src = fun_ctx.fun.get_local_type(source[0]).is_mutable_reference();
                if mid != &fun_mid {
                    if !struct_env
                        .env()
                        .language_version()
                        .language_version_for_public_struct()
                    {
                        fun_ctx.internal_error(format!(
                            "cross module struct access is not supported by language version {}",
                            struct_env.env().language_version()
                        ));
                        return;
                    }
                    self.gen_test_variant_api_call(
                        ctx, dest, source, fun_ctx, variant, inst, struct_env, is_mut_src,
                    );
                } else {
                    self.gen_struct_variant_oper(
                        ctx,
                        dest,
                        mid.qualified(*sid),
                        *variant,
                        inst,
                        source,
                        FF::Bytecode::TestVariant,
                        FF::Bytecode::TestVariantGeneric,
                    );
                }
            },
            Operation::MoveTo(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::MoveTo,
                    FF::Bytecode::MoveToGeneric,
                );
            },
            Operation::MoveFrom(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::MoveFrom,
                    FF::Bytecode::MoveFromGeneric,
                );
            },
            Operation::Exists(mid, sid, inst) => {
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    FF::Bytecode::Exists,
                    FF::Bytecode::ExistsGeneric,
                );
            },
            Operation::BorrowLoc => {
                let local = self.temp_to_local(fun_ctx, Some(ctx.attr_id), source[0]);
                if fun_ctx.fun.get_local_type(dest[0]).is_mutable_reference() {
                    self.emit(FF::Bytecode::MutBorrowLoc(local))
                } else {
                    self.emit(FF::Bytecode::ImmBorrowLoc(local))
                }
                self.abstract_push_result(ctx, dest)
            },
            Operation::BorrowField(mid, sid, inst, offset) => {
                let struct_env = &fun_ctx.module.env.get_struct(mid.qualified(*sid));
                let fun_mid = ctx.fun_ctx.fun.func_env.module_env.get_id();
                if mid != &fun_mid {
                    if !struct_env
                        .env()
                        .language_version()
                        .language_version_for_public_struct()
                    {
                        fun_ctx.internal_error(format!(
                            "cross module struct access is not supported by language version {}",
                            struct_env.env().language_version()
                        ));
                        return;
                    }
                    self.gen_borrow_field_api_call(
                        ctx, dest, source, fun_ctx, &None, inst, offset, struct_env,
                    );
                } else {
                    self.gen_borrow_field(
                        ctx,
                        dest,
                        mid.qualified(*sid),
                        inst.clone(),
                        None,
                        *offset,
                        source,
                    );
                }
            },
            Operation::BorrowVariantField(mid, sid, variants, inst, offset) => {
                let struct_env = &fun_ctx.module.env.get_struct(mid.qualified(*sid));
                let fun_mid = ctx.fun_ctx.fun.func_env.module_env.get_id();
                if mid != &fun_mid {
                    if !struct_env
                        .env()
                        .language_version()
                        .language_version_for_public_struct()
                    {
                        fun_ctx.internal_error(format!(
                            "cross module struct access is not supported by language version {}",
                            struct_env.env().language_version()
                        ));
                        return;
                    }
                    self.gen_borrow_field_api_call(
                        ctx,
                        dest,
                        source,
                        fun_ctx,
                        &Some(variants.clone()),
                        inst,
                        offset,
                        struct_env,
                    );
                } else {
                    self.gen_borrow_field(
                        ctx,
                        dest,
                        mid.qualified(*sid),
                        inst.clone(),
                        Some(variants),
                        *offset,
                        source,
                    );
                }
            },
            Operation::BorrowGlobal(mid, sid, inst) => {
                let is_mut = fun_ctx.fun.get_local_type(dest[0]).is_mutable_reference();
                self.gen_struct_oper(
                    ctx,
                    dest,
                    mid.qualified(*sid),
                    inst,
                    source,
                    if is_mut {
                        FF::Bytecode::MutBorrowGlobal
                    } else {
                        FF::Bytecode::ImmBorrowGlobal
                    },
                    if is_mut {
                        FF::Bytecode::MutBorrowGlobalGeneric
                    } else {
                        FF::Bytecode::ImmBorrowGlobalGeneric
                    },
                )
            },
            Operation::Vector => {
                let elem_type = if let Type::Vector(el) = fun_ctx.fun.get_local_type(dest[0]) {
                    el.as_ref().clone()
                } else {
                    fun_ctx.internal_error("expected vector type");
                    Type::new_prim(PrimitiveType::Bool)
                };
                let sign = self
                    .genr
                    .signature(&fun_ctx.module, &fun_ctx.loc, vec![elem_type]);
                self.gen_builtin(
                    ctx,
                    dest,
                    FF::Bytecode::VecPack(sign, source.len() as u64),
                    source,
                )
            },
            Operation::ReadRef => self.gen_builtin(ctx, dest, FF::Bytecode::ReadRef, source),
            Operation::WriteRef => {
                self.gen_builtin(ctx, dest, FF::Bytecode::WriteRef, &[source[1], source[0]])
            },
            Operation::Release => {
                // Move bytecode does not process release, values are released indirectly
                // when the borrowed head of the borrow chain is destroyed
            },
            Operation::Drop => {
                // Currently Destroy is only translated for references. It may also make
                // sense for other values, as we may figure later. Its known to be required
                // for references to make the bytecode verifier happy.
                let ty = ctx.fun_ctx.fun.get_local_type(source[0]);
                if ty.is_reference() {
                    self.gen_builtin(ctx, dest, FF::Bytecode::Pop, source)
                }
            },
            Operation::FreezeRef(_) => self.gen_builtin(ctx, dest, FF::Bytecode::FreezeRef, source),
            Operation::CastU8 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU8, source),
            Operation::CastU16 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU16, source),
            Operation::CastU32 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU32, source),
            Operation::CastU64 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU64, source),
            Operation::CastU128 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU128, source),
            Operation::CastU256 => self.gen_builtin(ctx, dest, FF::Bytecode::CastU256, source),
            Operation::CastI8 => self.gen_builtin(ctx, dest, FF::Bytecode::CastI8, source),
            Operation::CastI16 => self.gen_builtin(ctx, dest, FF::Bytecode::CastI16, source),
            Operation::CastI32 => self.gen_builtin(ctx, dest, FF::Bytecode::CastI32, source),
            Operation::CastI64 => self.gen_builtin(ctx, dest, FF::Bytecode::CastI64, source),
            Operation::CastI128 => self.gen_builtin(ctx, dest, FF::Bytecode::CastI128, source),
            Operation::CastI256 => self.gen_builtin(ctx, dest, FF::Bytecode::CastI256, source),
            Operation::Not => self.gen_builtin(ctx, dest, FF::Bytecode::Not, source),
            Operation::Add => self.gen_builtin(ctx, dest, FF::Bytecode::Add, source),
            Operation::Sub => self.gen_builtin(ctx, dest, FF::Bytecode::Sub, source),
            Operation::Mul => self.gen_builtin(ctx, dest, FF::Bytecode::Mul, source),
            Operation::Div => self.gen_builtin(ctx, dest, FF::Bytecode::Div, source),
            Operation::Mod => self.gen_builtin(ctx, dest, FF::Bytecode::Mod, source),
            Operation::BitOr => self.gen_builtin(ctx, dest, FF::Bytecode::BitOr, source),
            Operation::BitAnd => self.gen_builtin(ctx, dest, FF::Bytecode::BitAnd, source),
            Operation::Xor => self.gen_builtin(ctx, dest, FF::Bytecode::Xor, source),
            Operation::Shl => self.gen_builtin(ctx, dest, FF::Bytecode::Shl, source),
            Operation::Shr => self.gen_builtin(ctx, dest, FF::Bytecode::Shr, source),
            Operation::Lt => self.gen_builtin(ctx, dest, FF::Bytecode::Lt, source),
            Operation::Gt => self.gen_builtin(ctx, dest, FF::Bytecode::Gt, source),
            Operation::Le => self.gen_builtin(ctx, dest, FF::Bytecode::Le, source),
            Operation::Ge => self.gen_builtin(ctx, dest, FF::Bytecode::Ge, source),
            Operation::Or => self.gen_builtin(ctx, dest, FF::Bytecode::Or, source),
            Operation::And => self.gen_builtin(ctx, dest, FF::Bytecode::And, source),
            Operation::Eq => self.gen_builtin(ctx, dest, FF::Bytecode::Eq, source),
            Operation::Neq => self.gen_builtin(ctx, dest, FF::Bytecode::Neq, source),
            Operation::Negate => self.gen_builtin(ctx, dest, FF::Bytecode::Negate, source),

            Operation::TraceLocal(_)
            | Operation::TraceReturn(_)
            | Operation::TraceAbort
            | Operation::TraceExp(_, _)
            | Operation::TraceGlobalMem(_)
            | Operation::EmitEvent
            | Operation::EventStoreDiverge
            | Operation::OpaqueCallBegin(_, _, _)
            | Operation::OpaqueCallEnd(_, _, _)
            | Operation::GetField(_, _, _, _)
            | Operation::GetVariantField(_, _, _, _, _)
            | Operation::GetGlobal(_, _, _)
            | Operation::Uninit
            | Operation::Havoc(_)
            | Operation::Stop
            | Operation::IsParent(_, _)
            | Operation::WriteBack(_, _)
            | Operation::UnpackRef
            | Operation::PackRef
            | Operation::UnpackRefDeep
            | Operation::PackRefDeep => fun_ctx.internal_error("unexpected specification opcode"),
        }
    }

    /// Generate code for calling the borrow field api.
    fn gen_borrow_field_api_call(
        &mut self,
        ctx: &BytecodeContext<'_>,
        dest: &[usize],
        source: &[usize],
        fun_ctx: &FunctionContext<'_>,
        variants: &Option<Vec<Symbol>>,
        inst: &[Type],
        offset: &usize,
        struct_env: &StructEnv<'_>,
    ) {
        let is_mut_dest = fun_ctx.fun.get_local_type(dest[0]).is_mutable_reference();
        let is_mut_src = fun_ctx.fun.get_local_type(source[0]).is_mutable_reference();
        let vec = self.genr.struct_api_borrow_index(
            &ctx.fun_ctx.module,
            &fun_ctx.loc,
            struct_env,
            !is_mut_dest,
        );
        // generate code for calling borrow field API
        // for struct, we can directly call the API when the offset matches;
        // for enum, there are two cases:
        // when variants in the borrow instruction are exact match with the variants in the API, we can directly call the API;
        // when variants in the borrow instruction are a subset of the variants in the API,
        // we need to generate code for calling the test variant API for each variant in the instruction;
        // when variants in the borrow instruction are intersect with the variants in the API, we need to generate an error;
        let mut found = false;
        for ((cur_variants_opt, cur_offset, _), borrow_field_handle_index) in &vec {
            if offset == cur_offset {
                // handle borrow from a struct
                if cur_variants_opt.is_none() {
                    assert!(variants.is_none());
                    self.abstract_push_args(ctx, source, None);
                    // need to freeze mutable ref if the target is an immutable reference
                    if is_mut_src && !is_mut_dest {
                        self.emit(FF::Bytecode::FreezeRef);
                    }
                    if inst.is_empty() {
                        self.emit(FF::Bytecode::Call(*borrow_field_handle_index));
                    } else {
                        let function_instantiation_index =
                            self.genr.function_instantiation_index_from_handle_index(
                                *borrow_field_handle_index,
                                &ctx.fun_ctx.module,
                                &fun_ctx.loc,
                                inst.to_vec(),
                            );
                        self.emit(FF::Bytecode::CallGeneric(function_instantiation_index));
                    }
                    self.abstract_pop_n(ctx, source.len());
                    self.abstract_push_result(ctx, dest);
                    found = true;
                    break;
                } else if let Some(cur_variants) = cur_variants_opt {
                    // handle borrow from an enum
                    assert!(variants.is_some());
                    let cur_variants_set = cur_variants.iter().collect::<BTreeSet<_>>();
                    let variants_set = variants.as_ref().unwrap().iter().collect::<BTreeSet<_>>();
                    // case 1: exact match
                    if cur_variants_set == variants_set {
                        self.abstract_push_args(ctx, source, None);
                        // need to freeze mutable ref if the target is an immutable reference
                        if is_mut_src && !is_mut_dest {
                            self.emit(FF::Bytecode::FreezeRef);
                        }
                        if inst.is_empty() {
                            self.emit(FF::Bytecode::Call(*borrow_field_handle_index));
                        } else {
                            let function_instantiation_index =
                                self.genr.function_instantiation_index_from_handle_index(
                                    *borrow_field_handle_index,
                                    &ctx.fun_ctx.module,
                                    &fun_ctx.loc,
                                    inst.to_vec(),
                                );
                            self.emit(FF::Bytecode::CallGeneric(function_instantiation_index));
                        }
                        self.abstract_pop_n(ctx, source.len());
                        self.abstract_push_result(ctx, dest);
                        found = true;
                        break;
                    } else if variants_set.is_subset(&cur_variants_set) {
                        // case 2: variants in the borrow field are a subset of the variants in the api
                        let intersected_variants = variants_set
                            .intersection(&cur_variants_set)
                            .collect::<Vec<_>>();
                        let test_variant_api_map = self.genr.struct_api_test_variant_index(
                            &ctx.fun_ctx.module,
                            &fun_ctx.loc,
                            struct_env,
                        );
                        // before calling the test variant function, we need to flush the stack
                        self.abstract_flush_stack_before(ctx, 0);
                        let mut local_to_freeze = None;
                        if is_mut_src {
                            self.abstract_push_args(ctx, source, Some(&AssignKind::Copy));
                            self.emit(FF::Bytecode::FreezeRef);
                            let (temp, _) = self.stack.pop().unwrap();
                            let ty = fun_ctx.temp_type(temp).skip_reference();
                            let new_local = self.new_local(fun_ctx, ty.wrap_in_reference(false));
                            self.emit(FF::Bytecode::StLoc(new_local));
                            local_to_freeze = Some(new_local);
                        }
                        // the base code offset to compute the target offset for each test variant
                        let code_offset_for_test_variant = self.code.len();
                        let variants_size = intersected_variants.len();
                        // TODO(#18319): optimize code generation here.
                        // offset count for each test variant:
                        // if the source is a mutable reference:
                        //    copy source
                        //    freeze ref
                        //    stloc new_local
                        // code_offset_for_test_variant is the length of code here.
                        // 1. copy source / copy new_local
                        // 2. test variant
                        // 3. btrue L2
                        // 4. copy source / copy new_local
                        // 5. test variant
                        // 6. btrue L2
                        let offset_count_for_each_variant = 3;
                        // if the source is a mutable reference:
                        //     move new_local
                        //     pop
                        // 7. move source to stack
                        // 8. pop
                        // 9. ld 64
                        // 10. abort
                        // L2: if the source is a mutable reference:
                        //     move new_local
                        //     pop
                        // ...
                        let offset_count_for_abort = if is_mut_src { 6 } else { 4 };
                        for variant in intersected_variants {
                            if let Some(test_variant_handle_index) =
                                test_variant_api_map.get(variant)
                            {
                                // self.abstract_push_args(ctx, source, Some(&AssignKind::Copy));
                                // for test variant, we need to freeze mutable ref if the source is a mutable reference
                                if let Some(local) = local_to_freeze {
                                    self.emit(FF::Bytecode::CopyLoc(local));
                                } else {
                                    self.abstract_push_args(ctx, source, Some(&AssignKind::Copy));
                                }
                                if inst.is_empty() {
                                    self.emit(FF::Bytecode::Call(*test_variant_handle_index));
                                } else {
                                    let function_instantiation_index =
                                        self.genr.function_instantiation_index_from_handle_index(
                                            *test_variant_handle_index,
                                            &ctx.fun_ctx.module,
                                            &fun_ctx.loc,
                                            inst.to_vec(),
                                        );
                                    self.emit(FF::Bytecode::CallGeneric(
                                        function_instantiation_index,
                                    ));
                                }
                                if local_to_freeze.is_none() {
                                    self.abstract_pop_n(ctx, source.len());
                                }

                                // Block when the test variant fails.
                                self.code.push(FF::Bytecode::BrTrue(
                                    (code_offset_for_test_variant
                                        + variants_size * offset_count_for_each_variant
                                        + offset_count_for_abort)
                                        as FF::CodeOffset,
                                ));
                            } else {
                                fun_ctx.internal_error(format!(
                                    "cannot perform test variant {} of {} at offset {}",
                                    variant.display(struct_env.env().symbol_pool()),
                                    struct_env.get_full_name_with_address(),
                                    offset
                                ));
                            }
                        }
                        if let Some(local) = local_to_freeze {
                            self.emit(FF::Bytecode::MoveLoc(local));
                            self.emit(FF::Bytecode::Pop);
                        }
                        // when all the test variants fail, we need to abort the function
                        let local = self.temp_to_local(ctx.fun_ctx, Some(ctx.attr_id), source[0]);
                        self.code.push(FF::Bytecode::MoveLoc(local));
                        self.code.push(FF::Bytecode::Pop);
                        self.code
                            .push(FF::Bytecode::LdU64(well_known::INCOMPLETE_MATCH_ABORT_CODE));
                        self.code.push(FF::Bytecode::Abort);
                        if let Some(local) = local_to_freeze {
                            self.emit(FF::Bytecode::MoveLoc(local));
                            self.emit(FF::Bytecode::Pop);
                        }
                        // call borrow field api
                        self.abstract_push_args(ctx, source, None);
                        // need to freeze mutable ref if the target is an immutable reference
                        if is_mut_src && !is_mut_dest {
                            self.emit(FF::Bytecode::FreezeRef);
                        }
                        if inst.is_empty() {
                            self.emit(FF::Bytecode::Call(*borrow_field_handle_index));
                        } else {
                            let function_instantiation_index =
                                self.genr.function_instantiation_index_from_handle_index(
                                    *borrow_field_handle_index,
                                    &ctx.fun_ctx.module,
                                    &fun_ctx.loc,
                                    inst.to_vec(),
                                );
                            self.emit(FF::Bytecode::CallGeneric(function_instantiation_index));
                        }
                        self.abstract_pop_n(ctx, source.len());
                        self.abstract_push_result(ctx, dest);
                        found = true;
                        break;
                    } else if !variants_set
                        .intersection(&cur_variants_set)
                        .collect::<Vec<_>>()
                        .is_empty()
                    {
                        // handle the case e.x where x appears in multiple variants at the same offset but have different or may be instantiated with different types.
                        struct_env.env().error(&fun_ctx.fun.get_bytecode_loc(ctx.attr_id), &format!("cannot perform borrow operation for field `{}` for struct `{}` since it has or may be instantiated with different types in variants",
                        struct_env.get_field_by_offset_optional_variant(Some(cur_variants[0]), *offset).get_name().display(struct_env.env().symbol_pool()),
                        struct_env.get_full_name_with_address()));
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            fun_ctx.internal_error(format!(
                "cannot generate borrow field for {} at offset {}",
                struct_env.get_full_name_with_address(),
                offset
            ));
        }
    }

    /// Generates call to test variant api.
    fn gen_test_variant_api_call(
        &mut self,
        ctx: &BytecodeContext<'_>,
        dest: &[usize],
        source: &[usize],
        fun_ctx: &FunctionContext<'_>,
        variant: &Symbol,
        inst: &[Type],
        struct_env: &StructEnv<'_>,
        is_mut_src: bool,
    ) {
        let test_variant_api_map =
            self.genr
                .struct_api_test_variant_index(&ctx.fun_ctx.module, &fun_ctx.loc, struct_env);
        if let Some(handle_index) = test_variant_api_map.get(variant) {
            self.abstract_push_args(ctx, source, None);
            // need to freeze mutable ref
            if is_mut_src {
                self.emit(FF::Bytecode::FreezeRef);
            }
            if inst.is_empty() {
                self.emit(FF::Bytecode::Call(*handle_index));
            } else {
                let function_instantiation_index =
                    self.genr.function_instantiation_index_from_handle_index(
                        *handle_index,
                        &ctx.fun_ctx.module,
                        &fun_ctx.loc,
                        inst.to_vec(),
                    );
                self.emit(FF::Bytecode::CallGeneric(function_instantiation_index));
            }
            self.abstract_pop_n(ctx, source.len());
            self.abstract_push_result(ctx, dest);
        } else {
            fun_ctx.internal_error(format!(
                "variant {} not found in test variant api map",
                variant.display(struct_env.env().symbol_pool())
            ));
        }
    }

    /// Generates call to pack/unpack API.
    fn gen_pack_unpack_api_call<const IS_PACK: bool>(
        &mut self,
        ctx: &BytecodeContext<'_>,
        dest: &[usize],
        source: &[usize],
        fun_ctx: &FunctionContext<'_>,
        variant_opt: &Option<Symbol>,
        inst: &[Type],
        struct_env: &StructEnv<'_>,
    ) {
        let api_map = if IS_PACK {
            self.genr
                .struct_api_pack_index(&ctx.fun_ctx.module, &fun_ctx.loc, struct_env)
        } else {
            self.genr
                .struct_api_unpack_index(&ctx.fun_ctx.module, &fun_ctx.loc, struct_env)
        };
        if let Some(handle_index) = api_map.get(variant_opt) {
            self.abstract_push_args(ctx, source, None);
            if inst.is_empty() {
                self.emit(FF::Bytecode::Call(*handle_index));
            } else {
                let function_instantiation_index =
                    self.genr.function_instantiation_index_from_handle_index(
                        *handle_index,
                        &ctx.fun_ctx.module,
                        &fun_ctx.loc,
                        inst.to_vec(),
                    );
                self.emit(FF::Bytecode::CallGeneric(function_instantiation_index));
            }
            self.abstract_pop_n(ctx, source.len());
            self.abstract_push_result(ctx, dest);
        } else {
            fun_ctx.internal_error(format!(
                "not found in {} api map",
                if IS_PACK { "pack" } else { "unpack" }
            ));
        }
    }

    /// Generates code for a function call.
    fn gen_call(
        &mut self,
        ctx: &BytecodeContext,
        dest: &[TempIndex],
        id: QualifiedId<FunId>,
        inst: &[Type],
        source: &[TempIndex],
    ) {
        let fun_ctx = ctx.fun_ctx;
        self.abstract_push_args(ctx, source, None);
        if let Some(opcode) = ctx.fun_ctx.module.get_well_known_function_code(
            &ctx.fun_ctx.loc,
            id,
            Some(
                self.genr
                    .signature(&ctx.fun_ctx.module, &ctx.fun_ctx.loc, inst.to_vec()),
            ),
        ) {
            self.emit(opcode)
        } else if inst.is_empty() {
            let idx = self.genr.function_index(
                &fun_ctx.module,
                &fun_ctx.loc,
                &fun_ctx.module.env.get_function(id),
            );
            self.emit(FF::Bytecode::Call(idx))
        } else {
            let idx = self.genr.function_instantiation_index(
                &fun_ctx.module,
                &fun_ctx.loc,
                &fun_ctx.module.env.get_function(id),
                inst.to_vec(),
            );
            self.emit(FF::Bytecode::CallGeneric(idx))
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generates code for construction of a closure.
    fn gen_closure(
        &mut self,
        ctx: &BytecodeContext,
        dest: &[TempIndex],
        id: QualifiedId<FunId>,
        inst: &[Type],
        mask: ClosureMask,
        source: &[TempIndex],
    ) {
        let fun_ctx = ctx.fun_ctx;
        self.abstract_push_args(ctx, source, None);
        if inst.is_empty() {
            let idx = self.genr.function_index(
                &fun_ctx.module,
                &fun_ctx.loc,
                &fun_ctx.module.env.get_function(id),
            );
            self.emit(FF::Bytecode::PackClosure(idx, mask))
        } else {
            let idx = self.genr.function_instantiation_index(
                &fun_ctx.module,
                &fun_ctx.loc,
                &fun_ctx.module.env.get_function(id),
                inst.to_vec(),
            );
            self.emit(FF::Bytecode::PackClosureGeneric(idx, mask))
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generates code for invoking of a closure.
    fn gen_invoke(&mut self, ctx: &BytecodeContext, dest: &[TempIndex], source: &[TempIndex]) {
        let clos_type = ctx
            .fun_ctx
            .fun
            .get_local_type(*source.last().expect("invoke has function argument last"));
        let sign_idx = self
            .genr
            .signature(&ctx.fun_ctx.module, &ctx.fun_ctx.loc, vec![
                clos_type.clone()
            ]);
        self.abstract_push_args(ctx, source, None);
        self.emit(FF::Bytecode::CallClosure(sign_idx));
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generates code for an operation working on a structure. This can be a structure with or
    /// without generics: the two passed functions allow the caller to determine which bytecode
    /// to create for each case.
    fn gen_struct_oper(
        &mut self,
        ctx: &BytecodeContext,
        dest: &[TempIndex],
        id: QualifiedId<StructId>,
        inst: &[Type],
        source: &[TempIndex],
        mk_simple: impl FnOnce(FF::StructDefinitionIndex) -> FF::Bytecode,
        mk_generic: impl FnOnce(FF::StructDefInstantiationIndex) -> FF::Bytecode,
    ) {
        let fun_ctx = ctx.fun_ctx;
        self.abstract_push_args(ctx, source, None);
        let struct_env = &fun_ctx.module.env.get_struct(id);
        if inst.is_empty() {
            let idx = self
                .genr
                .struct_def_index(&fun_ctx.module, &fun_ctx.loc, struct_env);
            self.emit(mk_simple(idx))
        } else {
            let idx = self.genr.struct_def_instantiation_index(
                &fun_ctx.module,
                &fun_ctx.loc,
                struct_env,
                inst.to_vec(),
            );
            self.emit(mk_generic(idx))
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    fn gen_struct_variant_oper(
        &mut self,
        ctx: &BytecodeContext,
        dest: &[TempIndex],
        id: QualifiedId<StructId>,
        variant: Symbol,
        inst: &[Type],
        source: &[TempIndex],
        mk_simple: impl FnOnce(FF::StructVariantHandleIndex) -> FF::Bytecode,
        mk_generic: impl FnOnce(FF::StructVariantInstantiationIndex) -> FF::Bytecode,
    ) {
        let fun_ctx = ctx.fun_ctx;
        self.abstract_push_args(ctx, source, None);
        let struct_env = &fun_ctx.module.env.get_struct(id);
        if inst.is_empty() {
            let idx =
                self.genr
                    .struct_variant_index(&fun_ctx.module, &fun_ctx.loc, struct_env, variant);
            self.emit(mk_simple(idx))
        } else {
            let idx = self.genr.struct_variant_inst_index(
                &fun_ctx.module,
                &fun_ctx.loc,
                struct_env,
                variant,
                inst.to_vec(),
            );
            self.emit(mk_generic(idx))
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generate code for the borrow-field instruction.
    fn gen_borrow_field(
        &mut self,
        ctx: &BytecodeContext,
        dest: &[TempIndex],
        id: QualifiedId<StructId>,
        inst: Vec<Type>,
        variants: Option<&[Symbol]>,
        offset: usize,
        source: &[TempIndex],
    ) {
        let fun_ctx = ctx.fun_ctx;
        self.abstract_push_args(ctx, source, None);
        let struct_env = &fun_ctx.module.env.get_struct(id);
        let is_mut = fun_ctx.fun.get_local_type(dest[0]).is_mutable_reference();

        if let Some(variants) = variants {
            assert!(!variants.is_empty());
            let field_env =
                &struct_env.get_field_by_offset_optional_variant(Some(variants[0]), offset);
            if inst.is_empty() {
                let idx = self.genr.variant_field_index(
                    &fun_ctx.module,
                    &fun_ctx.loc,
                    variants,
                    field_env,
                );
                if is_mut {
                    self.emit(FF::Bytecode::MutBorrowVariantField(idx))
                } else {
                    self.emit(FF::Bytecode::ImmBorrowVariantField(idx))
                }
            } else {
                let idx = self.genr.variant_field_inst_index(
                    &fun_ctx.module,
                    &fun_ctx.loc,
                    variants,
                    field_env,
                    inst,
                );
                if is_mut {
                    self.emit(FF::Bytecode::MutBorrowVariantFieldGeneric(idx))
                } else {
                    self.emit(FF::Bytecode::ImmBorrowVariantFieldGeneric(idx))
                }
            }
        } else {
            let field_env = &struct_env.get_field_by_offset_optional_variant(None, offset);
            if inst.is_empty() {
                let idx = self
                    .genr
                    .field_index(&fun_ctx.module, &fun_ctx.loc, field_env);
                if is_mut {
                    self.emit(FF::Bytecode::MutBorrowField(idx))
                } else {
                    self.emit(FF::Bytecode::ImmBorrowField(idx))
                }
            } else {
                let idx =
                    self.genr
                        .field_inst_index(&fun_ctx.module, &fun_ctx.loc, field_env, inst);
                if is_mut {
                    self.emit(FF::Bytecode::MutBorrowFieldGeneric(idx))
                } else {
                    self.emit(FF::Bytecode::ImmBorrowFieldGeneric(idx))
                }
            }
        }
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest);
    }

    /// Generate code for a general builtin instruction.
    fn gen_builtin(
        &mut self,
        ctx: &BytecodeContext,
        dest: &[TempIndex],
        bc: FF::Bytecode,
        source: &[TempIndex],
    ) {
        self.abstract_push_args(ctx, source, None);
        self.emit(bc);
        self.abstract_pop_n(ctx, source.len());
        self.abstract_push_result(ctx, dest)
    }

    /// Generate code for the load instruction.
    fn gen_load(&mut self, ctx: &BytecodeContext, dest: &TempIndex, cons: &Constant) {
        self.flush_any_conflicts(ctx, std::slice::from_ref(dest), &[]);
        self.gen_load_push(ctx, cons, ctx.fun_ctx.fun.get_local_type(*dest));
        self.abstract_push_result(ctx, vec![*dest]);
    }

    fn gen_load_push(&mut self, ctx: &BytecodeContext, cons: &Constant, dest_type: &Type) {
        use Constant::*;
        match cons {
            Bool(b) => {
                if *b {
                    self.emit(FF::Bytecode::LdTrue)
                } else {
                    self.emit(FF::Bytecode::LdFalse)
                }
            },
            U8(n) => self.emit(FF::Bytecode::LdU8(*n)),
            U16(n) => self.emit(FF::Bytecode::LdU16(*n)),
            U32(n) => self.emit(FF::Bytecode::LdU32(*n)),
            U64(n) => self.emit(FF::Bytecode::LdU64(*n)),
            U128(n) => self.emit(FF::Bytecode::LdU128(*n)),
            U256(n) => self.emit(FF::Bytecode::LdU256(
                move_core_types::int256::U256::from_le_bytes(n.to_le_bytes()),
            )),
            I8(n) => self.emit(FF::Bytecode::LdI8(*n)),
            I16(n) => self.emit(FF::Bytecode::LdI16(*n)),
            I32(n) => self.emit(FF::Bytecode::LdI32(*n)),
            I64(n) => self.emit(FF::Bytecode::LdI64(*n)),
            I128(n) => self.emit(FF::Bytecode::LdI128(*n)),
            I256(n) => self.emit(FF::Bytecode::LdI256(
                move_core_types::int256::I256::from_le_bytes(n.to_le_bytes()),
            )),
            Vector(vec) if vec.is_empty() => {
                self.gen_vector_load_push(ctx, vec, dest_type);
            },
            _ => {
                let cons = self.genr.constant_index(
                    &ctx.fun_ctx.module,
                    &ctx.fun_ctx.loc,
                    cons,
                    dest_type,
                );
                self.emit(FF::Bytecode::LdConst(cons));
            },
        }
    }

    fn gen_vector_load_push(&mut self, ctx: &BytecodeContext, vec: &[Constant], vec_type: &Type) {
        let fun_ctx = ctx.fun_ctx;
        let elem_type = if let Type::Vector(el) = vec_type {
            el.as_ref().clone()
        } else {
            fun_ctx.internal_error("expected vector type");
            Type::new_prim(PrimitiveType::Bool)
        };
        for cons in vec.iter() {
            self.gen_load_push(ctx, cons, &elem_type);
        }
        let sign = self
            .genr
            .signature(&fun_ctx.module, &fun_ctx.loc, vec![elem_type]);
        self.emit(FF::Bytecode::VecPack(sign, vec.len() as u64));
    }

    /// Generates code for an inline spec block. The spec block needs
    /// to be rewritten s.t. free temporaries are replaced by the assigned
    /// locals. The spec block is then stored in the spec block table
    /// which will be written back to the function spec at the end of
    /// translation. In the actual Move bytecode, a `Nop` is inserted
    /// at the current code offset.
    fn gen_spec_block(&mut self, ctx: &BytecodeContext, spec: &Spec) {
        let mut replacer = |id: NodeId, target: RewriteTarget| {
            if let RewriteTarget::Temporary(temp) = target {
                Some(
                    ExpData::Temporary(
                        id,
                        self.temps.get(&temp).expect("temp has mapping").local as TempIndex,
                    )
                    .into_exp(),
                )
            } else {
                None
            }
        };
        let (_, spec) = ExpRewriter::new(ctx.fun_ctx.module.env, &mut replacer)
            .rewrite_spec_descent(&SpecBlockTarget::Inline, spec);
        self.spec_blocks.insert(self.code.len() as CodeOffset, spec);
        self.emit(FF::Bytecode::Nop)
    }

    /// Remap the spec blocks, given the mapping of new offsets to original offsets.
    fn remap_spec_blocks(&mut self, new_to_original_offsets: &[CodeOffset]) {
        if new_to_original_offsets.is_empty() {
            return;
        }
        let old_to_new = new_to_original_offsets
            .iter()
            .enumerate()
            .map(|(new_offset, old_offset)| (*old_offset, new_offset as CodeOffset))
            .collect::<BTreeMap<_, _>>();
        let largest_offset = (new_to_original_offsets.len() - 1) as CodeOffset;

        // Rewrite the spec blocks mapping.
        self.spec_blocks = std::mem::take(&mut self.spec_blocks)
            .into_iter()
            .map(|(old_offset, spec)| {
                // If there is no mapping found for the old offset, then we use the next largest
                // offset. If there is no such offset, then we use the overall largest offset.
                let new_offset = old_to_new
                    .range(old_offset..)
                    .next()
                    .map(|(_, v)| *v)
                    .unwrap_or(largest_offset);
                (new_offset, spec)
            })
            .collect::<BTreeMap<_, _>>();
    }

    /// Emits a file-format bytecode.
    fn emit(&mut self, bc: FF::Bytecode) {
        self.code.push(bc)
    }

    /// Ensure that on the abstract stack of the generator, the given temporaries are ready,
    /// in order, to be consumed. Ideally those are already on the stack, but if they are not,
    /// they will be made available.
    fn abstract_push_args(
        &mut self,
        ctx: &BytecodeContext,
        temps: impl AsRef<[TempIndex]>,
        push_kind: Option<&AssignKind>,
    ) {
        let fun_ctx = ctx.fun_ctx;
        let temps = temps.as_ref();
        // Ensure that temps on the stack which are used after this point are saved to locals.
        self.save_used_after(ctx, temps);
        // Now compute which temps need to be pushed, on top of any which are already on the stack
        let mut temps_to_push = self.analyze_stack(temps);
        // If any of the temps we need to push now are actually underneath the temps already on the stack,
        // and their values were not copied from locals, we need to even flush more of the stack to reach them.
        let mut stack_to_flush = self.stack.len();
        for temp in temps_to_push {
            if let Some(offs) = self
                .stack
                .iter()
                .position(|(t, copied)| !*copied && t == temp)
            {
                // The lowest point in the stack we need to flush.
                stack_to_flush = std::cmp::min(offs, stack_to_flush);
                // Unfortunately, whatever is on the stack already, needs to be flushed out and
                // pushed again. (We really should introduce a ROTATE opcode to the Move VM)
                temps_to_push = temps;
            }
        }
        self.abstract_flush_stack_before(ctx, stack_to_flush);
        // Finally, push `temps_to_push` onto the stack.
        for (pos, temp) in temps_to_push.iter().enumerate() {
            let local = self.temp_to_local(fun_ctx, Some(ctx.attr_id), *temp);
            let copied;
            match push_kind {
                Some(AssignKind::Move) => {
                    copied = false;
                    self.emit(FF::Bytecode::MoveLoc(local));
                },
                Some(AssignKind::Copy) => {
                    copied = true;
                    self.emit(FF::Bytecode::CopyLoc(local));
                },
                Some(AssignKind::Inferred) | Some(AssignKind::Store) => {
                    copied = false;
                    fun_ctx
                        .internal_error("Inferred and Store AssignKind should be not appear here.");
                },
                None => {
                    // Copy the temporary if it is copyable and still used after this code point, or
                    // if it appears again in temps_to_push.
                    if ctx.is_alive_after(*temp, &temps_to_push[pos + 1..], true) {
                        if !fun_ctx.is_copyable(*temp) {
                            fun_ctx.module.internal_error(
                                ctx.fun_ctx.fun.get_bytecode_loc(ctx.attr_id),
                                format!("value in `$t{}` expected to be copyable", temp),
                            )
                        }
                        copied = true;
                        self.emit(FF::Bytecode::CopyLoc(local))
                    } else {
                        copied = false;
                        self.emit(FF::Bytecode::MoveLoc(local));
                    }
                },
            }
            self.stack.push((*temp, copied));
        }
    }

    /// If a temp already on the abstract stack is both:
    ///   - not a source of the current instruction
    ///   - destination of the current instruction
    /// then, we have a conflicting write to that temp.
    ///
    /// This method ensures that conflicting writes do not happen by flushing out such temps
    /// from the abstract stack before emitting code for the current instruction.
    fn flush_any_conflicts(
        &mut self,
        ctx: &BytecodeContext,
        dests: &[TempIndex],
        sources: &[TempIndex],
    ) {
        let dests = BTreeSet::from_iter(dests.iter());
        let sources = BTreeSet::from_iter(sources.iter());
        let conflicts = dests.difference(&sources).collect::<BTreeSet<_>>();
        if let Some(pos) = self.stack.iter().position(|(t, _)| conflicts.contains(&t)) {
            self.abstract_flush_stack_before(ctx, pos);
        }
    }

    /// Ensures that all `temps` which are on the stack, were not copied from a local,
    /// and used after this program point are saved to locals. This flushes the stack
    /// as deep as needed for this.
    fn save_used_after(&mut self, ctx: &BytecodeContext, temps: &[TempIndex]) {
        let mut stack_to_flush = self.stack.len();
        for (i, temp) in temps.iter().enumerate() {
            if let Some(pos) = self
                .stack
                .iter()
                .position(|(t, copied)| !*copied && t == temp)
            {
                if ctx.is_alive_after(*temp, &temps[i + 1..], true) {
                    // Determine new lowest point to which we need to flush
                    stack_to_flush = std::cmp::min(stack_to_flush, pos);
                }
            }
        }
        // Notice that we flush the stack _before_ the next processed instruction, therefore
        // we use the before version of the below function.
        self.abstract_flush_stack_before(ctx, stack_to_flush)
    }

    /// Determines the maximal prefix of `temps` which are already on the stack, and
    /// returns the temps which are not and need to be pushed.
    fn analyze_stack<'t>(&mut self, temps: &'t [TempIndex]) -> &'t [TempIndex] {
        let mut temps_to_push = temps; // worst case need to push all
        let stack = self.stack.iter().map(|(t, _)| *t).collect::<Vec<_>>();
        for end in (1..=temps.len()).rev() {
            if stack.ends_with(&temps[0..end]) {
                // We found 0..end temps which are already on top of the stack. The remaining ones
                // need to be pushed.
                temps_to_push = &temps[end..temps.len()];
                break;
            }
        }
        temps_to_push
    }

    /// Flush the abstract stack, ensuring that all values on the stack are stored in locals, if
    /// they are still alive. The `before` parameter determines whether we care about
    /// variables alive before or after the current program point.
    fn abstract_flush_stack(&mut self, ctx: &BytecodeContext, top: usize, before: bool) {
        let fun_ctx = ctx.fun_ctx;
        while self.stack.len() > top {
            let (temp, copied) = self.stack.pop().unwrap();
            if !copied
                && ((before && ctx.is_alive_before(temp))
                    || (!before && ctx.is_alive_after(temp, &[], false))
                    || self.pinned.contains(&temp)
                    || !ctx.fun_ctx.is_droppable(temp))
            {
                // Only need to save to a local if the temp is: not copied from a local AND,
                // still used afterwards, is pinned, or is not droppable.
                let local = self.temp_to_local(fun_ctx, Some(ctx.attr_id), temp);
                self.emit(FF::Bytecode::StLoc(local));
            } else {
                self.emit(FF::Bytecode::Pop)
            }
        }
    }

    /// Shortcut for `abstract_flush_stack(..., true)`
    fn abstract_flush_stack_before(&mut self, ctx: &BytecodeContext, top: usize) {
        self.abstract_flush_stack(ctx, top, true)
    }

    /// Shortcut for `abstract_flush_stack(..., false)`
    fn abstract_flush_stack_after(&mut self, ctx: &BytecodeContext, top: usize) {
        self.abstract_flush_stack(ctx, top, false)
    }

    /// Push the result of an operation to the abstract stack.
    fn abstract_push_result(&mut self, ctx: &BytecodeContext, result: impl AsRef<[TempIndex]>) {
        let pre_stack_len = self.stack.len();
        let result = result.as_ref();
        // `flush_mark` is used to find the lowest index in `result` from which we flush.
        let mut flush_mark = result.len();
        // Push the temps in `result` onto the stack.
        // Make `flush_mark` the index of the earliest temp that is pinned.
        for (i, temp) in result.iter().enumerate() {
            if self.pinned.contains(temp) {
                // need to flush this right away and maintain a local for it
                flush_mark = flush_mark.min(i);
            }
            // The result was not copied from a local.
            self.stack.push((*temp, false));
        }
        // Check if there are any temps that could be flushed right away.
        // We only need to check from below the `flush_mark` (everything at `flush_mark`
        // and above will be flushed anyway). We work down the `result` and if we find a
        // temp that doesn't need to be flushed, we stop. Because, that temp could get
        // consumed by a subsequent instruction and may never need to be flushed.
        if let Some(flush_writes) = ctx.get_writes_to_flush() {
            for (i, temp) in result[..flush_mark].iter().enumerate().rev() {
                if flush_writes.contains(temp) {
                    flush_mark = i;
                } else {
                    break;
                }
            }
        }
        let stack_flush_mark = pre_stack_len + flush_mark;
        self.abstract_flush_stack_after(ctx, stack_flush_mark);
    }

    /// Pop a value from the abstract stack.
    fn abstract_pop(&mut self, ctx: &BytecodeContext) {
        if self.stack.pop().is_none() {
            ctx.fun_ctx.internal_error("unbalanced abstract stack")
        }
    }

    /// Pop a number of values from the abstract stack.
    fn abstract_pop_n(&mut self, ctx: &BytecodeContext, cnt: usize) {
        for _ in 0..cnt {
            self.abstract_pop(ctx)
        }
    }

    /// Creates a new local of type.
    fn new_local(&mut self, ctx: &FunctionContext, ty: Type) -> FF::LocalIndex {
        let local = ctx
            .module
            .checked_bound(&ctx.loc, self.locals.len(), MAX_LOCAL_COUNT, "local")
            as FF::LocalIndex;
        self.locals.push(ty);
        local
    }

    /// Allocates a local for the given temporary.
    /// If a local is not already available, then allocates one.
    /// While allocating one, it adds it to the source map, unless
    /// it is a parameter (these are recorded elsewhere).
    fn temp_to_local(
        &mut self,
        ctx: &FunctionContext,
        bc_attr_opt: Option<AttrId>,
        temp: TempIndex,
    ) -> FF::LocalIndex {
        if let Some(TempInfo { local }) = self.temps.get(&temp) {
            *local
        } else {
            let idx = self.new_local(ctx, ctx.temp_type(temp).to_owned());
            self.temps.insert(temp, TempInfo::new(idx));

            if temp < ctx.fun.get_parameter_count() {
                // `temp` is a parameter.
                // Don't add it to the source map here.
                idx
            } else {
                let loc = if let Some(id) = bc_attr_opt {
                    // Have a bytecode specific location for this local
                    ctx.fun.get_bytecode_loc(id)
                } else {
                    // Fall back to function identifier
                    ctx.fun.func_env.get_id_loc()
                };
                // Only add to the source map if it wasn't a parameter.
                let name = ctx.fun.get_local_name(temp);
                self.genr
                    .source_map
                    .add_local_mapping(ctx.def_idx, ctx.module.source_name(name, loc))
                    .expect(SOURCE_MAP_OK);
                idx
            }
        }
    }
}

impl FunctionContext<'_> {
    /// Emits an internal error for this function.
    pub fn internal_error(&self, msg: impl AsRef<str>) {
        self.module.internal_error(
            &self.loc,
            format!("file format generator: {}", msg.as_ref()),
        )
    }

    /// Gets the type of the temporary.
    pub fn temp_type(&self, temp: TempIndex) -> &Type {
        self.fun.get_local_type(temp)
    }

    /// Returns true of the given temporary can/should be copied when it is loaded onto the stack.
    /// Currently, this is using the `Copy` ability, but in the future it may also use lifetime
    /// pipeline results to check whether the variable is still accessed.
    pub fn is_copyable(&self, temp: TempIndex) -> bool {
        self.module
            .env
            .type_abilities(self.temp_type(temp), &self.type_parameters)
            .has_ability(ability::Ability::Copy)
    }

    /// Returns true of the given temporary can/should be dropped when flushing the stack.
    pub fn is_droppable(&self, temp: TempIndex) -> bool {
        self.module
            .env
            .type_abilities(self.temp_type(temp), &self.type_parameters)
            .has_ability(ability::Ability::Drop)
    }
}

impl BytecodeContext<'_> {
    /// Determine whether `temp` is alive (used) in the reachable code after this point,
    /// or is part of the remaining argument list. When `dest_check` is true, we additionally
    /// check if `temp` is also written to by the current instruction; if it is, then the
    /// definition of `temp` being considered here is killed, making it not alive after this point.
    pub fn is_alive_after(
        &self,
        temp: TempIndex,
        remaining_args: &[TempIndex],
        dest_check: bool,
    ) -> bool {
        if remaining_args.contains(&temp) {
            // Temp is used another time in the same argument list of this instruction, and
            // is alive after even if it is a destination
            return true;
        }
        let bc = &self.fun_ctx.fun.data.code[self.code_offset as usize];
        if dest_check && bc.dests().contains(&temp) {
            return false;
        }
        let an = self
            .fun_ctx
            .fun
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("livevar analysis result");
        an.get_live_var_info_at(self.code_offset)
            .map(|a| a.after.contains_key(&temp))
            .unwrap_or(false)
    }

    /// Determine whether `temp` is alive (used) in the reachable code before and until
    /// this point.
    pub fn is_alive_before(&self, temp: TempIndex) -> bool {
        let an = self
            .fun_ctx
            .fun
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("livevar analysis result");
        an.get_live_var_info_at(self.code_offset)
            .map(|a| a.before.contains_key(&temp))
            .unwrap_or(false)
    }

    /// Get the set of temps to flush if possible at the current code offset.
    pub fn get_writes_to_flush(&self) -> Option<&BTreeSet<TempIndex>> {
        self.fun_ctx
            .fun
            .get_annotations()
            .get::<FlushWritesAnnotation>()
            .and_then(|annotation| annotation.0.get(&self.code_offset))
    }
}
