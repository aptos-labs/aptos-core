// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Transformation which injects data invariants into the bytecode.
//!
//! This transformation adds the data invariant to all occurrences of the `WellFormed(x)` call
//! which has been introduced by the spec instrumenter, by essentially transforming `WellFormed(x)`
//! into `WellFormed(x) && <data invariant>(x)`. The `WellFormed` expressions are maintained in the
//! output for processing by the backend, in case type assumptions needed to be added by the backend
//! (which depends on the compilation scheme). It also handles PackRef/PackRefDeep
//! instructions introduced by memory instrumentation, as well as the Pack instructions.

use crate::options::ProverOptions;
use crate::variant_context_analysis::{VariantContextAnnotation, VariantContext};
use move_model::{
    ast,
    ast::{ConditionKind, Exp, ExpData, QuantKind, RewriteResult, TempIndex},
    exp_generator::ExpGenerator,
    metadata::LanguageVersion,
    model::{FunctionEnv, Loc, NodeId, StructEnv, QualifiedInstId},
    pragmas::{INTRINSIC_FUN_MAP_SPEC_GET, INTRINSIC_TYPE_MAP},
    ty::Type,
    well_known,
};
use move_stackless_bytecode::{
    function_data_builder::FunctionDataBuilder,
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation, PropKind},
};
use move_model::symbol::Symbol;

const INVARIANT_FAILS_MESSAGE: &str = "data invariant does not hold";

pub struct DataInvariantInstrumentationProcessor {}

impl DataInvariantInstrumentationProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl FunctionTargetProcessor for DataInvariantInstrumentationProcessor {
    fn process(
        &self,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() || fun_env.is_intrinsic() {
            // Nothing to do.
            return data;
        }
        let options = ProverOptions::get(fun_env.module_env.env);
        Instrumenter::run(&options, targets, fun_env, data)
    }

    fn name(&self) -> String {
        "data_invariant_instrumenter".to_string()
    }
}

struct Instrumenter<'a> {
    _options: &'a ProverOptions,
    _targets: &'a mut FunctionTargetsHolder,
    builder: FunctionDataBuilder<'a>,
    for_verification: bool,
}

impl<'a> Instrumenter<'a> {
    fn run(
        options: &'a ProverOptions,
        targets: &'a mut FunctionTargetsHolder,
        fun_env: &FunctionEnv<'a>,
        data: FunctionData,
    ) -> FunctionData {
        // Function is instrumented for verification if this is the verification variant,
        // or if it is function with a friend which is verified in the friends context.
        let for_verification = data.variant.is_verified() || fun_env.has_friend();
        let builder = FunctionDataBuilder::new(fun_env, data);
        let mut instrumenter = Instrumenter {
            _options: options,
            _targets: targets,
            builder,
            for_verification,
        };
        instrumenter.instrument();
        instrumenter.builder.data
    }

    fn instrument(&mut self) {
        // Extract and clear current code
        let old_code = std::mem::take(&mut self.builder.data.code);

        // Instrument and generate new code
        for bc in old_code {
            self.instrument_bytecode(bc.clone());
        }
    }

