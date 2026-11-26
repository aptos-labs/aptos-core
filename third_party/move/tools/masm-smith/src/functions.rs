// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generates functions: first every signature (visibility, type parameters, parameters, return
//! types, locals), then every body (see [`crate::blocks::build_body`]).

use crate::{
    blocks,
    config::{Config, DimensionFlag},
    error::Result,
    ir::{Function, Local, QualifiedStruct, StructLayout, Type, TypeParam},
    state::{FunctionState, LocalInit, LocalSlot, ModuleCtx, LOOP_COUNTER_NAME},
    types::{is_signer_ref, is_unsigned_integer, pick_type_param_constraint},
    violations,
};
use arbitrary::Unstructured;
use move_binary_format::file_format::{FunctionAttribute, Visibility};
use move_core_types::{
    ability::{Ability, AbilitySet},
    identifier::Identifier,
};

/// Signature-pass decisions that the body builder must preserve.
#[derive(Clone, Debug)]
pub(crate) enum BodyAdjustment {
    /// Forces a global borrow after omitting `acquires`, producing a missing-annotation
    /// violation. The orchestrator must leave the acquires list empty.
    ForceGlobalOpForDroppedAcquires,
    /// The body starts with the bounded-recursion prologue, recursing this way.
    BoundedRecursion(crate::recursion::RecursiveCall),
}

/// Signature-pass output: the function shape plus the violation decisions made at signature
/// time that the body pass must honor.
pub(crate) struct SignatureMeta {
    pub function: Function,
    pub body_adjustments: Vec<BodyAdjustment>,
    pub injected_violations: Vec<DimensionFlag>,
}

/// Generate a function *signature only* — the body is left empty. Used in the per-module
/// generation pipeline so all siblings' signatures are known before any body is generated.
pub(crate) fn build_function_signature(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_ctx: &ModuleCtx,
    name: Identifier,
) -> Result<SignatureMeta> {
    let return_types = pick_return_types(u)?;
    let params = pick_params(u, cfg, module_ctx)?;
    let (visibility, is_entry) = pick_visibility(u, &params, &return_types)?;
    let type_params = pick_function_type_params(u, cfg, is_entry, visibility)?;
    // Closure targets require `#[persistent]`. Choose it independently of visibility and
    // entry status for signatures with at most four parameters. Concrete slots are unsigned
    // integers or bools; generic slots are instantiated with primitives when packed.
    let mut attributes: Vec<FunctionAttribute> = Vec::new();
    let persistent_eligible = return_types
        .iter()
        .all(|t| is_closure_signature_ty(t, type_params.len()))
        && params
            .iter()
            .all(|p| is_closure_signature_ty(&p.ty, type_params.len()))
        && params.len() <= 4;
    if cfg.enable_closures && persistent_eligible && u.arbitrary::<bool>()? {
        attributes.push(FunctionAttribute::Persistent);
    }
    let locals_max = cfg.locals_per_fun_max.min(4);
    let local_count = u.int_in_range(0..=locals_max)?;
    let mut locals_ir: Vec<Local> = Vec::with_capacity(local_count as usize);
    for li in 0..local_count {
        locals_ir.push(Local {
            name: Identifier::new(format!("l{}", li)).expect("ident"),
            ty: pick_local_type(u, type_params.len())?,
        });
    }
    // Reserve a loop-counter local within the configured local limit. The body builder
    // may leave it unused if it chooses a loop-free CFG.
    if cfg.enable_loops
        && (locals_ir.len() as u8) < cfg.locals_per_fun_max
        && u.arbitrary::<bool>()?
    {
        locals_ir.push(Local {
            name: Identifier::new(LOOP_COUNTER_NAME).expect("ident"),
            ty: Type::U64,
        });
    }
    // Declare `acquires` for every key-able in-module struct (both generic and
    // non-generic). The bytecode acquires list is by struct-def index only — type args
    // aren't stored — so declaring just the unqualified name covers any instantiation the
    // body picks. Under `valid_acquires = false`, we deliberately omit the declaration so
    // any later global op in the body trips the acquires verifier.
    let module_keys: Vec<Identifier> = module_ctx
        .structs
        .iter()
        .filter(|s| {
            matches!(s.layout, crate::ir::StructLayout::Singleton(_))
                && s.abilities.has_ability(Ability::Key)
        })
        .map(|s| s.name.clone())
        .collect();
    let mut injected = Vec::new();
    let acquires_dropped = !module_keys.is_empty()
        && violations::should_violate(u, cfg, DimensionFlag::Acquires, &mut injected)?;
    let acquires = if acquires_dropped {
        Vec::new()
    } else {
        module_keys
    };
    let mut body_adjustments = Vec::new();
    if acquires_dropped {
        body_adjustments.push(BodyAdjustment::ForceGlobalOpForDroppedAcquires);
    }
    let function = Function {
        name,
        visibility,
        is_entry,
        is_native: false,
        attributes,
        type_params,
        params,
        locals: locals_ir,
        return_types,
        acquires,
        body: Vec::new(),
    };
    // Decided with the signature so callers generated before this body know it recurses.
    if let Some(call) = crate::recursion::pick_recursion(u, cfg, &function)? {
        body_adjustments.push(BodyAdjustment::BoundedRecursion(call));
    }
    Ok(SignatureMeta {
        function,
        body_adjustments,
        injected_violations: injected,
    })
}

