// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Rewrites a test into an equivalent one, for a metamorphic oracle. Both must produce the same
//! statuses, abort codes, return values, and resource writes. Gas, code offsets, function
//! indices, source positions, and whether a task runs out of gas or stack may differ.
//! [`compare_outputs`] checks two runner outputs against these rules.

use crate::{
    error::Result,
    instructions::{
        imm_instr, is_synthesizable_primitive, label_instr, name_instr, simple_op, type_instr,
    },
    ir::{
        Block, BlockId, Function, Instr, Local, MasmTest, Module, Operand, QualifiedFunction, Step,
        Type, Value,
    },
    recursion::body_entry_index,
    types::{is_integer, substitute},
};
use arbitrary::Unstructured;
use move_binary_format::file_format::{FunctionAttribute, Visibility};
use move_core_types::{ability::AbilitySet, identifier::Identifier, int256::U256};
use std::collections::BTreeSet;

/// A semantics-preserving rewrite.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rewrite {
    /// Guards a block with a comparison whose outcome is fixed, branching to an abort on the
    /// opposite outcome. Compares a parameter with itself when possible, otherwise equal literals.
    DeadBranch,
    /// Inserts instructions that push values and pop the result.
    StackNeutral,
    /// Swaps the declaration order of two non-parameter locals.
    LocalRenumber,
    /// Re-expresses a value: a literal as a sum or a widening cast, a local copy as a read
    /// through a reference, a commutative operation with swapped operands, or a `not` folded
    /// into the branch that follows it.
    ReExpress,
    /// Calls a function of another module through a closure of it instead of directly. Both
    /// forms count the callee's module as entered for reentrancy checks.
    ClosureCall,
    /// Calls a function of the same module through a closure of it instead of directly. A
    /// closure call re-enters the module, locking its resources during the call, so this only
    /// applies when the callee and its same-module callees access no global storage.
    LocalClosureCall,
}

impl Rewrite {
    pub const ALL: [Rewrite; 6] = [
        Rewrite::DeadBranch,
        Rewrite::StackNeutral,
        Rewrite::LocalRenumber,
        Rewrite::ReExpress,
        Rewrite::ClosureCall,
        Rewrite::LocalClosureCall,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Rewrite::DeadBranch => "dead_branch",
            Rewrite::StackNeutral => "stack_neutral",
            Rewrite::LocalRenumber => "local_renumber",
            Rewrite::ReExpress => "re_express",
            Rewrite::ClosureCall => "closure_call",
            Rewrite::LocalClosureCall => "local_closure_call",
        }
    }

    pub fn from_name(name: &str) -> Option<Rewrite> {
        Rewrite::ALL
            .into_iter()
            .find(|rewrite| rewrite.name() == name)
    }
}

/// Abort code of the block a dead branch guards against; the low bits number the site.
pub const DEAD_BRANCH_ABORT_BASE: u64 = 0xDEAD_0000_0000;

/// Most sites one variant rewrites.
const SITES_MAX: usize = 4;

/// Bytes drawn from the seed for the rewrite's choices.
const CHOICE_BYTES: usize = 1024;

/// Applies the first rewrite in `enabled` with a site in `test`, starting at a position chosen
/// by `seed`. Deterministic in `test` and `seed`.
pub fn choose_and_apply(
    test: &MasmTest,
    enabled: &[Rewrite],
    seed: u64,
) -> Option<(Rewrite, MasmTest)> {
    if enabled.is_empty() {
        return None;
    }
    let bytes = crate::seeded_bytes(seed, CHOICE_BYTES);
    let start = (seed % enabled.len() as u64) as usize;
    (0..enabled.len()).find_map(|offset| {
        let rewrite = enabled[(start + offset) % enabled.len()];
        apply(test, rewrite, &mut Unstructured::new(&bytes))
            .ok()
            .flatten()
            .map(|variant| (rewrite, variant))
    })
}