    fn instrument_bytecode(&mut self, bc: Bytecode) {
        use Bytecode::*;
        use Operation::*;
        match bc {
            // Remove Unpack, we currently don't need it.
            Call(_, _, UnpackRef, ..) | Call(_, _, UnpackRefDeep, ..) => {},

            // Instructions which lead to asserting data invariants.
            Call(id, dests, Pack(mid, sid, targs), srcs, aa) if self.for_verification => {
                let struct_temp = dests[0];
                self.builder
                    .emit(Call(id, dests, Pack(mid, sid, targs), srcs, aa));
                // Emit a shallow assert of the data invariant.
                self.emit_data_invariant_for_temp(false, PropKind::Assert, struct_temp);

            },
            Call(id, dests, PackVariant(mid, sid, variant, targs), srcs, aa)
                if self.for_verification =>
            {
                let struct_temp = dests[0];
                self.builder.emit(Call(
                    id,
                    dests,
                    PackVariant(mid, sid, variant, targs),
                    srcs,
                    aa,
                ));
                // Emit a shallow assert of the data invariant.
                self.emit_data_invariant_for_temp(false, PropKind::Assert, struct_temp);
            },
            Call(_, _, PackRef, srcs, _) if self.for_verification => {
                // Emit a shallow assert of the data invariant.
                self.emit_data_invariant_for_temp(false, PropKind::Assert, srcs[0]);
            },
            Call(id, _, PackRefDeep, srcs, _) if self.for_verification => {
                eprintln!("DEBUG: Processing PackRefDeep instruction: Call({:?}, PackRefDeep, {:?}, _)", id, srcs);
                // Get the type of the value being processed
                let env = self.builder.global_env();
                let ty = env.get_node_type(self.builder.mk_temporary(srcs[0]).node_id());
                eprintln!("DEBUG: PackRefDeep for type {:?}", ty);

                // Get the source location of this instruction from the node ID
                let temp_node_id = self.builder.mk_temporary(srcs[0]).node_id();
                let node_loc = env.get_node_loc(temp_node_id);
                eprintln!("DEBUG: PackRefDeep at source location: {}", node_loc.display(env));

                // Check if this is an enum type
                if let Type::Struct(mid, sid, _) = ty.skip_reference() {
                    let struct_env = env.get_module(*mid).into_struct(*sid);
                    if struct_env.has_variants() {
                        eprintln!("DEBUG: PackRefDeep for enum struct {} - need variant context",
                                 struct_env.get_name().display(env.symbol_pool()).to_string());
                    }
                }

                // Emit a deep assert of the data invariant.
                self.emit_data_invariant_for_temp(true, PropKind::Assert, srcs[0]);
            },

            // Augment WellFormed calls in assumptions. Currently those cannot appear in assertions.
            // We leave the old WellFormed check for the backend to process any type related
            // assumptions.
            Prop(id, PropKind::Assume, exp) => {
                let mut rewriter = |e: Exp| {
                    if let ExpData::Call(_, ast::Operation::WellFormed, args) = e.as_ref() {
                        let inv = self.builder.mk_join_bool(
                            ast::Operation::And,
                            self.translate_invariant(true, args[0].clone())
                                .into_iter()
                                .map(|(_, e)| e),
                        );
                        let e = self
                            .builder
                            .mk_join_opt_bool(ast::Operation::And, Some(e), inv)
                            .unwrap();
                        RewriteResult::Rewritten(e)
                    } else {
                        RewriteResult::Unchanged(e)
                    }
                };
                let exp = ExpData::rewrite(exp, &mut rewriter);
                self.builder.emit(Prop(id, PropKind::Assume, exp));
            },
            _ => self.builder.emit(bc),
        }
    }

    /// Emits a data invariant, shallow or deep, assume or assert, for the value in temporary.
    fn emit_data_invariant_for_temp(&mut self, deep: bool, kind: PropKind, temp: TempIndex) {
        let temp_exp = self.builder.mk_temporary(temp);
        for (loc, inv) in self.translate_invariant(deep, temp_exp) {
            self.builder.set_next_debug_comment(format!(
                "data invariant {}",
                loc.display(self.builder.global_env())
            ));
            if kind == PropKind::Assert {
                self.builder
                    .set_loc_and_vc_info(loc, INVARIANT_FAILS_MESSAGE);
            }
            self.builder.emit_with(|id| Bytecode::Prop(id, kind, inv));
        }
    }

    fn translate_invariant(&self, deep: bool, value: Exp) -> Vec<(Loc, Exp)> {
        self.translate_invariant_with_variant(deep, value, None)
    }