fn is_primitive_for_closure(ty: &Type) -> bool {
    is_unsigned_integer(ty) || matches!(ty, Type::Bool)
}

/// Closure-eligible signature slot. Either a narrow primitive ([`is_primitive_for_closure`])
/// or a type parameter that `try_pack_closure` will bind to a primitive at pack time.
fn is_closure_signature_ty(ty: &Type, n_type_params: usize) -> bool {
    matches!(ty, Type::TypeParam(idx) if usize::from(*idx) < n_type_params)
        || is_primitive_for_closure(ty)
}

/// Optionally adds type parameters with nonempty subsets of `copy + drop + store` as
/// constraints. Generated entry calls supply no type arguments, so entries stay non-generic.
fn pick_function_type_params(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    is_entry: bool,
    visibility: Visibility,
) -> Result<Vec<TypeParam>> {
    if !cfg.enable_generics || is_entry {
        return Ok(Vec::new());
    }
    // Generic signatures are generated only for public and private functions.
    if !matches!(visibility, Visibility::Public | Visibility::Private) {
        return Ok(Vec::new());
    }
    if !u.arbitrary::<bool>()? {
        return Ok(Vec::new());
    }
    let max = cfg.type_params_max.max(1);
    let count = u.int_in_range(1..=max)? as usize;
    let mut tps = Vec::with_capacity(count);
    for i in 0..count {
        tps.push(TypeParam {
            name: Identifier::new(format!("T{}", i)).expect("ident"),
            constraints: pick_type_param_constraint(u)?,
            is_phantom: false,
        });
    }
    Ok(tps)
}

/// Body-generation summary. The orchestrator combines per-function summaries into the
/// module-level/test-level outputs.
pub(crate) struct BodyOutput {
    pub body: Vec<crate::ir::Block>,
    pub injected_violations: Vec<DimensionFlag>,
    /// Deferred binary-level mutations recorded by body generators. The orchestrator
    /// pairs each entry with the publishing step index and forwards into
    /// [`crate::ir::PostProcessRecipe::intents`].
    pub post_process_intents: Vec<crate::ir::PostProcessIntent>,
    /// Number of `ld_const` instructions this body emitted. Bubbled up so the orchestrator
    /// can advance the module-wide running offset before building the next function — see
    /// [`crate::state::FunctionState::record_ld_const_emit`] for why the offset matters.
    pub ld_const_count: u16,
    /// Names of in-module key-able structs the body actually accessed via a global op.
    /// The orchestrator prunes the function's declared `acquires` set to this — declaring
    /// `acquires S` without ever acquiring S trips the verifier's
    /// `EXTRANEOUS_ACQUIRES_ANNOTATION` check.
    pub accessed_key_structs: std::collections::BTreeSet<Identifier>,
    /// Locals allocated during body generation, to append to the function's locals.
    pub scratch_locals: Vec<Local>,
}