/// Applies `rewrite` at one to [`SITES_MAX`] sites of `test`, or returns `None` when it has no
/// site.
pub fn apply(
    test: &MasmTest,
    rewrite: Rewrite,
    u: &mut Unstructured<'_>,
) -> Result<Option<MasmTest>> {
    let mut variant = test.clone();
    let sites = u.int_in_range(1..=SITES_MAX)?;
    let mut applied = 0;
    for site_number in 0..sites {
        let candidates = candidate_sites(&variant, rewrite);
        if candidates.is_empty() {
            break;
        }
        let site = candidates[u.int_in_range(0..=candidates.len() - 1)?];
        match rewrite {
            Rewrite::DeadBranch => insert_dead_branch(
                function_mut(&mut variant, site.step, site.function),
                site.block,
                site_number,
                u,
            )?,
            Rewrite::StackNeutral => insert_stack_neutral(
                &mut function_mut(&mut variant, site.step, site.function).body[site.block].instrs,
                site.instr,
                u,
            )?,
            Rewrite::LocalRenumber => {
                swap_locals(function_mut(&mut variant, site.step, site.function), u)?
            },
            Rewrite::ReExpress => re_express(
                function_mut(&mut variant, site.step, site.function),
                site.block,
                site.instr,
                u,
            )?,
            Rewrite::ClosureCall | Rewrite::LocalClosureCall => {
                let target = closure_target(&variant, site, rewrite == Rewrite::LocalClosureCall)
                    .expect("candidate sites have closure targets");
                call_through_closure(&mut variant, site, target);
            },
        }
        applied += 1;
    }
    Ok((applied > 0).then_some(variant))
}

/// How a variant's runner output relates to the original's.
#[derive(Debug, Eq, PartialEq)]
pub enum Agreement {
    Same,
    /// Exactly one side exhausted gas, the operand stack or the call stack at `task`, which a
    /// rewrite can cause. Later tasks are not compared: their storage may differ.
    Inconclusive {
        task: usize,
    },
    /// A publish the original accepted fails in the variant: the rewrite produced invalid code.
    VariantRejected {
        task: usize,
        original: String,
        variant: String,
    },
    Differ {
        task: usize,
        original: String,
        variant: String,
    },
}

/// Compares the runner outputs of a test and a rewritten variant of it, task by task.
pub fn compare_outputs(original: &str, variant: &str) -> Agreement {
    let original_tasks = task_outcomes(original);
    let variant_tasks = task_outcomes(variant);
    for task in 0..original_tasks.len().max(variant_tasks.len()) {
        let (Some(left), Some(right)) = (original_tasks.get(task), variant_tasks.get(task)) else {
            return Agreement::Differ {
                task,
                original: original_tasks
                    .get(task)
                    .map_or_else(String::new, |outcome| outcome.text.clone()),
                variant: variant_tasks
                    .get(task)
                    .map_or_else(String::new, |outcome| outcome.text.clone()),
            };
        };
        if left.text == right.text {
            continue;
        }
        match (left.is_exhausted(), right.is_exhausted()) {
            (true, true) => continue,
            (true, false) | (false, true) => return Agreement::Inconclusive { task },
            (false, false) => {},
        }
        if right.is_publish && right.failed && !left.failed {
            return Agreement::VariantRejected {
                task,
                original: left.text.clone(),
                variant: right.text.clone(),
            };
        }
        return Agreement::Differ {
            task,
            original: left.text.clone(),
            variant: right.text.clone(),
        };
    }
    Agreement::Same
}

/// Statuses a rewrite may legitimately change, since it changes gas use and stack depth.
const EXHAUSTION_STATUSES: [&str; 3] = [
    "OUT_OF_GAS",
    "EXECUTION_STACK_OVERFLOW",
    "CALL_STACK_OVERFLOW",
];

/// The parts of one task's output a rewrite must preserve.
struct TaskOutcome {
    is_publish: bool,
    failed: bool,
    status: Option<String>,
    text: String,
}

impl TaskOutcome {
    fn is_exhausted(&self) -> bool {
        self.status
            .as_deref()
            .is_some_and(|status| EXHAUSTION_STATUSES.contains(&status))
    }
}

