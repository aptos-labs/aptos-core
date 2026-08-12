// Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Rendering analysis: joins the mono type universe with the number-operation
//! classification into the bitvector "twin" renderings the Boogie encoding
//! declares (supply) and the renderings the emitted code references (demand).
//! See `doc/dev/bv_rendering.md`.
//!
//! Twins reconcile value-level bitvector classification with Boogie's
//! type-level rendering: `Vec int`/`Vec (bv64)` and the int/bv table twins
//! coexist so each slot can pick its rendering. Supply is currently
//! speculative (every instantiation passing a type filter gets a twin);
//! demand derives from `Bitwise` classification slots. A demanded twin
//! missing from supply surfaces as an undeclared type/function error out of
//! Boogie, far from its cause — the check here reports the gap at
//! generation time instead.

use crate::{
    boogie_helpers::{boogie_inst_suffix, boogie_type_suffix, bv_flag_for_type},
    options::BoogieOptions,
};
use move_model::{
    model::{GlobalEnv, QualifiedId, StructId},
    pragmas::INTRINSIC_TYPE_MAP,
    symbol::Symbol,
    ty::Type,
};
use move_prover_bytecode_pipeline::{
    mono_analysis::{self, MonoInfo},
    number_operation::GlobalNumberOperationState,
};
use move_stackless_bytecode::function_target_pipeline::{FunctionTargetsHolder, FunctionVariant};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

/// Environment variable enabling the twin-universe check and the demand
/// report. Off by default: firing diagnostics on green builds would churn
/// test baselines while the demand walker's coverage is still being
/// validated against the emission sites (see the coverage matrix in the
/// design doc).
const RENDERING_CHECK_ENV_VAR: &str = "MVP_BV_RENDERING_CHECK";

/// Vector element types receiving a speculative bv twin in the prelude:
/// every vector instantiation whose element closure is free of signed
/// integers and widthless `num` (neither has a bv rendering).
pub fn vec_twin_supply(env: &GlobalEnv, mono_info: &MonoInfo) -> Vec<Type> {
    mono_info
        .vec_inst
        .iter()
        .filter(|ty| {
            !crate::boogie_helpers::type_contains_signed_int(env, ty)
                && !crate::boogie_helpers::type_contains_widthless_num(env, ty)
        })
        .cloned()
        .collect()
}

