// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Combines two tests into one: the donor's modules are renamed and published after the
//! recipient's, and the two sets of transactions are interleaved. The result verifies whenever
//! both inputs do, and later mutations can make one test's code act on the other's state.

use crate::{
    error::Result,
    ir::{Function, MasmTest, Module, Operand, RunCall, Script, Step, StructLayout, Type},
    mutate::MUTATED_STEPS_MAX,
};
use arbitrary::Unstructured;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::collections::BTreeSet;

/// Most distinct modules a spliced test may publish.
pub const SPLICED_MODULES_MAX: usize = 4;

/// Splices a prefix of `donor`'s modules and the transactions that use only them into
/// `recipient`. Returns `None` when either test carries injected violations or the result
/// would exceed the module or step limits.
pub fn splice(
    recipient: &MasmTest,
    donor: &MasmTest,
    u: &mut Unstructured<'_>,
) -> Result<Option<MasmTest>> {
    if !is_valid_program(recipient) || !is_valid_program(donor) {
        return Ok(None);
    }
    let recipient_modules = module_ids(recipient);
    let donor_modules = module_ids(donor);
    let room = SPLICED_MODULES_MAX.saturating_sub(recipient_modules.len());
    if room == 0 || donor_modules.is_empty() {
        return Ok(None);
    }
    // A prefix is closed under dependencies: modules depend only on earlier ones.
    let keep = u.int_in_range(1..=room.min(donor_modules.len()))?;
    let kept: BTreeSet<ModuleId> = donor_modules[..keep].iter().cloned().collect();
    let renames = fresh_names(&recipient_modules, &donor_modules, keep);

    let mut donor_publishes = Vec::new();
    let mut donor_runs = Vec::new();
    let mut leading = true;
    for step in &donor.steps {
        leading &= matches!(step, Step::Publish(_));
        if !references_only(step, &kept) {
            continue;
        }
        let mut step = step.clone();
        rename_step(&mut step, &renames);
        if leading {
            donor_publishes.push(step);
        } else {
            donor_runs.push(step);
        }
    }

    let split = recipient
        .steps
        .iter()
        .position(|step| !matches!(step, Step::Publish(_)))
        .unwrap_or(recipient.steps.len());
    let (recipient_publishes, recipient_runs) = recipient.steps.split_at(split);
    let Some(budget) = MUTATED_STEPS_MAX.checked_sub(recipient.steps.len() + donor_publishes.len())
    else {
        return Ok(None);
    };
    donor_runs.truncate(budget);

    let mut steps = recipient_publishes.to_vec();
    steps.extend(donor_publishes);
    steps.extend(interleave(recipient_runs.to_vec(), donor_runs, u)?);
    Ok(Some(MasmTest {
        init: recipient.init.clone(),
        steps,
        injected_violations: Vec::new(),
        post_process: None,
    }))
}

fn is_valid_program(test: &MasmTest) -> bool {
    test.injected_violations.is_empty() && test.post_process.is_none()
}

/// Distinct module ids in first-publication order.
fn module_ids(test: &MasmTest) -> Vec<ModuleId> {
    let mut ids: Vec<ModuleId> = Vec::new();
    for step in &test.steps {
        if let Step::Publish(module) = step
            && !ids.contains(&module.id)
        {
            ids.push(module.id.clone());
        }
    }
    ids
}

/// Maps each of the first `keep` donor modules to a name in the same account that neither
/// test uses, so renaming in one pass cannot merge two modules.
fn fresh_names(
    recipient: &[ModuleId],
    donor: &[ModuleId],
    keep: usize,
) -> Vec<(ModuleId, ModuleId)> {
    let mut taken: BTreeSet<String> = recipient
        .iter()
        .chain(donor)
        .map(|id| id.name().as_str().to_owned())
        .collect();
    let mut next = 0usize;
    donor[..keep]
        .iter()
        .map(|id| {
            let name = loop {
                let candidate = format!("m{next}");
                next += 1;
                if taken.insert(candidate.clone()) {
                    break candidate;
                }
            };
            let renamed = ModuleId::new(
                *id.address(),
                Identifier::new(name).expect("generated module names are identifiers"),
            );
            (id.clone(), renamed)
        })
        .collect()
}

/// Interleaves two step sequences at random, keeping each one's internal order.
fn interleave(left: Vec<Step>, right: Vec<Step>, u: &mut Unstructured<'_>) -> Result<Vec<Step>> {
    let mut merged = Vec::with_capacity(left.len() + right.len());
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    loop {
        let step = match (left.peek().is_some(), right.peek().is_some()) {
            (true, true) => {
                if u.arbitrary::<bool>()? {
                    left.next()
                } else {
                    right.next()
                }
            },
            (true, false) => left.next(),
            (false, true) => right.next(),
            (false, false) => break,
        };
        merged.extend(step);
    }
    Ok(merged)
}