/// Splits runner output into tasks, keeping everything except gas, code offsets, function
/// indices, VM messages, execution state and the source excerpts of diagnostics.
fn task_outcomes(output: &str) -> Vec<TaskOutcome> {
    const ERROR_FIELDS: [&str; 3] = ["major_status: ", "sub_status: ", "location: "];
    let mut tasks: Vec<TaskOutcome> = Vec::new();
    let mut in_error_record = false;
    let mut in_diagnostic = false;
    for line in output.lines() {
        if let Some(header) = line.strip_prefix("task ") {
            let command = header.split_once(": ").map_or("", |(_, command)| command);
            tasks.push(TaskOutcome {
                is_publish: command.trim_start().starts_with("publish"),
                failed: false,
                status: None,
                text: String::new(),
            });
            in_error_record = false;
            in_diagnostic = false;
            continue;
        }
        let Some(task) = tasks.last_mut() else {
            continue;
        };
        if line.starts_with("gas used: ") {
            continue;
        }
        if line.starts_with("Error: ") {
            task.failed = true;
            in_error_record = line.ends_with('{');
            in_diagnostic = !in_error_record;
        } else if in_diagnostic {
            // Excerpts carry source line numbers, which a rewrite shifts; headlines do not.
            if !line.starts_with(|first: char| first.is_ascii_alphabetic()) {
                continue;
            }
        } else if in_error_record {
            let field = line.trim_start();
            if line == "}" {
                in_error_record = false;
            } else if let Some(status) = field.strip_prefix("major_status: ") {
                task.status = Some(status.trim_end_matches(',').to_owned());
            } else if !ERROR_FIELDS.iter().any(|prefix| field.starts_with(prefix)) {
                continue;
            }
        }
        task.text.push_str(line);
        task.text.push('\n');
    }
    tasks
}

/// Location of a rewrite: a step, a function in it (`None` for a script's entry), and a block
/// and instruction in that function where the rewrite needs them.
#[derive(Clone, Copy)]
struct Site {
    step: usize,
    function: Option<usize>,
    block: usize,
    instr: usize,
}

fn candidate_sites(test: &MasmTest, rewrite: Rewrite) -> Vec<Site> {
    let mut sites = Vec::new();
    for (step, function_idx, function) in rewritable_functions(test) {
        let site = |block, instr| Site {
            step,
            function: function_idx,
            block,
            instr,
        };
        let body_start = body_entry_index(function);
        match rewrite {
            Rewrite::DeadBranch => {
                sites.extend((body_start..function.body.len()).map(|block| site(block, 0)))
            },
            Rewrite::StackNeutral => {
                for (block_idx, block) in function.body.iter().enumerate().skip(body_start) {
                    sites.extend((0..block.instrs.len()).map(|instr| site(block_idx, instr)));
                }
            },
            Rewrite::LocalRenumber => {
                if function.locals.len() >= 2 {
                    sites.push(site(0, 0));
                }
            },
            Rewrite::ReExpress => {
                for (block_idx, block) in function.body.iter().enumerate().skip(body_start) {
                    sites.extend(
                        (0..block.instrs.len())
                            .filter(|&instr| {
                                !re_expressions(function, &block.instrs, instr).is_empty()
                            })
                            .map(|instr| site(block_idx, instr)),
                    );
                }
            },
            Rewrite::ClosureCall | Rewrite::LocalClosureCall => {
                let local = rewrite == Rewrite::LocalClosureCall;
                for (block_idx, block) in function.body.iter().enumerate().skip(body_start) {
                    sites.extend(
                        (0..block.instrs.len())
                            .map(|instr| site(block_idx, instr))
                            .filter(|&site| closure_target(test, site, local).is_some()),
                    );
                }
            },
        }
    }
    sites
}

/// Functions whose bodies may be rewritten: module functions other than natives and struct
/// API wrappers, whose bodies have a shape the verifier checks exactly, and script entries.
fn rewritable_functions(test: &MasmTest) -> Vec<(usize, Option<usize>, &Function)> {
    let mut functions = Vec::new();
    for (step_idx, step) in test.steps.iter().enumerate() {
        match step {
            Step::Publish(module) => functions.extend(
                module
                    .functions
                    .iter()
                    .enumerate()
                    .filter(|(_, function)| {
                        !function.is_native && !function.is_struct_api_wrapper()
                    })
                    .map(|(idx, function)| (step_idx, Some(idx), function)),
            ),
            Step::RunScript(script) => functions.push((step_idx, None, &script.entry)),
            Step::RunCall(_) => {},
        }
    }
    functions
}

fn function_ref(test: &MasmTest, step: usize, function: Option<usize>) -> &Function {
    match (&test.steps[step], function) {
        (Step::Publish(module), Some(idx)) => &module.functions[idx],
        (Step::RunScript(script), None) => &script.entry,
        _ => unreachable!("sites point at module functions or script entries"),
    }
}

fn function_mut(test: &mut MasmTest, step: usize, function: Option<usize>) -> &mut Function {
    match (&mut test.steps[step], function) {
        (Step::Publish(module), Some(idx)) => &mut module.functions[idx],
        (Step::RunScript(script), None) => &mut script.entry,
        _ => unreachable!("sites point at module functions or script entries"),
    }
}