/// Generate the body for `function`, using `module_ctx` for cross-function call targets.
/// `adjustments` carries signature-time decisions the body builder must honor (see
/// [`BodyAdjustment`]).
pub(crate) fn build_function_body(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_ctx: &ModuleCtx,
    function: &Function,
    adjustments: &[BodyAdjustment],
    ld_const_base: u16,
) -> Result<BodyOutput> {
    let param_slots: Vec<LocalSlot> = function
        .params
        .iter()
        .map(|l| LocalSlot {
            name: l.name.clone(),
            ty: l.ty.clone(),
            init: LocalInit::Available,
        })
        .collect();
    let local_slots: Vec<LocalSlot> = function
        .locals
        .iter()
        .map(|l| LocalSlot {
            name: l.name.clone(),
            ty: l.ty.clone(),
            init: LocalInit::Uninit,
        })
        .collect();
    let type_param_constraints: Vec<AbilitySet> =
        function.type_params.iter().map(|p| p.constraints).collect();
    let mut state = FunctionState::new(
        cfg,
        module_ctx,
        param_slots,
        local_slots,
        type_param_constraints,
        function.return_types.clone(),
        ld_const_base,
    );
    if module_ctx.module_id.is_some() {
        state.function_name = Some(function.name.clone());
    }
    let mut body = blocks::build_body(u, &mut state)?;
    if adjustments
        .iter()
        .any(|a| matches!(a, BodyAdjustment::ForceGlobalOpForDroppedAcquires))
    {
        // Insert a global borrow in the first block to require the omitted acquires
        // annotation. The inserted sequence preserves the operand stack.
        blocks::append_forced_global_op(u, &mut state, &mut body)?;
    }
    let recursion = adjustments.iter().find_map(|adjustment| match adjustment {
        BodyAdjustment::BoundedRecursion(call) => Some(*call),
        BodyAdjustment::ForceGlobalOpForDroppedAcquires => None,
    });
    if let (Some(module), Some(call), Some(depth)) = (
        module_ctx.module_id.as_ref(),
        recursion,
        crate::recursion::depth_parameter(function),
    ) {
        crate::recursion::prepend_prologue(
            u, &mut state, module, function, depth, call, &mut body,
        )?;
    }
    let injected_violations = std::mem::take(&mut state.injected_violations);
    let accessed_key_structs = std::mem::take(&mut state.accessed_key_structs);
    let post_process_intents = std::mem::take(&mut state.post_process_intents);
    let ld_const_count = state.ld_const_count_in_function;
    let scratch_locals = std::mem::take(&mut state.scratch_locals);
    Ok(BodyOutput {
        body,
        injected_violations,
        post_process_intents,
        ld_const_count,
        accessed_key_structs,
        scratch_locals,
    })
}

/// Builds a script and records its text-level violations. Calls to `module_ctx.external_modules`
/// use fully qualified names. Binary mutation intents are discarded because the runner
/// reassembles scripts from text.
pub(crate) fn build_script(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_ctx: &ModuleCtx,
) -> Result<(crate::ir::Script, Vec<DimensionFlag>)> {
    // Scripts take only leading `&signer` parameters, so they can store resources and call
    // functions that need a signer; the runner passes the accounts via `--signers`.
    let signer_count = if cfg.enable_signers {
        u.int_in_range(0..=2)?
    } else {
        0
    };
    let mut signers = Vec::with_capacity(signer_count);
    let mut params = Vec::with_capacity(signer_count);
    for idx in 0..signer_count {
        signers.push(crate::types::pick_account_address(u)?);
        params.push(Local {
            name: Identifier::new(format!("p{idx}")).expect("ident"),
            ty: Type::Reference(false, Box::new(Type::Signer)),
        });
    }
    let entry = Function {
        name: Identifier::new("main").expect("ident"),
        visibility: Visibility::Public,
        is_entry: true,
        is_native: false,
        attributes: vec![],
        type_params: vec![],
        params,
        locals: vec![],
        return_types: vec![],
        acquires: vec![],
        body: Vec::new(),
    };
    let out = build_function_body(u, cfg, module_ctx, &entry, &[], 0)?;
    let entry = Function {
        body: out.body,
        locals: out.scratch_locals,
        ..entry
    };
    Ok((
        crate::ir::Script {
            module_aliases: vec![],
            entry,
            signers,
        },
        out.injected_violations,
    ))
}

fn pick_local_type(u: &mut Unstructured<'_>, type_param_count: usize) -> Result<Type> {
    if type_param_count > 0 && u.int_in_range(0u8..=3)? == 0 {
        // Occasionally use the function's own type parameter as a local type.
        let idx = u.int_in_range(0..=type_param_count - 1)? as u16;
        return Ok(Type::TypeParam(idx));
    }
    let choice = u.int_in_range(0u8..=15)?;
    Ok(match choice {
        0 => Type::U64,
        1 => Type::U8,
        2 => Type::Bool,
        3 => Type::Vector(Box::new(Type::U64)),
        4 => Type::Vector(Box::new(Type::U8)),
        5 => Type::Address,
        6 => Type::U16,
        7 => Type::U32,
        8 => Type::U128,
        9 => Type::U256,
        10 => Type::I8,
        11 => Type::I16,
        12 => Type::I32,
        13 => Type::I64,
        14 => Type::I128,
        _ => Type::I256,
    })
}