    /// New helper: propagate parent enum/variant context
    fn translate_invariant_with_variant(
        &self,
        deep: bool,
        value: Exp,
        parent_enum_variant: Option<(Type, Symbol, Exp)>,
    ) -> Vec<(Loc, Exp)> {
        let env = self.builder.global_env();
        let ty = env.get_node_type(value.node_id());
        match ty.skip_reference() {
            Type::Struct(mid, sid, targs) => {
                let struct_env = env.get_module(*mid).into_struct(*sid);
                if struct_env.is_intrinsic_of(INTRINSIC_TYPE_MAP) {
                    let decl = env
                        .get_intrinsics()
                        .get_decl_for_struct(&mid.qualified(*sid))
                        .expect("intrinsic declaration");
                    let spec_fun_get = decl
                        .lookup_spec_fun(env, INTRINSIC_FUN_MAP_SPEC_GET)
                        .expect("intrinsic map_get function");
                    let mut loc = env.unknown_loc();
                    let quant = self.builder.mk_map_quant_opt(
                        QuantKind::Forall,
                        value,
                        spec_fun_get,
                        &targs[0],
                        &targs[1],
                        &mut |e| {
                            let invs = self.translate_invariant_with_variant(deep, e, parent_enum_variant.clone());
                            if !invs.is_empty() {
                                loc = invs[0].0.clone();
                            }
                            self.builder.mk_join_bool(ast::Operation::And, invs.into_iter().map(|(_, e)| e))
                        },
                    );
                    if let Some(e) = quant {
                        vec![(loc, e)]
                    } else {
                        vec![]
                    }
                } else {
                    self.translate_invariant_for_struct_with_variant(deep, value, struct_env, targs, parent_enum_variant)
                }
            },
            Type::Vector(ety) => {
                let mut loc = env.unknown_loc();
                let quant = self.builder.mk_vector_quant_opt(QuantKind::Forall, value, ety, &mut |elem| {
                    let invs = self.translate_invariant_with_variant(deep, elem, parent_enum_variant.clone());
                    if !invs.is_empty() {
                        loc = invs[0].0.clone();
                    }
                    self.builder.mk_join_bool(ast::Operation::And, invs.into_iter().map(|(_, e)| e))
                });
                if let Some(e) = quant {
                    vec![(loc, e)]
                } else {
                    vec![]
                }
            },
            _ => vec![],
        }
    }

    fn translate_invariant_for_struct(
        &self,
        deep: bool,
        value: Exp,
        struct_env: StructEnv<'_>,
        targs: &[Type],
    ) -> Vec<(Loc, Exp)> {
        self.translate_invariant_for_struct_with_variant(deep, value, struct_env, targs, None)
    }