/// A `call` that can become a closure call.
struct ClosureTarget {
    function: QualifiedFunction,
    type_args: Vec<Type>,
    closure: Type,
    /// Every published version of the callee, as (step, function index), so all of them get
    /// the `persistent` attribute and republishes stay compatible.
    versions: Vec<(usize, usize)>,
}

/// The callee of the instruction at `site` if it is a `call` that can go through a closure:
/// to another module's public function (`local` false) or to a function of the same module
/// (`local` true). The callee needs a closure-typed signature, which is the same in every
/// version of its module, and no `acquires`. A same-module callee also needs no global
/// storage access in itself or its same-module callees, since the closure call re-enters the
/// module.
fn closure_target(test: &MasmTest, site: Site, local: bool) -> Option<ClosureTarget> {
    let caller = function_ref(test, site.step, site.function);
    let instr = &caller.body.get(site.block)?.instrs.get(site.instr)?;
    let [Operand::Function(function, type_args)] = instr.operands.as_slice() else {
        return None;
    };
    if instr.opcode != "call" {
        return None;
    }
    let caller_module = match &test.steps[site.step] {
        Step::Publish(module) => Some(module),
        Step::RunScript(_) | Step::RunCall(_) => None,
    };
    if caller_module.is_some_and(|module| module.id == function.module) != local {
        return None;
    }
    let mut versions = Vec::new();
    let mut signature: Option<&Function> = None;
    for (step_idx, step) in test.steps.iter().enumerate() {
        let Step::Publish(module) = step else {
            continue;
        };
        if module.id != function.module {
            continue;
        }
        let (index, callee) = module
            .functions
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.name == function.name)?;
        if !closure_callable(callee, type_args, local)
            || signature.is_some_and(|first| !same_signature(first, callee))
        {
            return None;
        }
        signature = Some(callee);
        versions.push((step_idx, index));
    }
    let callee = signature?;
    if let Some(module) = caller_module.filter(|_| local) {
        if accesses_globals(module, &function.name) {
            return None;
        }
    }
    let concrete = |types: &[Type]| -> Vec<Type> {
        types.iter().map(|ty| substitute(ty, type_args)).collect()
    };
    let params: Vec<Type> = callee.params.iter().map(|param| param.ty.clone()).collect();
    Some(ClosureTarget {
        function: function.clone(),
        type_args: type_args.clone(),
        closure: Type::Function {
            args: concrete(&params),
            results: concrete(&callee.return_types),
            abilities: AbilitySet::PUBLIC_FUNCTIONS,
        },
        versions,
    })
}

/// Whether `callee` can be packed as a closure with `type_args` and called from where a direct
/// call to it was valid: the shapes the generator's own closures use.
fn closure_callable(callee: &Function, type_args: &[Type], local: bool) -> bool {
    let primitive_after_substitution =
        |ty: &Type| is_synthesizable_primitive(&substitute(ty, type_args));
    (local || callee.visibility == Visibility::Public)
        && !callee.is_native
        && !callee.is_entry
        && !callee.is_struct_api_wrapper()
        && callee.acquires.is_empty()
        && callee.params.len() <= 4
        && !callee.return_types.is_empty()
        && callee
            .params
            .iter()
            .all(|param| primitive_after_substitution(&param.ty))
        && callee.return_types.iter().all(primitive_after_substitution)
}

fn same_signature(left: &Function, right: &Function) -> bool {
    left.visibility == right.visibility
        && left.type_params.len() == right.type_params.len()
        && left
            .params
            .iter()
            .map(|param| &param.ty)
            .eq(right.params.iter().map(|param| &param.ty))
        && left.return_types == right.return_types
}

/// Opcodes accessing global storage.
const GLOBAL_OPS: [&str; 5] = [
    "borrow_global",
    "mut_borrow_global",
    "exists",
    "move_from",
    "move_to",
];

/// Whether `name` or a function it calls statically in `module`, transitively, accesses global
/// storage.
fn accesses_globals(module: &Module, name: &Identifier) -> bool {
    let mut visited = BTreeSet::new();
    let mut pending = vec![name.clone()];
    while let Some(current) = pending.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == current)
        else {
            continue;
        };
        for instr in function.body.iter().flat_map(|block| &block.instrs) {
            if GLOBAL_OPS.contains(&instr.opcode.as_str()) {
                return true;
            }
            if instr.opcode == "call" {
                if let [Operand::Function(callee, _)] = instr.operands.as_slice() {
                    if callee.module == module.id {
                        pending.push(callee.name.clone());
                    }
                }
            }
        }
    }
    false
}