fn references_only(step: &Step, allowed: &BTreeSet<ModuleId>) -> bool {
    let mut step = step.clone();
    let mut ok = true;
    visit_step(&mut step, &mut |id| ok &= allowed.contains(id));
    ok
}

fn rename_step(step: &mut Step, renames: &[(ModuleId, ModuleId)]) {
    visit_step(step, &mut |id| {
        if let Some((_, renamed)) = renames.iter().find(|(from, _)| from == id) {
            *id = renamed.clone();
        }
    });
}

/// Calls `visit` on every module id a step names, other than the stdlib's.
fn visit_step(step: &mut Step, visit: &mut impl FnMut(&mut ModuleId)) {
    match step {
        Step::Publish(module) => visit_module(module, visit),
        Step::RunScript(script) => visit_script(script, visit),
        Step::RunCall(call) => visit_call(call, visit),
    }
}

fn visit_module(module: &mut Module, visit: &mut impl FnMut(&mut ModuleId)) {
    visit_generated(&mut module.id, visit);
    for (_, id) in &mut module.module_aliases {
        visit_generated(id, visit);
    }
    for id in &mut module.friends {
        visit_generated(id, visit);
    }
    for decl in &mut module.structs {
        match &mut decl.layout {
            StructLayout::Singleton(fields) => {
                for field in fields {
                    visit_type(&mut field.ty, visit);
                }
            },
            StructLayout::Variants(variants) => {
                for field in variants.iter_mut().flat_map(|variant| &mut variant.fields) {
                    visit_type(&mut field.ty, visit);
                }
            },
        }
    }
    for function in &mut module.functions {
        visit_function(function, visit);
    }
}

fn visit_script(script: &mut Script, visit: &mut impl FnMut(&mut ModuleId)) {
    for (_, id) in &mut script.module_aliases {
        visit_generated(id, visit);
    }
    visit_function(&mut script.entry, visit);
}

fn visit_call(call: &mut RunCall, visit: &mut impl FnMut(&mut ModuleId)) {
    visit_generated(&mut call.target.module, visit);
    for ty in &mut call.type_args {
        visit_type(ty, visit);
    }
}

fn visit_function(function: &mut Function, visit: &mut impl FnMut(&mut ModuleId)) {
    for local in function.params.iter_mut().chain(function.locals.iter_mut()) {
        visit_type(&mut local.ty, visit);
    }
    for ty in &mut function.return_types {
        visit_type(ty, visit);
    }
    for instr in function
        .body
        .iter_mut()
        .flat_map(|block| block.instrs.iter_mut())
    {
        for operand in &mut instr.operands {
            visit_operand(operand, visit);
        }
    }
}

fn visit_operand(operand: &mut Operand, visit: &mut impl FnMut(&mut ModuleId)) {
    match operand {
        Operand::Function(function, type_args) => {
            visit_generated(&mut function.module, visit);
            type_args.iter_mut().for_each(|ty| visit_type(ty, visit));
        },
        Operand::Struct(owner, type_args)
        | Operand::Field {
            owner, type_args, ..
        }
        | Operand::VariantFields {
            owner, type_args, ..
        } => {
            visit_generated(&mut owner.module, visit);
            type_args.iter_mut().for_each(|ty| visit_type(ty, visit));
        },
        Operand::Type(ty) => visit_type(ty, visit),
        Operand::Imm(_) | Operand::Local(_) | Operand::Label(_) | Operand::ClosureMask(_) => {},
    }
}

fn visit_type(ty: &mut Type, visit: &mut impl FnMut(&mut ModuleId)) {
    match ty {
        Type::Struct(owner, type_args) => {
            visit_generated(&mut owner.module, visit);
            type_args.iter_mut().for_each(|arg| visit_type(arg, visit));
        },
        Type::Vector(inner) | Type::Reference(_, inner) => visit_type(inner, visit),
        Type::Function { args, results, .. } => {
            args.iter_mut()
                .chain(results.iter_mut())
                .for_each(|arg| visit_type(arg, visit));
        },
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Address
        | Type::Signer
        | Type::TypeParam(_) => {},
    }
}

/// Visits `id` unless it names a stdlib module, which splicing neither renames nor filters.
fn visit_generated(id: &mut ModuleId, visit: &mut impl FnMut(&mut ModuleId)) {
    if !id.address().is_special() {
        visit(id);
    }
}
