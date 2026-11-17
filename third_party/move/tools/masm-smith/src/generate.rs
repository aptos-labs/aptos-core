// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Builds a whole test from the input bytes.
//!
//! Modules are generated one at a time, declarations first and function bodies last, and each
//! module may call into the ones before it. Transactions that call functions or run scripts come
//! after every module is published.

use crate::{
    config::{Config, DimensionFlag},
    error::{Error, Result},
    functions,
    ir::{
        InitDirective, MasmTest, Module, QualifiedFunction, RunCall, RuntimeArg, Step, Type, Value,
    },
    state::ModuleCtx,
    struct_api, structs,
    types::{is_signer_ref, pick_account_address},
    violations as violation_helper,
};
use arbitrary::Unstructured;
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

pub fn generate(u: &mut Unstructured<'_>, cfg: &Config) -> Result<MasmTest> {
    let init = Some(init_directive());
    let module_count = pick_module_count(u, cfg)?;
    // Pre-compute all module ids so any module can declare any other as a friend, regardless
    // of generation order.
    let module_ids: Vec<ModuleId> = (0..module_count)
        .map(|i| {
            ModuleId::new(
                AccountAddress::from_hex_literal("0x42").expect("address"),
                ident(&format!("m{}", i)),
            )
        })
        .collect();
    let mut steps = Vec::with_capacity(module_count as usize);
    let mut injected_violations = Vec::new();
    let mut callable: Vec<(QualifiedFunction, Vec<Type>)> = Vec::new();
    let mut published: Vec<Module> = Vec::with_capacity(module_count as usize);
    let mut intents: Vec<(usize, crate::ir::PostProcessIntent)> = Vec::new();
    for i in 0..module_count {
        let out = build_module(u, cfg, i, &module_ids, &published)?;
        injected_violations.extend(out.injected_violations);
        let step_idx = steps.len();
        intents.extend(out.post_process_intents.into_iter().map(|i| (step_idx, i)));
        callable.extend(entry_callables(&out.module));
        steps.push(Step::Publish(out.module.clone()));
        published.push(out.module);
    }

    // Fill the remaining step budget with entry-function calls or inline scripts.
    let post_publish_max = cfg.steps_max.saturating_sub(steps.len() as u8);
    let post_count = u.int_in_range(0..=post_publish_max)?;
    for _ in 0..post_count {
        // Without a callable entry function, only an inline script is available.
        let want_script =
            callable.is_empty() || violation_helper::chance(u, cfg.run_script_vs_call_prob)?;
        if want_script {
            let script_ctx = ModuleCtx {
                module_id: None,
                structs: Vec::new(),
                functions: Vec::new(),
                external_modules: published.clone(),
                ..Default::default()
            };
            let (script, script_violations) = functions::build_script(u, cfg, &script_ctx)?;
            injected_violations.extend(script_violations);
            steps.push(Step::RunScript(script));
        } else {
            let (target, param_types) = u.choose(&callable)?.clone();
            let (signers, args) = build_call_args(u, cfg, &param_types)?;
            steps.push(Step::RunCall(RunCall {
                target,
                type_args: Vec::new(),
                args,
                signers,
            }));
        }
    }

    let post_process = if intents.is_empty() {
        None
    } else {
        Some(crate::ir::PostProcessRecipe { intents })
    };

    Ok(MasmTest {
        init,
        steps,
        injected_violations,
        post_process,
    })
}

fn pick_module_count(u: &mut Unstructured<'_>, cfg: &Config) -> Result<u8> {
    let max = cfg.module_count_max.max(1);
    Ok(u.int_in_range(1..=max)?)
}

fn init_directive() -> InitDirective {
    InitDirective {
        addresses: vec![(
            ident("gen"),
            AccountAddress::from_hex_literal("0x42").expect("address"),
        )],
    }
}