/// Replaces the `call` at `site` with `pack_closure` of the callee and `call_closure`, and marks
/// every version of the callee `persistent`, which closure targets need.
fn call_through_closure(test: &mut MasmTest, site: Site, target: ClosureTarget) {
    for (step, index) in &target.versions {
        let Step::Publish(module) = &mut test.steps[*step] else {
            unreachable!("versions are publish steps");
        };
        let attributes = &mut module.functions[*index].attributes;
        if !attributes
            .iter()
            .any(|attribute| matches!(attribute, FunctionAttribute::Persistent))
        {
            attributes.push(FunctionAttribute::Persistent);
        }
    }
    let function = function_mut(test, site.step, site.function);
    function.body[site.block]
        .instrs
        .splice(site.instr..=site.instr, [
            Instr {
                opcode: "pack_closure".to_string(),
                operands: vec![
                    Operand::Function(target.function, target.type_args),
                    Operand::Imm(Value::U64(0)),
                ],
            },
            type_instr("call_closure", target.closure),
        ]);
}

/// Splits block `block` into a guard and the original instructions, with the guard branching
/// to a new aborting block when its predicate fails. The predicate always holds.
fn insert_dead_branch(
    function: &mut Function,
    block: usize,
    site_number: usize,
    u: &mut Unstructured<'_>,
) -> Result<()> {
    let (mut guard, always_true) = predicate(function, u)?;
    let next_id = function
        .body
        .iter()
        .map(|block| block.id.0)
        .max()
        .unwrap_or(0)
        + 1;
    let (rest, dead) = (BlockId(next_id), BlockId(next_id + 1));
    guard.push(label_instr(
        if always_true { "br_false" } else { "br_true" },
        dead,
    ));
    guard.push(label_instr("branch", rest));
    let original = std::mem::replace(&mut function.body[block].instrs, guard);
    function.body.insert(block + 1, Block {
        id: rest,
        instrs: original,
    });
    function.body.push(Block {
        id: dead,
        instrs: vec![
            imm_instr(
                "ld_u64",
                Value::U64(DEAD_BRANCH_ABORT_BASE | site_number as u64),
            ),
            simple_op("abort"),
        ],
    });
    Ok(())
}

/// Instructions computing a predicate whose value is fixed, and that value. It compares a
/// parameter with itself where one can be copied anywhere in the body (never moved or mutably
/// borrowed), and two equal literals otherwise.
fn predicate(function: &Function, u: &mut Unstructured<'_>) -> Result<(Vec<Instr>, bool)> {
    let copyable: Vec<&Local> = function
        .params
        .iter()
        .filter(|param| {
            is_synthesizable_primitive(&param.ty)
                && !is_moved_or_mutably_borrowed(function, &param.name)
        })
        .collect();
    if copyable.is_empty() {
        let value = u.int_in_range(0..=u64::from(u32::MAX))?;
        return Ok((
            vec![
                imm_instr("ld_u64", Value::U64(value)),
                imm_instr("ld_u64", Value::U64(value)),
                simple_op("eq"),
            ],
            true,
        ));
    }
    let param = copyable[u.int_in_range(0..=copyable.len() - 1)?];
    // Integers also support ordering, which is irreflexive for `lt` and reflexive for `ge`.
    let comparisons: &[(&str, bool)] = if is_integer(&param.ty) {
        &[("eq", true), ("neq", false), ("lt", false), ("ge", true)]
    } else {
        &[("eq", true), ("neq", false)]
    };
    let (comparison, always_true) = comparisons[u.int_in_range(0..=comparisons.len() - 1)?];
    Ok((
        vec![
            name_instr("copy_loc", param.name.clone()),
            name_instr("copy_loc", param.name.clone()),
            simple_op(comparison),
        ],
        always_true,
    ))
}

fn is_moved_or_mutably_borrowed(function: &Function, local: &Identifier) -> bool {
    function
        .body
        .iter()
        .flat_map(|block| &block.instrs)
        .any(|instr| {
            matches!(instr.opcode.as_str(), "move_loc" | "mut_borrow_loc")
                && matches!(instr.operands.as_slice(), [Operand::Local(name)] if name == local)
        })
}

