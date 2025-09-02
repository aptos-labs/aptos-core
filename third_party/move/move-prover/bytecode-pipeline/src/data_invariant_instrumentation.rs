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
use move_model::{
    ast,
    ast::{ConditionKind, Exp, ExpData, QuantKind, RewriteResult, TempIndex},
    exp_generator::ExpGenerator,
    metadata::LanguageVersion,
    model::{FunctionEnv, Loc, NodeId, StructEnv},
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
            Call(_, _, PackRefDeep, srcs, _) if self.for_verification => {
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

                    // When dealing with a map, we cannot maintain individual locations for
                    // invariants. Instead we choose just one as a representative.
                    // TODO(refactoring): we should use the spec block position instead.
                    let mut loc = env.unknown_loc();
                    let quant = self.builder.mk_map_quant_opt(
                        QuantKind::Forall,
                        value,
                        spec_fun_get,
                        &targs[0],
                        &targs[1],
                        &mut |e| {
                            let invs = self.translate_invariant(deep, e);
                            if !invs.is_empty() {
                                loc = invs[0].0.clone();
                            }
                            self.builder
                                .mk_join_bool(ast::Operation::And, invs.into_iter().map(|(_, e)| e))
                        },
                    );
                    if let Some(e) = quant {
                        vec![(loc, e)]
                    } else {
                        vec![]
                    }
                } else {
                    self.translate_invariant_for_struct(deep, value, struct_env, targs)
                }
            },
            Type::Vector(ety) => {
                // When dealing with a vector, we cannot maintain individual locations for
                // invariants. Instead we choose just one as a representative.
                // TODO(refactoring): we should use the spec block position instead.
                let mut loc = env.unknown_loc();
                let quant =
                    self.builder
                        .mk_vector_quant_opt(QuantKind::Forall, value, ety, &mut |elem| {
                            let invs = self.translate_invariant(deep, elem);
                            if !invs.is_empty() {
                                loc = invs[0].0.clone();
                            }
                            self.builder
                                .mk_join_bool(ast::Operation::And, invs.into_iter().map(|(_, e)| e))
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
        use ast::Operation::*;
        use ExpData::*;

        let lang_ver_ge_2 = self
            .builder
            .global_env()
            .language_version()
            .is_at_least(LanguageVersion::V2_0);
        // First generate a conjunction for all invariants on this struct.
        let mut result = vec![];
        for cond in struct_env
            .get_spec()
            .filter_kind(ConditionKind::StructInvariant)
        {
            // Rewrite the invariant expression, inserting `value` for the struct target.
            // By convention, selection from the target is represented as a `Select` operation with
            // an empty argument list. It is guaranteed that this uniquely identifies the
            // target, as any other `Select` will have exactly one argument.
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
            // Also instantiate types.
            let env = self.builder.global_env();
            let node_rewriter = &mut |id: NodeId| ExpData::instantiate_node(env, id, targs);

            let exp =
                ExpData::rewrite_exp_and_node_id(cond.exp.clone(), exp_rewriter, node_rewriter);
            result.push((cond.loc.clone(), exp));
        }

        // If this is deep, recurse over all fields. For enum variants, need to guard the
        // field invariants with an appropriate test.
        if deep {
            if struct_env.has_variants() {
                for variant in struct_env.get_variants() {
                    let is_variant =
                        self.builder
                            .mk_variant_test(&struct_env, variant, value.clone());
                    for field_env in struct_env.get_fields_of_variant(variant) {
                        let field_exp =
                            self.builder
                                .mk_field_select(&field_env, targs, value.clone());
                        result.extend(self.translate_invariant(deep, field_exp).iter().map(
                            |inv| {
                                (
                                    inv.0.clone(),
                                    self.builder.mk_implies(is_variant.clone(), inv.1.clone()),
                                )
                            },
                        ));
                    }
                }
            } else {
                for field_env in struct_env.get_fields() {
                    let field_exp = self
                        .builder
                        .mk_field_select(&field_env, targs, value.clone());
                    result.extend(self.translate_invariant(deep, field_exp));
                }
            }
        }
        result
    }
}