    /// New helper: propagate parent enum/variant context
    fn translate_invariant_for_struct_with_variant(
        &self,
        deep: bool,
        value: Exp,
        struct_env: StructEnv<'_>,
        targs: &[Type],
        parent_enum_variant: Option<(Type, Symbol, Exp)>,
    ) -> Vec<(Loc, Exp)> {
        use ast::Operation::*;
        use ExpData::*;

        let lang_ver_ge_2 = self
            .builder
            .global_env()
            .language_version()
            .is_at_least(LanguageVersion::V2_0);
        let mut result = vec![];
        for cond in struct_env
            .get_spec()
            .filter_kind(ConditionKind::StructInvariant)
        {
            let exp_rewriter = &mut |e: Exp| match e.as_ref() {
                LocalVar(_, var) => {
                    if lang_ver_ge_2
                        && var
                            .display(self.builder.global_env().symbol_pool())
                            .to_string()
                            == well_known::RECEIVER_PARAM_NAME
                    {
                        RewriteResult::Rewritten(value.clone())
                    } else {
                        RewriteResult::Unchanged(e)
                    }
                },
                Call(id, oper @ Select(..), args) if args.is_empty() => RewriteResult::Rewritten(
                    Call(*id, oper.to_owned(), vec![value.clone()]).into_exp(),
                ),
                _ => RewriteResult::Unchanged(e),
            };
            let env = self.builder.global_env();
            let node_rewriter = &mut |id: NodeId| ExpData::instantiate_node(env, id, targs);
            let exp = ExpData::rewrite_exp_and_node_id(cond.exp.clone(), exp_rewriter, node_rewriter);
            let mut guarded_exp = exp.clone();
            // If this is a field of an enum variant, guard with a TestVariant
            if let Some((enum_ty, variant_sym, enum_val)) = &parent_enum_variant {
                // Build: (TestVariant(enum_val, variant_sym) ==> exp)
                let test = self.mk_test_variant(enum_ty.clone(), enum_val.clone(), *variant_sym);
                guarded_exp = self.builder.mk_implies(test, exp);
            }
            result.push((cond.loc.clone(), guarded_exp));
        }
        if deep {
            if struct_env.has_variants() {
                let variant_context = self.builder.data.annotations.get::<VariantContextAnnotation>();
                if variant_context.is_none() {
                    return result;
                }
                let variant_annotation = variant_context.unwrap();
                let current_offset = self.builder.data.code.len() as u16;
                let temp_idx = match value.as_ref() {
                    ExpData::LocalVar(_, var) => {
                        let temp_index = var.display(self.builder.global_env().symbol_pool()).to_string();
                        temp_index.parse::<usize>().ok()
                    },
                    ExpData::Temporary(_, temp) => {
                        Some(*temp)
                    },
                    _ => None
                };
                if let Some(temp_idx) = temp_idx {
                    let struct_id = QualifiedInstId {
                        module_id: struct_env.module_env.get_id(),
                        id: struct_env.get_id(),
                        inst: targs.to_vec(),
                    };
                    let mut most_recent_context: Option<&VariantContext> = None;
                    let mut most_recent_offset = 0;
                    for (offset, context) in variant_annotation.get_all_contexts() {
                        if context.has_variant(&struct_id, temp_idx) && *offset <= current_offset {
                            if *offset > most_recent_offset {
                                most_recent_offset = *offset;
                                most_recent_context = Some(context);
                            }
                        }
                    }
                    if let Some(context) = most_recent_context {
                        if let Some(active_variant) = context.get_variant(&struct_id, temp_idx) {
                            for field_env in struct_env.get_fields_of_variant(active_variant) {
                                let field_exp = self
                                    .builder
                                    .mk_field_select(&field_env, targs, value.clone());
                                // Pass enum type, variant symbol, and enum value as parent context
                                let enum_ty = Type::Struct(struct_env.module_env.get_id(), struct_env.get_id(), targs.to_vec());
                                result.extend(self.translate_invariant_with_variant(deep, field_exp, Some((enum_ty, active_variant, value.clone()))));
                            }
                            return result;
                        }
                    }
                    return result;
                } else {
                    return result;
                }
            } else {
                for field_env in struct_env.get_fields() {
                    let field_exp = self
                        .builder
                        .mk_field_select(&field_env, targs, value.clone());
                    result.extend(self.translate_invariant_with_variant(deep, field_exp, parent_enum_variant.clone()));
                }
            }
        }
        result
    }

    /// Helper to build a TestVariant expression
    fn mk_test_variant(&self, enum_ty: Type, enum_val: Exp, variant: Symbol) -> Exp {
        use ast::Operation;
        use ExpData;
        // Extract module and struct id from enum_ty
        if let Type::Struct(mid, sid, _) = &enum_ty {
            let node_id = self.builder.global_env().new_node(self.builder.global_env().unknown_loc(), enum_ty.clone());
            ExpData::Call(
                node_id,
                Operation::TestVariants(*mid, *sid, vec![variant]),
                vec![enum_val],
            )
            .into_exp()
        } else {
            panic!("mk_test_variant called with non-enum type");
        }
    }
}