/// Map instances receiving a speculative bv value twin in the prelude:
/// per instantiation, exactly those whose value type has a legal bv
/// rendering that differs from the plain one (nested unsigned values like
/// `vector<u8>` included; struct/bool values render identically and would
/// duplicate the base instance). Only non-empty per-map subsets are
/// returned.
pub fn table_twin_supply(
    env: &GlobalEnv,
    mono_info: &MonoInfo,
) -> Vec<(QualifiedId<StructId>, BTreeSet<(Type, Type)>)> {
    mono_info
        .table_inst
        .iter()
        .filter_map(|(qid, ty_args)| {
            let bv_ty_args = ty_args
                .iter()
                .filter(|(_, vty)| {
                    let vty = vty.skip_reference();
                    !crate::boogie_helpers::type_contains_signed_int(env, vty)
                        && !crate::boogie_helpers::type_contains_widthless_num(env, vty)
                        && boogie_type_suffix(env, vty, true)
                            != boogie_type_suffix(env, vty, false)
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            (!bv_ty_args.is_empty()).then_some((*qid, bv_ty_args))
        })
        .collect()
}

/// Twin renderings demanded by `Bitwise` classification slots, per type
/// family. Structs have no twin supply today (one datatype per
/// instantiation); their demand is recorded as input for the struct-twin
/// design stage.
#[derive(Default)]
pub struct TwinDemand {
    pub vec: BTreeSet<Type>,
    pub table: BTreeMap<QualifiedId<StructId>, BTreeSet<(Type, Type)>>,
    pub structs: BTreeMap<QualifiedId<StructId>, BTreeSet<StructRendering>>,
    /// The stage-3 struct twin supply: demanded generic-class identities
    /// with their per-argument flag projection (the twin's name suffix
    /// renders argument `i` in bv when flagged). `None` marks identities
    /// whose rendering cannot be projected onto arguments (a bv field whose
    /// declared type is not a bare type parameter) — reported, not twinned.
    pub struct_twin_projection: BTreeMap<(QualifiedId<StructId>, Vec<Type>), Option<Vec<bool>>>,
}

/// A demanded rendering of one struct instantiation. Classification is
/// per-field, so the rendering unit is the vector of per-field bv flags —
/// positional mixes of the same concrete integer type are distinct
/// renderings (`x: bv64, y: int` vs `x: int, y: bv64`). Field slots are
/// program-global, so per (struct, instantiation) at most one non-plain
/// rendering exists per run; the plain rendering always exists (it is what
/// templates and plain components construct).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StructRendering {
    pub inst: Vec<Type>,
    /// Per-field bv flag, in declaration order (variant fields follow their
    /// variant order), derived from `(field slot oper, instantiated field
    /// type)` — the same derivation the datatype declaration uses.
    pub field_flags: Vec<(Symbol, bool)>,
}

impl TwinDemand {
    /// Register the bv rendering of `ty` (already instantiated, already
    /// known to render bv per `bv_flag_for_type`): record every twin the
    /// rendered type references, mirroring the recursion of `boogie_type`.
    /// A twin is demanded only where the bv rendering differs from the int
    /// rendering (otherwise the base declaration serves both).
    fn register(&mut self, env: &GlobalEnv, ty: &Type) {
        let ty = ty.skip_reference();
        match ty {
            Type::Vector(et) => {
                if boogie_type_suffix(env, et, true) != boogie_type_suffix(env, et, false) {
                    self.vec.insert((**et).clone());
                    self.register(env, et);
                }
            },
            Type::Struct(mid, sid, inst) => {
                let struct_env = env.get_module(*mid).into_struct(*sid);
                if struct_env.is_intrinsic_of(INTRINSIC_TYPE_MAP) {
                    let (kty, vty) = (&inst[0], &inst[1]);
                    if boogie_type_suffix(env, vty, true) != boogie_type_suffix(env, vty, false) {
                        self.table
                            .entry(struct_env.get_qualified_id())
                            .or_default()
                            .insert((kty.clone(), vty.clone()));
                        self.register(env, vty);
                    }
                }
                // Non-intrinsic structs: nothing to register from a value
                // slot. A struct value's rendering is its declaration's
                // slot-derived rendering — struct demand is computed once
                // per (struct, instantiation) from the field slots (see
                // `compute_demand`), never from the value's own slot.
            },
            Type::Tuple(ts) => {
                // Tuple elements render with the tuple's flag.
                for t in ts {
                    self.register(env, t);
                }
            },
            _ => {},
        }
    }
}

/// Compute the twin demand from the classification slots of every function
/// instance in the mono universe: temporaries (parameters included) and
/// return positions, per function variant, instantiated with the instance's
/// type arguments. This mirrors what the translator derives when it renders
/// each slot. Coverage grows toward the design doc's matrix (spec nodes,
/// memory, closures) as the check is validated against emission.
pub fn compute_demand(
    env: &GlobalEnv,
    mono_info: &MonoInfo,
    targets: &FunctionTargetsHolder,
) -> TwinDemand {
    let mut demand = TwinDemand::default();
    let global_state = match env.get_extension::<GlobalNumberOperationState>() {
        Some(state) => state,
        None => return demand,
    };
    for ((fun_qid, variant), insts) in &mono_info.funs {
        let fun_env = env.get_function(*fun_qid);
        if !targets.get_target_variants(&fun_env).contains(variant) {
            continue;
        }
        let target = targets.get_target(&fun_env, variant);
        let baseline_flag = *variant == FunctionVariant::Baseline;
        let mut register_slot = |ty: &Type, oper| {
            for inst in insts {
                let ty = ty.instantiate(inst);
                if bv_flag_for_type(env, oper, ty.skip_reference()) {
                    demand.register(env, &ty);
                }
            }
        };
        for idx in 0..target.get_local_count() {
            if let Some(oper) =
                global_state.get_temp_index_oper(fun_qid.module_id, fun_qid.id, idx, baseline_flag)
            {
                register_slot(target.get_local_type(idx), oper);
            }
        }
        // Trace instructions declare `$temp_N'<suffix>'` per traced
        // rendering, but they reference locals, so the temp slots above
        // already cover them.
        if let Some(ret_opers) = global_state
            .get_ret_map()
            .get(&(fun_qid.module_id, fun_qid.id))
        {
            for (idx, oper) in ret_opers {
                if *idx < fun_env.get_return_count() {
                    register_slot(&fun_env.get_result_type_at(*idx), oper);
                }
            }
        }
    }
    // Spec function parameter/return slots, joined with the spec-fun mono
    // instances: a `Bitwise` slot renders its (instantiated) type's twins
    // in the emitted spec function signature and body.
    for (qid, insts) in &mono_info.spec_funs {
        let Some((param_opers, ret_opers)) = global_state
            .spec_fun_operation_map
            .get(&(qid.module_id, qid.id))
        else {
            continue;
        };
        let module_env = env.get_module(qid.module_id);
        let decl = module_env.get_spec_fun(qid.id);
        for inst in insts {
            for (i, oper) in param_opers.iter().enumerate() {
                if let Some(param) = decl.params.get(i) {
                    let ty = param.1.instantiate(inst);
                    if bv_flag_for_type(env, oper, ty.skip_reference()) {
                        demand.register(env, &ty);
                    }
                }
            }
            for oper in ret_opers {
                let ty = decl.result_type.instantiate(inst);
                if bv_flag_for_type(env, oper, ty.skip_reference()) {
                    demand.register(env, &ty);
                }
            }
        }
    }
    // Spec expression nodes. Node classifications are shared across the
    // contexts a spec expression is instantiated into; closed node types
    // register directly. Open node types depend on the enclosing context's
    // instantiation — those contexts' own slots (params, temps, spec-fun
    // slots) cover the instantiated demand.
    for (node_id, oper) in &global_state.exp_operation_map {
        if let Some(ty) = env.get_node_type_opt(*node_id) {
            let ty = ty.skip_reference();
            if !ty.is_open() && bv_flag_for_type(env, oper, ty) {
                demand.register(env, ty);
            }
        }
    }
    // Struct field slots: a `Bitwise` field renders as its bv twin inside
    // the (single) datatype declaration of every instantiation, so the
    // declaration itself references twins (`Vec (bv8)`, map value twins) —
    // and the instantiation is a struct-twin candidate: its declaration is
    // rendering-dependent, which the struct-twin stage must make explicit.
    // Field slots are the poisoning channel: plain-struct temporaries do
    // not classify `Bitwise` themselves. The demanded identity is the full
    // per-field rendering vector — a field whose bv rendering equals its
    // int rendering contributes `false` (its declaration is
    // rendering-independent even under a `Bitwise` slot).
    for (struct_qid, insts) in &mono_info.structs {
        let struct_env = env.get_struct(*struct_qid);
        if struct_env.is_intrinsic() {
            continue;
        }
        let all_fields = if struct_env.has_variants() {
            struct_env
                .get_variants()
                .collect::<Vec<_>>()
                .into_iter()
                .flat_map(|v| struct_env.get_fields_of_variant(v))
                .collect::<Vec<_>>()
        } else {
            struct_env.get_fields().collect::<Vec<_>>()
        };
        for inst in insts {
            let mut field_flags = Vec::new();
            // Per-argument projection of the field flags, for the twin's
            // name suffix. Faithful only when every bv field's declared
            // type is a bare type parameter; `None` otherwise.
            let mut arg_flags = Some(vec![false; inst.len()]);
            for field_env in &all_fields {
                let field_ty = field_env.get_type().instantiate(inst);
                let flag = crate::boogie_helpers::field_bv_flag_global_state(
                    &global_state,
                    field_env,
                    env,
                    &field_ty,
                ) && boogie_type_suffix(env, &field_ty, true)
                    != boogie_type_suffix(env, &field_ty, false);
                if flag {
                    // The declaration references the field type's bv
                    // rendering: register the twins it mentions.
                    demand.register(env, &field_ty);
                    match (field_env.get_type(), &mut arg_flags) {
                        (Type::TypeParameter(idx), Some(flags)) => {
                            flags[idx as usize] = true;
                        },
                        _ => arg_flags = None,
                    }
                }
                field_flags.push((field_env.get_name(), flag));
            }
            if field_flags.iter().any(|(_, flag)| *flag) {
                if !inst.is_empty() {
                    demand
                        .struct_twin_projection
                        .insert((*struct_qid, inst.clone()), arg_flags);
                }
                demand
                    .structs
                    .entry(*struct_qid)
                    .or_default()
                    .insert(StructRendering {
                        inst: inst.clone(),
                        field_flags,
                    });
            }
        }
    }
    demand
}

/// Records twin references derived during translation. Armed (as an env
/// extension) only between prelude emission and the end of translation, and
/// only when the check is enabled — the prelude's own speculative supply
/// emission must not be recorded. The records feed two generation-time
/// assertions: every reference must be in supply (else Boogie reports an
/// undeclared name, three layers from the cause) and in the walker's demand
/// (else the walker's coverage matrix has a gap).
#[derive(Default)]
pub struct TwinRefRecorder {
    vec: Mutex<BTreeSet<Type>>,
    table: Mutex<BTreeSet<(QualifiedId<StructId>, Type, Type)>>,
}

pub(crate) fn record_vec_twin_ref(env: &GlobalEnv, et: &Type) {
    if let Some(rec) = env.get_extension::<TwinRefRecorder>() {
        rec.vec.lock().unwrap().insert(et.clone());
    }
}

pub(crate) fn record_table_twin_ref(env: &GlobalEnv, qid: QualifiedId<StructId>, inst: &[Type]) {
    if let Some(rec) = env.get_extension::<TwinRefRecorder>() {
        rec.table
            .lock()
            .unwrap()
            .insert((qid, inst[0].clone(), inst[1].clone()));
    }
}

/// Rendering info installed as an env extension by the driver before
/// generation: the demand computed by the walker, and the stage-3 struct
/// twin supply (projectable generic-class identities) that name-rendering
/// sites consult.
pub struct RenderingInfo {
    pub demand: TwinDemand,
    pub struct_twins: BTreeMap<(QualifiedId<StructId>, Vec<Type>), Vec<bool>>,
}

/// The per-argument twin flags for a dragged struct instantiation, if this
/// (struct, instantiation) has a projectable twin rendering. Consulted
/// unconditionally by the non-intrinsic struct naming branch: a dragged
/// instantiation's declaration and every reference to it render the twin
/// suffix (struct-typed slots never classify `Bitwise`, so the naming
/// helpers' `bv_flag` parameter cannot carry this information).
pub fn struct_twin_flags(
    env: &GlobalEnv,
    qid: QualifiedId<StructId>,
    inst: &[Type],
) -> Option<Vec<bool>> {
    if PLAIN_MODE.with(|m| m.get()) {
        return None;
    }
    let info = env.get_extension::<RenderingInfo>()?;
    info.struct_twins.get(&(qid, inst.to_vec())).cloned()
}

thread_local! {
    static PLAIN_MODE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Scoped plain-rendering mode: while a guard is alive, the twin lookup is
/// suppressed and field flags render plainly, so everything emitted —
/// including nested struct references — belongs to the plain world. Used to
/// emit the plain datatype of a dragged instantiation (what templates and
/// undragged worlds construct).
pub struct PlainRenderingGuard;

impl PlainRenderingGuard {
    pub fn new() -> Self {
        PLAIN_MODE.with(|m| m.set(true));
        PlainRenderingGuard
    }
}

impl Default for PlainRenderingGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PlainRenderingGuard {
    fn drop(&mut self) {
        PLAIN_MODE.with(|m| m.set(false));
    }
}

pub fn plain_rendering_active() -> bool {
    PLAIN_MODE.with(|m| m.get())
}

/// Whether a spec function belongs to the plain rendering world: no
/// `Bitwise` slot anywhere in its signature. Its declaration and its call
/// sites render types and names under the plain guard when true.
pub fn spec_fun_plain_world(
    env: &GlobalEnv,
    mid: move_model::model::ModuleId,
    fid: move_model::model::SpecFunId,
) -> bool {
    use move_prover_bytecode_pipeline::number_operation::NumOperation;
    env.get_extension::<GlobalNumberOperationState>()
        .is_none_or(|state| {
            state
                .spec_fun_operation_map
                .get(&(mid, fid))
                .is_none_or(|(params, rets)| {
                    params
                        .iter()
                        .chain(rets.iter())
                        .all(|oper| *oper != NumOperation::Bitwise)
                })
        })
}

pub fn install_rendering_info(env: &GlobalEnv, targets: &FunctionTargetsHolder) {
    let mono_info = mono_analysis::get_info(env);
    let demand = compute_demand(env, &mono_info, targets);
    let struct_twins = demand
        .struct_twin_projection
        .iter()
        .filter_map(|(key, flags)| flags.clone().map(|f| (key.clone(), f)))
        .collect();
    env.set_extension(RenderingInfo {
        demand,
        struct_twins,
    });
}

/// The twin-universe check, enabled by `MVP_BV_RENDERING_CHECK`:
/// - `start` computes demand and supply, reports every
///   demanded-but-unsupplied vec/table twin as an error and dumps the
///   struct twin demand — the renderings the struct-twin stage must be able
///   to declare — to stderr;
/// - `arm` (after the prelude) installs the emission recorder;
/// - `finish` (after translation) checks every recorded twin reference
///   against supply (error) and against the walker's demand (coverage-gap
///   report).
pub struct TwinUniverseCheck {
    info: std::rc::Rc<RenderingInfo>,
    vec_supply: BTreeSet<Type>,
    table_supply: BTreeMap<QualifiedId<StructId>, BTreeSet<(Type, Type)>>,
}

impl TwinUniverseCheck {
    pub fn start(
        env: &GlobalEnv,
        options: &BoogieOptions,
        targets: &FunctionTargetsHolder,
    ) -> Option<Self> {
        if std::env::var(RENDERING_CHECK_ENV_VAR).is_err() {
            return None;
        }
        if options.use_cvc5 {
            // No twins exist on the cvc5 path.
            return None;
        }
        let mono_info = mono_analysis::get_info(env);
        if env.get_extension::<RenderingInfo>().is_none() {
            install_rendering_info(env, targets);
        }
        let info = env
            .get_extension::<RenderingInfo>()
            .expect("rendering info installed");
        let demand = &info.demand;

        let vec_supply: BTreeSet<Type> = vec_twin_supply(env, &mono_info).into_iter().collect();
        for ty in demand.vec.difference(&vec_supply) {
            env.error(
                &env.unknown_loc(),
                &format!(
                    "[rendering] demanded vector bv twin has no supply: Vec ({})",
                    boogie_type_suffix(env, ty, true)
                ),
            );
        }
        let table_supply: BTreeMap<QualifiedId<StructId>, BTreeSet<(Type, Type)>> =
            table_twin_supply(env, &mono_info).into_iter().collect();
        for (qid, insts) in &demand.table {
            let supplied = table_supply.get(qid);
            for (kty, vty) in insts {
                if !supplied.is_some_and(|s| s.contains(&(kty.clone(), vty.clone()))) {
                    env.error(
                        &env.unknown_loc(),
                        &format!(
                            "[rendering] demanded map bv twin has no supply: {}'{}_{}'",
                            env.get_struct(*qid).get_full_name_str(),
                            boogie_type_suffix(env, kty, false),
                            boogie_type_suffix(env, vty, true),
                        ),
                    );
                }
            }
        }
        Self::report_struct_demand(env, demand);
        Some(TwinUniverseCheck {
            info,
            vec_supply,
            table_supply,
        })
    }

    pub fn arm(&self, env: &GlobalEnv) {
        env.set_extension(TwinRefRecorder::default());
    }

    pub fn finish(self, env: &GlobalEnv) {
        let Some(recorder) = env.get_extension::<TwinRefRecorder>() else {
            return;
        };
        for et in recorder.vec.lock().unwrap().iter() {
            if boogie_type_suffix(env, et, true) == boogie_type_suffix(env, et, false) {
                // The bv rendering coincides with the plain one; the base
                // declaration serves the reference.
                continue;
            }
            if !self.vec_supply.contains(et) {
                env.error(
                    &env.unknown_loc(),
                    &format!(
                        "[rendering] emission referenced unsupplied vector bv twin: Vec ({})",
                        boogie_type_suffix(env, et, true)
                    ),
                );
            } else if !self.info.demand.vec.contains(et) {
                eprintln!(
                    "[rendering] walker coverage gap: emission referenced vec twin '{}' \
                     not in computed demand",
                    boogie_type_suffix(env, et, true)
                );
            }
        }
        for (qid, kty, vty) in recorder.table.lock().unwrap().iter() {
            if boogie_type_suffix(env, vty, true) == boogie_type_suffix(env, vty, false) {
                continue;
            }
            let name = || {
                format!(
                    "{}'{}_{}'",
                    env.get_struct(*qid).get_full_name_str(),
                    boogie_type_suffix(env, kty, false),
                    boogie_type_suffix(env, vty, true)
                )
            };
            if !self
                .table_supply
                .get(qid)
                .is_some_and(|s| s.contains(&(kty.clone(), vty.clone())))
            {
                env.error(
                    &env.unknown_loc(),
                    &format!(
                        "[rendering] emission referenced unsupplied map bv twin: {}",
                        name()
                    ),
                );
            } else if !self
                .info
                .demand
                .table
                .get(qid)
                .is_some_and(|s| s.contains(&(kty.clone(), vty.clone())))
            {
                eprintln!(
                    "[rendering] walker coverage gap: emission referenced map twin '{}' \
                     not in computed demand",
                    name()
                );
            }
        }
    }

    fn report_struct_demand(env: &GlobalEnv, demand: &TwinDemand) {
        if demand.structs.is_empty() {
            return;
        }
        eprintln!("[rendering] struct twin demand (no supply exists for structs):");
        for (qid, renderings) in &demand.structs {
            let struct_env = env.get_struct(*qid);
            for rendering in renderings {
                let fields = rendering
                    .field_flags
                    .iter()
                    .filter(|(_, flag)| *flag)
                    .map(|(sym, _)| sym.display(env.symbol_pool()).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let twin = match demand
                    .struct_twin_projection
                    .get(&(*qid, rendering.inst.clone()))
                {
                    Some(Some(flags)) => format!(
                        " -> twin{}",
                        boogie_inst_suffix(env, &rendering.inst, flags)
                    ),
                    Some(None) => " (rendering not projectable onto arguments)".to_string(),
                    None => String::new(),
                };
                eprintln!(
                    "  {}{} bv fields: {}{}",
                    struct_env.get_full_name_str(),
                    boogie_inst_suffix(env, &rendering.inst, &[]),
                    fields,
                    twin,
                );
            }
        }
    }
}

/// Report a demand summary for debugging; used by tests of the walker.
pub fn demand_summary(env: &GlobalEnv, demand: &TwinDemand) -> String {
    let mut out = String::new();
    for ty in &demand.vec {
        out.push_str(&format!("vec:{}\n", boogie_type_suffix(env, ty, true)));
    }
    for (qid, insts) in &demand.table {
        for (k, v) in insts {
            out.push_str(&format!(
                "table:{}:{}_{}\n",
                env.get_struct(*qid).get_full_name_str(),
                boogie_type_suffix(env, k, false),
                boogie_type_suffix(env, v, true)
            ));
        }
    }
    for (qid, renderings) in &demand.structs {
        for rendering in renderings {
            out.push_str(&format!(
                "struct:{}:{}:{}\n",
                env.get_struct(*qid).get_full_name_str(),
                boogie_inst_suffix(env, &rendering.inst, &[]),
                rendering
                    .field_flags
                    .iter()
                    .map(|(sym, flag)| format!(
                        "{}={}",
                        sym.display(env.symbol_pool()),
                        if *flag { "bv" } else { "int" }
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
    }
    out
}
