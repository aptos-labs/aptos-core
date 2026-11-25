// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Mutates generated tests while keeping them verifiable.
//!
//! The fuzz corpus stores each test as serialized IR ([`encode`] / [`decode`]), not as the bytes
//! that generated it, so entries survive generator changes and a small mutation makes a small
//! change. Each mutation in [`mutate`] either regenerates part of the test or makes a local edit
//! that keeps types intact. Some add, repeat, move, or drop transactions, so later ones see state
//! left by earlier ones, including across module upgrades.

use crate::{
    config::Config,
    error::Result,
    functions,
    generate::{build_call_args, entry_callables},
    instructions,
    ir::{Function, Instr, MasmTest, Module, RunCall, Step, Value},
    state::{ModuleCtx, LOOP_COUNTER_NAME},
    types::{is_signer_ref, pick_account_address},
};
use arbitrary::Unstructured;
use move_core_types::{
    ability::Ability,
    int256::{I256, U256},
};

/// Leading byte of an encoded test. Bumped whenever the IR's serialized form changes, so
/// entries written by an older generator are rejected instead of decoded wrongly.
const ENCODING_VERSION: u8 = 3;

/// Serializes `test` for storage in a fuzzing corpus: version byte, BCS body, then a checksum
/// of both so byte-level edits are rejected rather than decoded into a malformed test.
pub fn encode(test: &MasmTest) -> Vec<u8> {
    let mut bytes = vec![ENCODING_VERSION];
    bytes.extend(bcs::to_bytes(test).expect("IR types serialize"));
    let checksum = fnv1a(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    bytes
}

/// Inverse of [`encode`]. Returns `None` for bytes that are not an intact encoded test of the
/// current version.
pub fn decode(bytes: &[u8]) -> Option<MasmTest> {
    let split = bytes.len().checked_sub(CHECKSUM_LEN)?;
    let (payload, checksum) = bytes.split_at(split);
    if checksum != fnv1a(payload).to_le_bytes() {
        return None;
    }
    let (&version, body) = payload.split_first()?;
    if version != ENCODING_VERSION {
        return None;
    }
    bcs::from_bytes(body).ok()
}

const CHECKSUM_LEN: usize = 8;

/// 64-bit FNV-1a. Detects accidental corruption; not collision resistant.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xCBF2_9CE4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

/// Applies one mutation to `test`, drawing every choice from `u`. Regenerated code follows
/// `cfg`, which must be the all-valid config `test` was generated with. Returns `false` when
/// no mutation applies, leaving `test` unchanged.
pub fn mutate(test: &mut MasmTest, cfg: &Config, u: &mut Unstructured<'_>) -> Result<bool> {
    for _ in 0..MUTATION_ATTEMPTS {
        let applied = match u.int_in_range(0u8..=10)? {
            0 => regenerate_function_body(test, cfg, u)?,
            1 => perturb_literal(test, u)?,
            2 => swap_operator(test, u)?,
            3 => insert_run_step(test, cfg, u)?,
            4 => duplicate_run_step(test, u)?,
            5 => move_run_step(test, u)?,
            6 => remove_run_step(test, u)?,
            7 => republish_module(test, cfg, u)?,
            8 => retarget_address(test, u)?,
            9 => reroll_call_arguments(test, cfg, u)?,
            _ => seed_resource(test, cfg, u)?,
        };
        if applied {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Each mutation kind is tried at most this many times before [`mutate`] gives up; most fail
/// only when the test has no site of that kind.
const MUTATION_ATTEMPTS: usize = 8;

/// Step limit for mutated and spliced tests, above the generator's so mutation can build
/// longer transaction histories.
pub const MUTATED_STEPS_MAX: usize = 16;

/// Replaces one generated function's body with a freshly generated one, keeping its
/// signature. Struct API wrappers keep their fixed bodies.
fn regenerate_function_body(
    test: &mut MasmTest,
    cfg: &Config,
    u: &mut Unstructured<'_>,
) -> Result<bool> {
    let modules: Vec<usize> = published_modules(test).map(|(idx, _)| idx).collect();
    let Some(&step_idx) = choose(u, &modules)? else {
        return Ok(false);
    };
    regenerate_body_in_step(test, cfg, step_idx, u)
}

/// Regenerates one function body of the module published at `step_idx`.
fn regenerate_body_in_step(
    test: &mut MasmTest,
    cfg: &Config,
    step_idx: usize,
    u: &mut Unstructured<'_>,
) -> Result<bool> {
    let Step::Publish(module) = &test.steps[step_idx] else {
        unreachable!("callers pass publish steps")
    };
    let candidates: Vec<usize> = module
        .functions
        .iter()
        .enumerate()
        .filter(|(_, function)| !function.is_struct_api_wrapper())
        .map(|(fun_idx, _)| fun_idx)
        .collect();
    let Some(&fun_idx) = choose(u, &candidates)? else {
        return Ok(false);
    };
    // Callees come only from modules first published before this one, so an upgrade cannot
    // introduce a dependency cycle.
    let dependencies: Vec<_> = test
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Publish(published) => Some(published.id.clone()),
            Step::RunScript(_) | Step::RunCall(_) => None,
        })
        .take_while(|id| *id != module.id)
        .collect();
    let external: Vec<Module> = latest_modules_before(test, step_idx)
        .into_iter()
        .filter(|other| dependencies.contains(&other.id))
        .collect();
    let module_ctx = ModuleCtx {
        module_id: Some(module.id.clone()),
        structs: module.structs.clone(),
        functions: module.functions.clone(),
        external_modules: external,
        ..Default::default()
    };
    let original = &module.functions[fun_idx];
    let signature = signature_with_all_acquires(original, module);
    // A recursive function stays recursive the same way, so its callers' depth arguments
    // still match.
    let adjustments: Vec<functions::BodyAdjustment> = crate::recursion::recursive_call(original)
        .map(functions::BodyAdjustment::BoundedRecursion)
        .into_iter()
        .collect();
    let out = functions::build_function_body(u, cfg, &module_ctx, &signature, &adjustments, 0)?;
    // Callers were generated against the original `acquires` list; a body that acquires a
    // different set would invalidate them.
    let mut acquires = signature.acquires.clone();
    acquires.retain(|name| out.accessed_key_structs.contains(name));
    if acquires != original.acquires {
        return Ok(false);
    }
    let mut regenerated = signature;
    regenerated.body = out.body;
    regenerated.locals.extend(out.scratch_locals);
    regenerated.acquires = acquires;
    let Step::Publish(module) = &mut test.steps[step_idx] else {
        unreachable!("callers pass publish steps")
    };
    module.functions[fun_idx] = regenerated;
    Ok(true)
}

/// Inserts a fresh `run` step after the last publish: a call to an entry function with new
/// arguments, or a newly generated script over every module published so far.
fn insert_run_step(test: &mut MasmTest, cfg: &Config, u: &mut Unstructured<'_>) -> Result<bool> {
    if test.steps.len() >= MUTATED_STEPS_MAX {
        return Ok(false);
    }
    let position = u.int_in_range(first_run_position(test)..=test.steps.len())?;
    let step = fresh_run_step(test, cfg, position, u)?;
    test.steps.insert(position, step);
    Ok(true)
}

/// Repeats a `run` step later in the sequence, so it runs against the state its first
/// execution left behind (e.g. a second `move_to` of the same resource).
fn duplicate_run_step(test: &mut MasmTest, u: &mut Unstructured<'_>) -> Result<bool> {
    if test.steps.len() >= MUTATED_STEPS_MAX {
        return Ok(false);
    }
    let runs = run_step_indices(test);
    let Some(&source) = choose(u, &runs)? else {
        return Ok(false);
    };
    let position = u.int_in_range(source + 1..=test.steps.len())?;
    let step = test.steps[source].clone();
    test.steps.insert(position, step);
    Ok(true)
}

/// Moves a `run` step to another position after the leading publishes, which declare every
/// module a run step can reference.
fn move_run_step(test: &mut MasmTest, u: &mut Unstructured<'_>) -> Result<bool> {
    let runs = run_step_indices(test);
    let Some(&source) = choose(u, &runs)? else {
        return Ok(false);
    };
    let step = test.steps.remove(source);
    let position = u.int_in_range(first_run_position(test)..=test.steps.len())?;
    test.steps.insert(position, step);
    Ok(position != source)
}

/// Removes a `run` step or a module upgrade, balancing the mutations that add steps.
fn remove_run_step(test: &mut MasmTest, u: &mut Unstructured<'_>) -> Result<bool> {
    let removable: Vec<usize> = (first_run_position(test)..test.steps.len()).collect();
    let Some(&index) = choose(u, &removable)? else {
        return Ok(false);
    };
    test.steps.remove(index);
    Ok(true)
}

/// Republishes a module with one function body regenerated, appended after the existing
/// steps and followed by fresh `run` steps. Struct layouts and every signature are unchanged,
/// so the upgrade passes the compatibility check, and the new code runs against resources the
/// old code stored.
fn republish_module(test: &mut MasmTest, cfg: &Config, u: &mut Unstructured<'_>) -> Result<bool> {
    if test.steps.len() + 2 > MUTATED_STEPS_MAX {
        return Ok(false);
    }
    let modules: Vec<usize> = published_modules(test).map(|(idx, _)| idx).collect();
    let Some(&step_idx) = choose(u, &modules)? else {
        return Ok(false);
    };
    let mut upgraded = test.clone();
    let Step::Publish(module) = upgraded.steps[step_idx].clone() else {
        unreachable!("chosen from publish steps")
    };
    upgraded.steps.push(Step::Publish(module));
    let upgrade_idx = upgraded.steps.len() - 1;
    if !regenerate_body_in_step(&mut upgraded, cfg, upgrade_idx, u)? {
        return Ok(false);
    }
    let follow_up = fresh_run_step(&upgraded, cfg, upgraded.steps.len(), u)?;
    upgraded.steps.push(follow_up);
    *test = upgraded;
    Ok(true)
}

/// Makes a function that takes a `&signer` store a resource on entry, so transactions that
/// reach it leave state for later ones.
fn seed_resource(test: &mut MasmTest, cfg: &Config, u: &mut Unstructured<'_>) -> Result<bool> {
    let candidates: Vec<(usize, usize)> = published_modules(test)
        .flat_map(|(step_idx, module)| {
            module
                .functions
                .iter()
                .enumerate()
                .filter(|(_, function)| {
                    !function.is_struct_api_wrapper()
                        && function.params.iter().any(|param| is_signer_ref(&param.ty))
                })
                .map(move |(fun_idx, _)| (step_idx, fun_idx))
        })
        .collect();
    let Some(&(step_idx, fun_idx)) = choose(u, &candidates)? else {
        return Ok(false);
    };
    let Step::Publish(module) = &test.steps[step_idx] else {
        unreachable!("candidates come from publish steps")
    };
    let module_ctx = ModuleCtx {
        module_id: Some(module.id.clone()),
        structs: module.structs.clone(),
        functions: module.functions.clone(),
        external_modules: Vec::new(),
        ..Default::default()
    };
    let function = &module.functions[fun_idx];
    let Some(prefix) = instructions::resource_store_prefix(u, cfg, &module_ctx, function)? else {
        return Ok(false);
    };
    let Step::Publish(module) = &mut test.steps[step_idx] else {
        unreachable!("candidates come from publish steps")
    };
    let function = &mut module.functions[fun_idx];
    let entry = crate::recursion::body_entry_index(function);
    let Some(entry) = function.body.get_mut(entry) else {
        return Ok(false);
    };
    entry.instrs.splice(0..0, prefix);
    Ok(true)
}

/// Points one address literal at an account from the shared pool, aliasing the accounts that
/// different transactions read and write.
fn retarget_address(test: &mut MasmTest, u: &mut Unstructured<'_>) -> Result<bool> {
    let sites = instruction_sites(test, |instr| {
        matches!(instr.operands.as_slice(), [
            _,
            crate::ir::Operand::Imm(Value::Address(_))
        ])
    });
    let Some(&site) = choose(u, &sites)? else {
        return Ok(false);
    };
    let instr = instruction_at(test, site);
    let [_, crate::ir::Operand::Imm(Value::Address(address))] = instr.operands.as_mut_slice()
    else {
        unreachable!("site filter requires an address literal")
    };
    let replacement = pick_account_address(u)?;
    let changed = *address != replacement;
    *address = replacement;
    Ok(changed)
}

/// Draws new signers and arguments for one entry-function call.
fn reroll_call_arguments(
    test: &mut MasmTest,
    cfg: &Config,
    u: &mut Unstructured<'_>,
) -> Result<bool> {
    let calls: Vec<usize> = test
        .steps
        .iter()
        .enumerate()
        .filter(|(_, step)| matches!(step, Step::RunCall(_)))
        .map(|(idx, _)| idx)
        .collect();
    let Some(&index) = choose(u, &calls)? else {
        return Ok(false);
    };
    let Step::RunCall(call) = &test.steps[index] else {
        unreachable!("chosen from call steps")
    };
    let Some(param_types) = latest_modules_before(test, index)
        .iter()
        .flat_map(entry_callables)
        .find(|(target, _)| *target == call.target)
        .map(|(_, param_types)| param_types)
    else {
        return Ok(false);
    };
    let (signers, args) = build_call_args(u, cfg, &param_types)?;
    let Step::RunCall(call) = &mut test.steps[index] else {
        unreachable!("chosen from call steps")
    };
    call.signers = signers;
    call.args = args;
    Ok(true)
}

/// Index of the first step after the last of the leading publishes.
fn first_run_position(test: &MasmTest) -> usize {
    test.steps
        .iter()
        .position(|step| !matches!(step, Step::Publish(_)))
        .unwrap_or(test.steps.len())
}

fn run_step_indices(test: &MasmTest) -> Vec<usize> {
    test.steps
        .iter()
        .enumerate()
        .filter(|(_, step)| matches!(step, Step::RunScript(_) | Step::RunCall(_)))
        .map(|(idx, _)| idx)
        .collect()
}

/// A new `run` step for `position`: a call to an entry function published before it, or a
/// script over the modules published before it.
fn fresh_run_step(
    test: &MasmTest,
    cfg: &Config,
    position: usize,
    u: &mut Unstructured<'_>,
) -> Result<Step> {
    let published = latest_modules_before(test, position);
    let callable: Vec<_> = published.iter().flat_map(entry_callables).collect();
    let want_script =
        callable.is_empty() || crate::violations::chance(u, cfg.run_script_vs_call_prob)?;
    if want_script {
        let script_ctx = ModuleCtx {
            module_id: None,
            structs: Vec::new(),
            functions: Vec::new(),
            external_modules: published,
            ..Default::default()
        };
        let (script, _violations) = functions::build_script(u, cfg, &script_ctx)?;
        return Ok(Step::RunScript(script));
    }
    let (target, param_types) = u.choose(&callable)?.clone();
    let (signers, args) = build_call_args(u, cfg, &param_types)?;
    Ok(Step::RunCall(RunCall {
        target,
        type_args: Vec::new(),
        args,
        signers,
    }))
}

/// The latest published version of every module published before `position`, in first
/// publication order.
fn latest_modules_before(test: &MasmTest, position: usize) -> Vec<Module> {
    let mut modules: Vec<Module> = Vec::new();
    for step in &test.steps[..position] {
        if let Step::Publish(module) = step {
            match modules.iter_mut().find(|existing| existing.id == module.id) {
                Some(existing) => *existing = module.clone(),
                None => modules.push(module.clone()),
            }
        }
    }
    modules
}

/// `function` with its body cleared, its scratch locals dropped, and its `acquires` list
/// widened to every key struct the module declares, matching what the signature pass hands
/// to body generation.
fn signature_with_all_acquires(function: &Function, module: &Module) -> Function {
    let mut signature = function.clone();
    signature.body.clear();
    signature
        .locals
        .retain(|local| !local.name.as_str().starts_with("__scratch"));
    signature.acquires = module
        .structs
        .iter()
        .filter(|decl| {
            matches!(decl.layout, crate::ir::StructLayout::Singleton(_))
                && decl.abilities.has_ability(Ability::Key)
        })
        .map(|decl| decl.name.clone())
        .collect();
    signature
}

/// Rewrites the value of one literal load, keeping its type.
fn perturb_literal(test: &mut MasmTest, u: &mut Unstructured<'_>) -> Result<bool> {
    let sites = instruction_sites(test, |instr| {
        matches!(instr.operands.as_slice(), [crate::ir::Operand::Imm(value)]
            if is_scalar_literal(&instr.opcode, value))
    });
    let Some(&site) = choose(u, &sites)? else {
        return Ok(false);
    };
    let instr = instruction_at(test, site);
    let [crate::ir::Operand::Imm(value)] = instr.operands.as_mut_slice() else {
        unreachable!("site filter requires one immediate operand")
    };
    *value = perturbed(value, u)?;
    Ok(true)
}

fn is_scalar_literal(opcode: &str, value: &Value) -> bool {
    opcode.starts_with("ld_")
        && matches!(
            value,
            Value::U8(_)
                | Value::U16(_)
                | Value::U32(_)
                | Value::U64(_)
                | Value::U128(_)
                | Value::U256(_)
                | Value::I8(_)
                | Value::I16(_)
                | Value::I32(_)
                | Value::I64(_)
                | Value::I128(_)
                | Value::I256(_)
        )
}

/// A value of the same type as `value`: a boundary value or a random one.
fn perturbed(value: &Value, u: &mut Unstructured<'_>) -> Result<Value> {
    // 0..=3 pick a boundary (zero, one, max, min); 4 picks a random value.
    let pick = u.int_in_range(0u8..=4)?;
    macro_rules! scalar {
        ($variant:ident, $ty:ty) => {
            Value::$variant(match pick {
                0 => 0,
                1 => 1,
                2 => <$ty>::MAX,
                3 => <$ty>::MIN,
                _ => u.arbitrary()?,
            })
        };
    }
    Ok(match value {
        Value::U8(_) => scalar!(U8, u8),
        Value::U16(_) => scalar!(U16, u16),
        Value::U32(_) => scalar!(U32, u32),
        Value::U64(_) => scalar!(U64, u64),
        Value::U128(_) => scalar!(U128, u128),
        Value::I8(_) => scalar!(I8, i8),
        Value::I16(_) => scalar!(I16, i16),
        Value::I32(_) => scalar!(I32, i32),
        Value::I64(_) => scalar!(I64, i64),
        Value::I128(_) => scalar!(I128, i128),
        Value::U256(_) => Value::U256(match pick {
            0 => U256::ZERO,
            1 => U256::ONE,
            2 => U256::MAX,
            3 => U256::MIN,
            _ => U256::from_le_bytes(u.arbitrary()?),
        }),
        Value::I256(_) => Value::I256(match pick {
            0 => I256::ZERO,
            1 => I256::ONE,
            2 => I256::MAX,
            3 => I256::MIN,
            _ => I256::from_le_bytes(u.arbitrary()?),
        }),
        Value::Bool(_) | Value::Address(_) | Value::Bytes(_) | Value::Vector(_) => {
            unreachable!("is_scalar_literal admits only integer literals")
        },
    })
}

/// Opcodes that pop the same operand types and push the same result type, so any member can
/// replace any other.
const OPERATOR_GROUPS: &[&[&str]] = &[
    &["add", "sub", "mul", "div", "mod"],
    &["bit_and", "bit_or", "xor"],
    &["shl", "shr"],
    &["lt", "le", "gt", "ge"],
    &["eq", "neq"],
    &["and", "or"],
];

/// Replaces one binary operator with another from its group.
fn swap_operator(test: &mut MasmTest, u: &mut Unstructured<'_>) -> Result<bool> {
    let sites = instruction_sites(test, |instr| {
        instr.operands.is_empty() && operator_group(&instr.opcode).is_some()
    });
    let Some(&site) = choose(u, &sites)? else {
        return Ok(false);
    };
    let instr = instruction_at(test, site);
    let group = operator_group(&instr.opcode).expect("site filter requires a grouped opcode");
    let alternatives: Vec<&str> = group
        .iter()
        .copied()
        .filter(|opcode| *opcode != instr.opcode)
        .collect();
    let Some(&replacement) = choose(u, &alternatives)? else {
        return Ok(false);
    };
    instr.opcode = replacement.to_string();
    Ok(true)
}

fn operator_group(opcode: &str) -> Option<&'static [&'static str]> {
    OPERATOR_GROUPS
        .iter()
        .copied()
        .find(|group| group.contains(&opcode))
}

/// Location of one instruction: step, function (`None` for a script's entry), block, index.
#[derive(Clone, Copy)]
struct Site {
    step: usize,
    function: Option<usize>,
    block: usize,
    instr: usize,
}

/// Every instruction in generated code accepted by `keep`. Struct API wrappers are excluded:
/// their bodies have a fixed shape the verifier checks exactly. So are loop and recursion
/// bounds.
fn instruction_sites(test: &MasmTest, keep: impl Fn(&Instr) -> bool) -> Vec<Site> {
    let mut sites = Vec::new();
    for (step_idx, step) in test.steps.iter().enumerate() {
        let functions: Vec<(Option<usize>, &Function)> = match step {
            Step::Publish(module) => module
                .functions
                .iter()
                .enumerate()
                .filter(|(_, function)| !function.is_struct_api_wrapper())
                .map(|(idx, function)| (Some(idx), function))
                .collect(),
            Step::RunScript(script) => vec![(None, &script.entry)],
            Step::RunCall(_) => Vec::new(),
        };
        for (function_idx, function) in functions {
            // The bounded-recursion prologue precedes the body; editing it could make the
            // recursion unbounded.
            let body_start = crate::recursion::body_entry_index(function);
            for (block_idx, block) in function.body.iter().enumerate().skip(body_start) {
                let protected = loop_control(&block.instrs);
                for (instr_idx, instr) in block.instrs.iter().enumerate() {
                    if !protected[instr_idx] && keep(instr) {
                        sites.push(Site {
                            step: step_idx,
                            function: function_idx,
                            block: block_idx,
                            instr: instr_idx,
                        });
                    }
                }
            }
        }
    }
    sites
}

/// Marks the instructions that bound a generated loop: each run from a read of the loop
/// counter, or from the literal stored into it, through the store back to it. Editing them
/// could make the loop never terminate.
fn loop_control(instrs: &[Instr]) -> Vec<bool> {
    let touches_counter = |instr: &Instr, opcode: &str| {
        instr.opcode == opcode
            && matches!(instr.operands.as_slice(),
                [crate::ir::Operand::Local(name)] if name.as_str() == LOOP_COUNTER_NAME)
    };
    let mut protected = vec![false; instrs.len()];
    let mut run_start: Option<usize> = None;
    for (idx, instr) in instrs.iter().enumerate() {
        if run_start.is_none() && touches_counter(instr, "copy_loc") {
            run_start = Some(idx);
        }
        if touches_counter(instr, "st_loc") {
            // A store without a preceding read is the preheader: its value is the literal
            // just before it.
            let start = run_start.unwrap_or(idx.saturating_sub(1));
            protected[start..=idx]
                .iter_mut()
                .for_each(|slot| *slot = true);
            run_start = None;
        }
    }
    protected
}

fn instruction_at(test: &mut MasmTest, site: Site) -> &mut Instr {
    let function = match (&mut test.steps[site.step], site.function) {
        (Step::Publish(module), Some(idx)) => &mut module.functions[idx],
        (Step::RunScript(script), None) => &mut script.entry,
        _ => unreachable!("sites point at publish functions or script entries"),
    };
    &mut function.body[site.block].instrs[site.instr]
}

fn published_modules(test: &MasmTest) -> impl Iterator<Item = (usize, &Module)> {
    test.steps
        .iter()
        .enumerate()
        .filter_map(|(idx, step)| match step {
            Step::Publish(module) => Some((idx, module)),
            Step::RunScript(_) | Step::RunCall(_) => None,
        })
}

fn choose<'a, T>(u: &mut Unstructured<'_>, items: &'a [T]) -> Result<Option<&'a T>> {
    if items.is_empty() {
        return Ok(None);
    }
    Ok(Some(&items[u.int_in_range(0..=items.len() - 1)?]))
}