fn build_module(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    idx: u8,
    all_module_ids: &[ModuleId],
    external: &[Module],
) -> Result<ModuleBuildOutput> {
    let id = all_module_ids[idx as usize].clone();

    let (struct_decls, mut violations) = structs::build_structs(u, cfg, &id)?;

    // Choose each other module as a friend with 50% probability, without duplicates.
    // A friends violation adds the module itself to its friend list.
    let mut friends: Vec<ModuleId> = Vec::new();
    if cfg.enable_friends {
        for other in all_module_ids {
            if other == &id {
                continue;
            }
            if u.arbitrary::<bool>()? {
                friends.push(other.clone());
            }
        }
    }
    if violation_helper::should_violate(u, cfg, DimensionFlag::Friends, &mut violations)? {
        friends.push(id.clone());
    }

    // Struct API wrappers have fixed bodies and do not count toward `functions_per_module_max`.
    let wrappers = struct_api::build_wrappers(u, cfg, &id, &struct_decls)?;
    violations.extend(wrappers.injected_violations);

    // Generate all signatures before bodies so sibling call targets are available.
    let mut module_ctx = ModuleCtx {
        module_id: Some(id.clone()),
        structs: struct_decls.clone(),
        functions: Vec::new(),
        external_modules: external.to_vec(),
        ..Default::default()
    };
    let fun_count = u.int_in_range(0..=cfg.functions_per_module_max)?;
    let mut sig_metas: Vec<functions::SignatureMeta> = Vec::with_capacity(fun_count as usize);
    for fi in 0..fun_count {
        let name = ident(&format!("f{}", fi));
        let meta = functions::build_function_signature(u, cfg, &module_ctx, name)?;
        violations.extend(meta.injected_violations.iter().copied());
        sig_metas.push(meta);
    }
    module_ctx.functions = sig_metas
        .iter()
        .map(|m| m.function.clone())
        .chain(wrappers.functions.iter().cloned())
        .collect();
    module_ctx.recursive = sig_metas
        .iter()
        .filter(|meta| {
            meta.body_adjustments.iter().any(|adjustment| {
                matches!(adjustment, functions::BodyAdjustment::BoundedRecursion(_))
            })
        })
        .map(|meta| meta.function.name.clone())
        .collect();

    // Place wrappers before callers so the struct API checker validates them first.
    let mut funs = wrappers.functions;
    let mut post_process_intents: Vec<crate::ir::PostProcessIntent> = Vec::new();
    // Count `ld_const` instructions across functions to estimate constant-pool indices.
    // Assembler deduplication can make the actual indices smaller.
    let mut ld_const_running: u16 = 0;
    for meta in &sig_metas {
        let out = functions::build_function_body(
            u,
            cfg,
            &module_ctx,
            &meta.function,
            &meta.body_adjustments,
            ld_const_running,
        )?;
        ld_const_running = ld_const_running.saturating_add(out.ld_const_count);
        let mut f = meta.function.clone();
        f.body = out.body;
        f.locals.extend(out.scratch_locals);
        // Remove unused acquires annotations to avoid `EXTRANEOUS_ACQUIRES_ANNOTATION`.
        // For a deliberate missing-annotation violation, keep the list empty: the body
        // builder inserts a global borrow that requires the omitted annotation.
        let acquires_was_dropped = meta.body_adjustments.iter().any(|a| {
            matches!(
                a,
                functions::BodyAdjustment::ForceGlobalOpForDroppedAcquires
            )
        });
        if !acquires_was_dropped {
            f.acquires
                .retain(|name| out.accessed_key_structs.contains(name));
        }
        funs.push(f);
        violations.extend(out.injected_violations);
        post_process_intents.extend(out.post_process_intents);
    }

    Ok(ModuleBuildOutput {
        module: Module {
            id,
            address_aliases: vec![],
            module_aliases: vec![],
            friends,
            structs: struct_decls,
            functions: funs,
        },
        injected_violations: violations,
        post_process_intents,
    })
}

/// Functions of `module` a `run` step can call, with their parameter types: public entry
/// functions without return values.
pub(crate) fn entry_callables(
    module: &Module,
) -> impl Iterator<Item = (QualifiedFunction, Vec<Type>)> + '_ {
    module
        .functions
        .iter()
        .filter(|function| {
            function.is_entry
                && function.visibility == move_binary_format::file_format::Visibility::Public
                && function.return_types.is_empty()
        })
        .map(|function| {
            (
                QualifiedFunction {
                    module: module.id.clone(),
                    name: function.name.clone(),
                },
                function
                    .params
                    .iter()
                    .map(|param| param.ty.clone())
                    .collect(),
            )
        })
}

struct ModuleBuildOutput {
    module: Module,
    injected_violations: Vec<DimensionFlag>,
    post_process_intents: Vec<crate::ir::PostProcessIntent>,
}

fn ident(name: &str) -> Identifier {
    Identifier::new(name).expect("valid identifier")
}

/// Split a callable's parameter types into the leading `&signer` slots (handed to the runner
/// via `--signers`) and the remaining synthesizable primitives (handed via `--args`). The
/// runner prepends serialized signer values to the deserialized arg list, so the entry
/// function's params line up positionally with `signers ++ args`.
pub(crate) fn build_call_args(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    param_types: &[Type],
) -> Result<(Vec<AccountAddress>, Vec<RuntimeArg>)> {
    let mut signers = Vec::new();
    let mut args = Vec::new();
    let mut signer_phase = true;
    for ty in param_types {
        if signer_phase && is_signer_ref(ty) {
            signers.push(pick_account_address(u)?);
            continue;
        }
        signer_phase = false;
        args.push(match ty {
            Type::U8 => match crate::instructions::integer_literal(u, cfg, ty)?.1 {
                Value::U8(value) => RuntimeArg::U8(value),
                _ => unreachable!("integer_literal returns the requested type"),
            },
            // Mostly small, like literals, so a bounded-recursion depth is usually shallow.
            Type::U64 => match crate::instructions::integer_literal(u, cfg, ty)?.1 {
                Value::U64(value) => RuntimeArg::U64(value),
                _ => unreachable!("integer_literal returns the requested type"),
            },
            Type::Bool => RuntimeArg::Bool(u.arbitrary()?),
            // `pick_visibility` restricts entry parameters to types handled above.
            other => {
                return Err(Error::Internal(format!(
                    "entry param type not synthesizable as a runtime arg: {:?}",
                    other
                )))
            },
        });
    }
    Ok((signers, args))
}