/// Inserts, before instruction `index`, instructions that leave the stack as it was.
fn insert_stack_neutral(
    instrs: &mut Vec<Instr>,
    index: usize,
    u: &mut Unstructured<'_>,
) -> Result<()> {
    let small = |u: &mut Unstructured<'_>| u.int_in_range(0..=u64::from(u32::MAX));
    let sequence = match u.int_in_range(0..=3)? {
        0 => vec![imm_instr("ld_u64", Value::U64(small(u)?)), simple_op("pop")],
        1 => vec![
            simple_op(
                if u.arbitrary()? {
                    "ld_true"
                } else {
                    "ld_false"
                },
            ),
            simple_op("pop"),
        ],
        2 => vec![
            imm_instr("ld_u8", Value::U8(u.arbitrary()?)),
            simple_op("cast_u64"),
            simple_op("pop"),
        ],
        _ => vec![
            imm_instr("ld_u64", Value::U64(small(u)?)),
            imm_instr("ld_u64", Value::U64(small(u)?)),
            simple_op("add"),
            simple_op("pop"),
        ],
    };
    instrs.splice(index..index, sequence);
    Ok(())
}

/// Swaps two distinct non-parameter locals in the declaration order, renumbering them.
fn swap_locals(function: &mut Function, u: &mut Unstructured<'_>) -> Result<()> {
    let count = function.locals.len();
    let first = u.int_in_range(0..=count - 1)?;
    let offset = u.int_in_range(1..=count - 1)?;
    function.locals.swap(first, (first + offset) % count);
    Ok(())
}

/// A way to re-express the instruction at a position.
#[derive(Clone, Copy)]
enum ReExpression {
    /// `ld_uN c` as `ld_uN a; ld_uN b; add` with `a + b = c`.
    SplitLiteral,
    /// `ld_uN c` as `ld_u256 c; cast_uN`.
    WidenLiteral,
    /// `copy_loc x` as `borrow_loc x; read_ref`, for a primitive local.
    BorrowRead,
    /// `a; b; op` as `b; a; op'`, where `a` and `b` push without side effects and `op'` is
    /// `op`, or its mirror for an ordering.
    SwapOperands,
    /// `not; br_false L` as `br_true L`, and `not; br_true L` as `br_false L`.
    FoldNot,
}

fn re_expressions(function: &Function, instrs: &[Instr], index: usize) -> Vec<ReExpression> {
    let instr = &instrs[index];
    let mut found = Vec::new();
    if UnsignedLiteral::of(instr).is_some() {
        found.push(ReExpression::SplitLiteral);
        found.push(ReExpression::WidenLiteral);
    }
    if instr.opcode == "copy_loc" {
        if let [Operand::Local(name)] = instr.operands.as_slice() {
            let ty = function
                .params
                .iter()
                .chain(&function.locals)
                .find(|local| &local.name == name)
                .map(|local| &local.ty);
            if ty.is_some_and(is_synthesizable_primitive) {
                found.push(ReExpression::BorrowRead);
            }
        }
    }
    if let [first, second, operator] = &instrs[index..instrs.len().min(index + 3)] {
        if pushes_without_effects(first)
            && pushes_without_effects(second)
            && operator.operands.is_empty()
            && swapped_operator(&operator.opcode).is_some()
        {
            found.push(ReExpression::SwapOperands);
        }
    }
    if instr.opcode == "not"
        && instrs
            .get(index + 1)
            .is_some_and(|next| matches!(next.opcode.as_str(), "br_true" | "br_false"))
    {
        found.push(ReExpression::FoldNot);
    }
    found
}

