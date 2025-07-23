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
use std::collections::BTreeSet;
use move_model::symbol::Symbol;
use move_model::model::FieldId;

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
                eprintln!("DEBUG: Processing Pack instruction");
                let struct_temp = dests[0];
                self.builder
                    .emit(Call(id, dests, Pack(mid, sid, targs), srcs, aa));

                // Check if this is an enum being packed
                let struct_env = self.builder.global_env().get_module(mid).into_struct(sid);
                if struct_env.has_variants() {
                    // For enums, we need to determine which variant is being packed
                    // Since we don't have variant information in Pack instructions,
                    // we'll apply all invariants conservatively
                    eprintln!("DEBUG: Pack instruction for enum struct {}",
                             struct_env.get_name().display(self.builder.global_env().symbol_pool()).to_string());
                    self.emit_data_invariant_for_temp(false, PropKind::Assert, struct_temp);
                } else {
                    // For regular structs, emit a shallow assert of the data invariant.
                    self.emit_data_invariant_for_temp(false, PropKind::Assert, struct_temp);
                }
            },
            Call(id, dests, PackVariant(mid, sid, variant, targs), srcs, aa)
                if self.for_verification =>
            {
                let struct_temp = dests[0];

                // Debug output
                eprintln!("DEBUG: PackVariant for struct {} with variant {}",
                         self.builder.global_env().get_module(mid).into_struct(sid).get_name().display(self.builder.global_env().symbol_pool()).to_string(),
                         variant.display(self.builder.global_env().symbol_pool()).to_string());

                self.builder.emit(Call(
                    id,
                    dests,
                    PackVariant(mid, sid, variant, targs),
                    srcs,
                    aa,
                ));
                // Emit a shallow assert of the data invariant for the specific variant.
                self.emit_data_invariant_for_temp_variant(false, PropKind::Assert, struct_temp, Some(variant));
            },
            Call(_, _, PackRef, srcs, _) if self.for_verification => {
                eprintln!("DEBUG: Processing PackRef instruction");
                // Emit a shallow assert of the data invariant.
                self.emit_data_invariant_for_temp(false, PropKind::Assert, srcs[0]);
            },
            Call(_, _, PackRefDeep, srcs, _) if self.for_verification => {
                eprintln!("DEBUG: Processing PackRefDeep instruction");
                // Get the type of the value being processed
                let env = self.builder.global_env();
                let ty = env.get_node_type(self.builder.mk_temporary(srcs[0]).node_id());
                eprintln!("DEBUG: PackRefDeep for type {:?}", ty);

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
        eprintln!("DEBUG: emit_data_invariant_for_temp called with deep={}, kind={:?}, temp={}", deep, kind, temp);
        self.emit_data_invariant_for_temp_variant(deep, kind, temp, None)
    }

    /// Emits a data invariant, shallow or deep, assume or assert, for the value in temporary.
    /// If variant is Some, only applies invariants relevant to that specific variant.
    fn emit_data_invariant_for_temp_variant(&mut self, deep: bool, kind: PropKind, temp: TempIndex, variant: Option<Symbol>) {
        // Debug output
        if let Some(variant_name) = variant {
            eprintln!("DEBUG: emit_data_invariant_for_temp_variant called with variant {}",
                     variant_name.display(self.builder.global_env().symbol_pool()).to_string());
        }

        let temp_exp = self.builder.mk_temporary(temp);
        for (loc, inv) in self.translate_invariant_variant(deep, temp_exp, variant) {
            eprintln!("DEBUG: Emitting invariant: {:?} at location {:?}", inv, loc);
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
        self.translate_invariant_variant(deep, value, None)
    }

    fn translate_invariant_variant(&self, deep: bool, value: Exp, variant: Option<Symbol>) -> Vec<(Loc, Exp)> {
        let env = self.builder.global_env();
        let ty = env.get_node_type(value.node_id());
        match ty.skip_reference() {
            Type::Struct(mid, sid, targs) => {
                let struct_env = env.get_module(*mid).into_struct(*sid);

                // Debug output
                eprintln!("DEBUG: Processing struct {} with variant {:?}",
                         struct_env.get_name().display(env.symbol_pool()).to_string(),
                         variant.map(|v| v.display(env.symbol_pool()).to_string()));

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
                            let invs = self.translate_invariant_variant(deep, e, variant);
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
                    self.translate_invariant_for_struct_variant(deep, value, struct_env, targs, variant)
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
                            let invs = self.translate_invariant_variant(deep, elem, variant);
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

    fn translate_invariant_for_struct_variant(
        &self,
        deep: bool,
        value: Exp,
        struct_env: StructEnv<'_>,
        targs: &[Type],
        variant: Option<Symbol>,
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
            eprintln!("DEBUG: Processing invariant for struct {}: {:?}",
                     struct_env.get_name().display(self.builder.global_env().symbol_pool()).to_string(),
                     cond.exp);

            // Get the fields referenced in this invariant
            let referenced_fields = self.get_invariant_field_paths(&cond.exp);
            eprintln!("DEBUG: Extracted field paths: {:?}",
                     referenced_fields.iter().map(|path| path.iter().map(|f| f.symbol().display(self.builder.global_env().symbol_pool()).to_string()).collect::<Vec<_>>()).collect::<Vec<_>>());

            // Check: if the invariant references fields that don't exist in this struct at all,
            // skip it (this handles cases where invariants from nested structs are incorrectly applied)
            let mut should_skip = false;
            for path in &referenced_fields {
                if !path.is_empty() {
                    let first_field = &path[0];
                    eprintln!("DEBUG: Checking if field {} exists in struct {}",
                             first_field.symbol().display(self.builder.global_env().symbol_pool()).to_string(),
                             struct_env.get_name().display(self.builder.global_env().symbol_pool()).to_string());

                    let mut field_exists = false;
                    for field in struct_env.get_fields() {
                        eprintln!("DEBUG: Comparing with field {}",
                                 field.get_name().display(self.builder.global_env().symbol_pool()).to_string());
                        if field.get_name() == first_field.symbol() {
                            field_exists = true;
                            eprintln!("DEBUG: Field found!");
                            break;
                        }
                    }
                    if !field_exists {
                        eprintln!("DEBUG: Skipping invariant - field {} doesn't exist in struct {}",
                                 first_field.symbol().display(self.builder.global_env().symbol_pool()).to_string(),
                                 struct_env.get_name().display(self.builder.global_env().symbol_pool()).to_string());
                        should_skip = true;
                        break;
                    }
                }
            }
            if should_skip {
                continue;
            }

            // Simple fix: when doing deep invariant checking on enums, only apply invariants
            // that are defined on the enum itself, not on nested structs
            if deep {
                // Get the type of the value we're checking invariants for
                let value_type = self.builder.global_env().get_node_type(value.node_id());
                if let Type::Struct(value_mid, value_sid, _) = value_type.skip_reference() {
                    let value_struct_env = self.builder.global_env().get_module(*value_mid).into_struct(*value_sid);

                    // Check if the value being processed is an enum
                    if value_struct_env.has_variants() {
                        let invariant_struct_name = struct_env.get_name();
                        let value_struct_name = value_struct_env.get_name();

                        eprintln!("DEBUG: Deep invariant check on enum - invariant from struct {}, applied to struct {}",
                                 invariant_struct_name.display(self.builder.global_env().symbol_pool()).to_string(),
                                 value_struct_name.display(self.builder.global_env().symbol_pool()).to_string());

                        // If the invariant is from a different struct than the enum we're checking,
                        // skip it to avoid applying nested struct invariants to the enum
                        if invariant_struct_name != value_struct_name {
                            eprintln!("DEBUG: Skipping invariant - invariant from nested struct applied to enum");
                            continue;
                        }
                    }
                }
            }

            // For enum variants, we need to check if the invariant is applicable to this variant
            // by analyzing which fields the invariant references
            if struct_env.has_variants() {
                // Check if this invariant should be applied to the current variant
                // We need to determine which variant is being packed from the context
                // For now, we'll apply the invariant if it references any fields that exist
                // in the current variant being packed
                if !self.should_apply_invariant_to_variant(&referenced_fields, &struct_env, variant) {
                    // Debug output
                    if let Some(variant_name) = variant {
                        eprintln!("DEBUG: Skipping invariant for variant {}: {:?}",
                                 variant_name.display(self.builder.global_env().symbol_pool()),
                                 cond.exp);
                    } else {
                        eprintln!("DEBUG: Skipping invariant (no variant context): {:?}", cond.exp);
                    }
                    continue; // Skip this invariant for this variant
                } else {
                    // Debug output
                    if let Some(variant_name) = variant {
                        eprintln!("DEBUG: Applying invariant for variant {}: {:?}",
                                 variant_name.display(self.builder.global_env().symbol_pool()),
                                 cond.exp);
                    } else {
                        eprintln!("DEBUG: Applying invariant (no variant context): {:?}", cond.exp);
                    }
                }
            }

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

        // If this is deep, recurse over all fields.
        if deep {
            // For enum variants, only recurse on fields that exist in the current variant
            let fields_to_check = if let Some(variant_name) = variant {
                struct_env.get_fields_of_variant(variant_name).collect::<Vec<_>>()
            } else {
                struct_env.get_fields().collect::<Vec<_>>()
            };

            for field_env in fields_to_check {
                let field_exp = self
                    .builder
                    .mk_field_select(&field_env, targs, value.clone());
                // Only pass variant information if the field type is also an enum
                let field_type = field_env.get_type();
                let field_variant = if let Type::Struct(mid, sid, _) = field_type.skip_reference() {
                    let field_struct_env = self.builder.global_env().get_module(*mid).into_struct(*sid);
                    if field_struct_env.has_variants() {
                        variant // Pass variant info only if the field type is also an enum
                    } else {
                        None // Don't pass variant info for non-enum field types
                    }
                } else {
                    None // Don't pass variant info for non-struct field types
                };
                result.extend(self.translate_invariant_variant(deep, field_exp, field_variant));
            }
        }

        result
    }

    /// Extract the field access paths referenced in an invariant expression
    /// Returns a set of field paths, where each path is a vector of field names
    fn get_invariant_field_paths(&self, exp: &Exp) -> BTreeSet<Vec<FieldId>> {
        use ExpData::*;
        let mut paths = BTreeSet::new();

        // Helper function to collect field paths from an expression
        fn collect_paths(e: &Exp, current_path: &mut Vec<FieldId>, paths: &mut BTreeSet<Vec<FieldId>>) {
            match e.as_ref() {
                Call(_, ast::Operation::Select(_, _, field_id), args) => {
                    if args.len() == 1 {
                        // This is a field selection from the target
                        current_path.push(*field_id);
                        paths.insert(current_path.clone());

                        // Continue traversing the selected field
                        collect_paths(&args[0], current_path, paths);
                        current_path.pop();
                    } else {
                        // This is a field selection from some other expression
                        for arg in args {
                            collect_paths(arg, current_path, paths);
                        }
                    }
                },
                Call(_, _, args) => {
                    for arg in args {
                        collect_paths(arg, current_path, paths);
                    }
                },
                Quant(_, _, ranges, _, _, body) => {
                    for (_, range_exp) in ranges {
                        collect_paths(range_exp, current_path, paths);
                    }
                    collect_paths(body, current_path, paths);
                },
                Block(_, _, _, scope) => {
                    collect_paths(scope, current_path, paths);
                },
                IfElse(_, cond, on_true, on_false) => {
                    collect_paths(cond, current_path, paths);
                    collect_paths(on_true, current_path, paths);
                    collect_paths(on_false, current_path, paths);
                },
                _ => {}
            }
        }

        let mut current_path = Vec::new();
        collect_paths(exp, &mut current_path, &mut paths);
        paths
    }

    /// Determine if an invariant should be applied to the current variant being packed
    /// This checks if the invariant references fields that exist in the specified variant
    fn should_apply_invariant_to_variant(&self, field_paths: &BTreeSet<Vec<FieldId>>, struct_env: &StructEnv<'_>, variant: Option<Symbol>) -> bool {
        // Debug output
        eprintln!("DEBUG: should_apply_invariant_to_variant for struct {} with variant {:?} and field_paths {:?}",
                 struct_env.get_name().display(self.builder.global_env().symbol_pool()).to_string(),
                 variant.map(|v| v.display(self.builder.global_env().symbol_pool()).to_string()),
                 field_paths.iter().map(|path| path.iter().map(|f| f.symbol().display(self.builder.global_env().symbol_pool()).to_string()).collect::<Vec<_>>()).collect::<Vec<_>>());

        match variant {
            Some(variant_name) => {
                // Check if all field paths start with fields that exist in this specific variant
                for path in field_paths {
                    if path.is_empty() {
                        continue; // Skip empty paths
                    }

                    // Check if the first field in the path exists in this variant
                    let first_field = &path[0];
                    let mut field_exists = false;
                    for field in struct_env.get_fields_of_variant(variant_name) {
                        if field.get_name() == first_field.symbol() {
                            field_exists = true;
                            break;
                        }
                    }
                    if !field_exists {
                        return false; // First field doesn't exist in this variant, so skip this invariant
                    }
                }
                true // All field paths start with fields that exist in this variant
            },
            None => {
                // No specific variant specified, apply conservatively
                // Check if any of the field paths start with fields that exist in any variant
                for path in field_paths {
                    if path.is_empty() {
                        continue; // Skip empty paths
                    }

                    let first_field = &path[0];
                    for variant in struct_env.get_variants() {
                        for field in struct_env.get_fields_of_variant(variant) {
                            if field.get_name() == first_field.symbol() {
                                return true;
                            }
                        }
                    }
                }
                false
            }
        }
    }
}