fn pick_params(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    module_ctx: &ModuleCtx,
) -> Result<Vec<Local>> {
    let max = (cfg.locals_per_fun_max / 2).clamp(1, 4);
    let count = u.int_in_range(0..=max)?;
    let mut out = Vec::with_capacity(count as usize);
    // Optionally lead with `&signer` params. Move's entry-arg / `move_to` plumbing requires
    // signers come first, before any regular args — the transactional runner prepends
    // `--signers` values to the deserialized arg list. Cap at 2 so smaller signatures stay
    // common. Tests that need stable entropy alignment can flip `enable_signers` off.
    let signer_cap = count.min(2);
    let signer_count = if cfg.enable_signers && signer_cap > 0 && u.arbitrary::<bool>()? {
        u.int_in_range(1..=signer_cap)?
    } else {
        0
    };
    for i in 0..signer_count {
        out.push(Local {
            name: Identifier::new(format!("p{}", i)).expect("ident"),
            ty: Type::Reference(false, Box::new(Type::Signer)),
        });
    }
    for i in signer_count..count {
        out.push(Local {
            name: Identifier::new(format!("p{}", i)).expect("ident"),
            ty: pick_param_type(u, module_ctx)?,
        });
    }
    Ok(out)
}

/// Pick a parameter type. Roughly equal weight between primitive types and (when any are
/// declared) a singleton struct — from either the current module or a previously-published
/// foreign module. Struct params disqualify the function from being `entry` (the
/// transactional runner can't synthesize struct args).
fn pick_param_type(u: &mut Unstructured<'_>, module_ctx: &ModuleCtx) -> Result<Type> {
    // Combined pool of (module_id, decl) for monomorphic singleton structs visible from
    // here. Foreign singletons are legal in *type* positions; we can't `pack`/`unpack`
    // them here, but they can flow through as opaque values returned from `try_call` on
    // public foreign functions.
    let mut struct_pool: Vec<(
        move_core_types::language_storage::ModuleId,
        &crate::ir::StructDecl,
    )> = Vec::new();
    if let Some(mid) = module_ctx.module_id.as_ref() {
        for s in &module_ctx.structs {
            if s.type_params.is_empty() && matches!(s.layout, StructLayout::Singleton(_)) {
                struct_pool.push((mid.clone(), s));
            }
        }
    }
    for m in &module_ctx.external_modules {
        for s in &m.structs {
            if s.type_params.is_empty() && matches!(s.layout, StructLayout::Singleton(_)) {
                struct_pool.push((m.id.clone(), s));
            }
        }
    }
    let prim = u.int_in_range(0u8..=3)?;
    match (prim, struct_pool.is_empty()) {
        (0, _) => Ok(Type::U64),
        (1, _) => Ok(Type::U8),
        (2, _) => Ok(Type::Bool),
        (3, false) => {
            let (pick_mid, pick) = u.choose(&struct_pool)?.clone();
            Ok(Type::Struct(
                QualifiedStruct {
                    module: pick_mid,
                    name: pick.name.clone(),
                },
                vec![],
            ))
        },
        _ => Ok(Type::U64),
    }
}

/// Argument types the transactional runner can synthesize as entry-function inputs:
/// primitives we serialize via `--args`, plus `&signer` (the runner prepends `--signers`
/// values; the VM auto-borrows them when the parameter is a reference — see
/// `move-vm/runtime/src/move_vm.rs::deserialize_args`).
fn is_entry_param_ty(t: &Type) -> bool {
    matches!(t, Type::U8 | Type::U64 | Type::Bool) || is_signer_ref(t)
}

fn pick_return_types(u: &mut Unstructured<'_>) -> Result<Vec<Type>> {
    let choice = u.int_in_range(0u8..=3)?;
    Ok(match choice {
        0 => vec![],
        1 => vec![Type::U64],
        2 => vec![Type::Bool],
        _ => vec![Type::U8],
    })
}

/// Chooses visibility and entry status. Entries must return nothing and accept only `u8`,
/// `u64`, `bool`, or `&signer`, which generated run steps can supply.
fn pick_visibility(
    u: &mut Unstructured<'_>,
    params: &[Local],
    return_types: &[Type],
) -> Result<(Visibility, bool)> {
    let choice = u.int_in_range(0u8..=5)?;
    let entry_args_ok = params.iter().all(|p| is_entry_param_ty(&p.ty));
    let can_be_entry = return_types.is_empty() && entry_args_ok;
    Ok(match choice {
        0 => (Visibility::Private, false),
        1 => (Visibility::Public, false),
        2 => (Visibility::Friend, false),
        3 if can_be_entry => (Visibility::Public, true),
        4 if can_be_entry => (Visibility::Public, true),
        _ => (Visibility::Public, false),
    })
}