fn re_express(
    function: &mut Function,
    block: usize,
    index: usize,
    u: &mut Unstructured<'_>,
) -> Result<()> {
    let options = re_expressions(function, &function.body[block].instrs, index);
    let choice = options[u.int_in_range(0..=options.len() - 1)?];
    let instrs = &mut function.body[block].instrs;
    match choice {
        ReExpression::SplitLiteral => {
            let literal = UnsignedLiteral::of(&instrs[index]).expect("checked by re_expressions");
            let first = u.int_in_range(0..=literal.value)?;
            instrs.splice(index..=index, [
                literal.load(first),
                literal.load(literal.value - first),
                simple_op("add"),
            ]);
        },
        ReExpression::WidenLiteral => {
            let literal = UnsignedLiteral::of(&instrs[index]).expect("checked by re_expressions");
            let mut bytes = [0u8; 32];
            bytes[..16].copy_from_slice(&literal.value.to_le_bytes());
            instrs.splice(index..=index, [
                imm_instr("ld_u256", Value::U256(U256::from_le_bytes(bytes))),
                simple_op(literal.width.cast_opcode()),
            ]);
        },
        ReExpression::BorrowRead => {
            let borrow = Instr {
                opcode: "borrow_loc".to_string(),
                operands: instrs[index].operands.clone(),
            };
            instrs.splice(index..=index, [borrow, simple_op("read_ref")]);
        },
        ReExpression::SwapOperands => {
            instrs.swap(index, index + 1);
            let operator = &mut instrs[index + 2];
            operator.opcode = swapped_operator(&operator.opcode)
                .expect("checked by re_expressions")
                .to_string();
        },
        ReExpression::FoldNot => {
            instrs.remove(index);
            let branch = &mut instrs[index];
            branch.opcode = if branch.opcode == "br_true" {
                "br_false"
            } else {
                "br_true"
            }
            .to_string();
        },
    }
    Ok(())
}

/// The operator computing the same result with its operands swapped.
fn swapped_operator(opcode: &str) -> Option<&'static str> {
    Some(match opcode {
        "add" => "add",
        "mul" => "mul",
        "bit_and" => "bit_and",
        "bit_or" => "bit_or",
        "xor" => "xor",
        "eq" => "eq",
        "neq" => "neq",
        "and" => "and",
        "or" => "or",
        "lt" => "gt",
        "gt" => "lt",
        "le" => "ge",
        "ge" => "le",
        _ => return None,
    })
}

/// Literal loads and local copies, which cannot fail and whose order does not matter.
fn pushes_without_effects(instr: &Instr) -> bool {
    instr.opcode.starts_with("ld_") || instr.opcode == "copy_loc"
}

#[derive(Clone, Copy)]
enum UnsignedWidth {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl UnsignedWidth {
    fn load_opcode(self) -> &'static str {
        match self {
            UnsignedWidth::U8 => "ld_u8",
            UnsignedWidth::U16 => "ld_u16",
            UnsignedWidth::U32 => "ld_u32",
            UnsignedWidth::U64 => "ld_u64",
            UnsignedWidth::U128 => "ld_u128",
        }
    }

    fn cast_opcode(self) -> &'static str {
        match self {
            UnsignedWidth::U8 => "cast_u8",
            UnsignedWidth::U16 => "cast_u16",
            UnsignedWidth::U32 => "cast_u32",
            UnsignedWidth::U64 => "cast_u64",
            UnsignedWidth::U128 => "cast_u128",
        }
    }

    /// `value`, which fits this width, as a literal of it.
    fn value(self, value: u128) -> Value {
        let narrow = "literal fits the width it came from";
        match self {
            UnsignedWidth::U8 => Value::U8(value.try_into().expect(narrow)),
            UnsignedWidth::U16 => Value::U16(value.try_into().expect(narrow)),
            UnsignedWidth::U32 => Value::U32(value.try_into().expect(narrow)),
            UnsignedWidth::U64 => Value::U64(value.try_into().expect(narrow)),
            UnsignedWidth::U128 => Value::U128(value),
        }
    }
}

/// An unsigned literal load of at most 128 bits.
struct UnsignedLiteral {
    width: UnsignedWidth,
    value: u128,
}

impl UnsignedLiteral {
    fn of(instr: &Instr) -> Option<UnsignedLiteral> {
        let [Operand::Imm(value)] = instr.operands.as_slice() else {
            return None;
        };
        let (width, value) = match (instr.opcode.as_str(), value) {
            ("ld_u8", Value::U8(value)) => (UnsignedWidth::U8, u128::from(*value)),
            ("ld_u16", Value::U16(value)) => (UnsignedWidth::U16, u128::from(*value)),
            ("ld_u32", Value::U32(value)) => (UnsignedWidth::U32, u128::from(*value)),
            ("ld_u64", Value::U64(value)) => (UnsignedWidth::U64, u128::from(*value)),
            ("ld_u128", Value::U128(value)) => (UnsignedWidth::U128, *value),
            _ => return None,
        };
        Some(UnsignedLiteral { width, value })
    }

    fn load(&self, value: u128) -> Instr {
        imm_instr(self.width.load_opcode(), self.width.value(value))
    }
}
