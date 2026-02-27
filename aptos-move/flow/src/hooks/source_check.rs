// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! PostToolUse hook: syntactic checks and formatting for Move source files.
//!
//! Triggered after Edit/Write tool calls on `.move` files. Reads the hook
//! JSON from stdin, extracts the file path, and runs fast offline checks
//! (no full compilation). Outputs diagnostics to stdout; silent when clean.
//! Exits 2 when any diagnostic is found (including deprecation warnings),
//! 0 otherwise.
//!
//! ## Checks performed
//!
//! 1. **Parse check** — runs the Move parser and reports syntax errors.
//! 2. **AST checks** (only when parsing succeeds):
//!    - `old()` called outside `ensures`, update invariants, or loop invariants.
//!    - `old(expr)` in loop invariants where `expr` is not a simple parameter name.
//!    - Dereference (`*e`) inside spec expressions.
//!    - Borrow (`&e`) inside spec expressions.
//! 3. **Text checks** (always run) — flags deprecated Move 1 syntax:
//!    - `borrow_global<` (use `&T[addr]`).
//!    - `borrow_global_mut<` (use `&mut T[addr]`).
//!    - `acquires` annotations (no longer needed).
//! 4. **Auto-format** — when no errors are found, runs `movefmt` in-place
//!    (silently skipped if `movefmt` is not installed).

use anyhow::Result;
use legacy_move_compiler::{
    diagnostics::{
        codes::{TypeSafety, Uncategorized},
        report_diagnostics_to_buffer, Diagnostic, Diagnostics, FilesSourceText,
    },
    parser::{
        ast::{
            Definition, Exp, Exp_, FunctionBody_, ModuleDefinition, ModuleMember, NameAccessChain_,
            Sequence, SequenceItem_, SpecBlock, SpecBlockMember_, SpecBlockTarget_,
            SpecConditionKind_,
        },
        syntax::parse_file_string,
    },
    shared::{CompilationEnv, Flags, LanguageVersion},
};
use move_command_line_common::files::FileHash;
use move_ir_types::location::Loc;
use move_symbol_pool::Symbol;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf, process::Command};

/// JSON shape of the PostToolUse hook input (only the fields we need).
#[derive(Deserialize)]
struct HookInput {
    tool_input: ToolInput,
}

#[derive(Deserialize)]
struct ToolInput {
    file_path: Option<String>,
}

/// Result of running checks on a Move source file.
pub(crate) struct CheckResult {
    pub output: String,
    /// Any diagnostic was emitted (parse errors, AST warnings, deprecation).
    pub has_errors: bool,
    /// The source file failed to parse (used to skip formatting).
    pub has_parse_errors: bool,
}

/// Entry point: reads stdin JSON, runs checks, prints output.
pub fn run() -> Result<()> {
    move_compiler_v2::logging::setup_logging(None);
    log::info!("edit hook invoked");

    let input: HookInput = match serde_json::from_reader(std::io::stdin()) {
        Ok(v) => v,
        Err(_) => {
            log::info!("edit hook: malformed input, skipping");
            return Ok(());
        },
    };

    let path = match input.tool_input.file_path {
        Some(p) if p.ends_with(".move") => p,
        _ => {
            log::info!("edit hook: not a .move file, skipping");
            return Ok(());
        },
    };

    log::info!("edit hook: checking `{}`", path);

    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            log::info!("edit hook: cannot read `{}`: {}", path, e);
            return Ok(());
        },
    };

    let result = check(&path, &source);
    if !result.has_parse_errors {
        format_file(&path);
    }
    if result.output.is_empty() {
        log::info!("edit hook: `{}` is clean", path);
    } else {
        log::info!(
            "edit hook: `{}` has diagnostics (has_errors={}):\n{}",
            path,
            result.has_errors,
            result.output
        );
        print!("{}", result.output);
    }
    if result.has_errors {
        std::process::exit(2);
    }
    Ok(())
}

/// Core check logic (testable without stdin/filesystem).
pub(crate) fn check(path: &str, source: &str) -> CheckResult {
    let file_hash = FileHash::new(source);
    let file_name = Symbol::from(path);
    let mut files: FilesSourceText = HashMap::new();
    files.insert(file_hash, (file_name, source.to_string()));

    let (definitions, parse_diags, has_parse_errors) = parse_check(path, source, file_hash);

    let mut all_diags = parse_diags;

    // AST checks only if parse succeeded
    if let Some(defs) = &definitions {
        let ast_diags = ast_checks(defs, file_hash);
        all_diags.extend(ast_diags);
    }

    // Text checks always run
    let text_diags = text_checks(source, file_hash);
    all_diags.extend(text_diags);

    let has_errors = has_parse_errors || !all_diags.is_empty();

    if all_diags.is_empty() {
        return CheckResult {
            output: String::new(),
            has_errors,
            has_parse_errors,
        };
    }

    let buf = report_diagnostics_to_buffer(&files, all_diags);
    let output = String::from_utf8_lossy(&buf).to_string();
    CheckResult {
        output,
        has_errors,
        has_parse_errors,
    }
}

/// Parse the source file and return (definitions, diagnostics, has_errors).
fn parse_check(
    _path: &str,
    source: &str,
    file_hash: FileHash,
) -> (Option<Vec<Definition>>, Diagnostics, bool) {
    let known_attributes = aptos_framework::extended_checks::get_all_attribute_names().clone();
    let flags = Flags::empty().set_language_version(LanguageVersion::V2_5);
    let mut env = CompilationEnv::new(flags, known_attributes);

    match parse_file_string(&mut env, file_hash, source) {
        Ok((defs, _comments)) => {
            // Collect any warnings the parser emitted (e.g. unknown attributes)
            let mut diags = Diagnostics::new();
            if let Err(env_diags) = env.check_diags_at_or_above_severity(
                legacy_move_compiler::diagnostics::codes::Severity::Warning,
            ) {
                diags.extend(env_diags);
            }
            (Some(defs), diags, false)
        },
        Err(parse_diags) => {
            // Also include any env diagnostics accumulated before the error
            let mut diags = parse_diags;
            if let Err(env_diags) = env.check_diags_at_or_above_severity(
                legacy_move_compiler::diagnostics::codes::Severity::Warning,
            ) {
                diags.extend(env_diags);
            }
            (None, diags, true)
        },
    }
}

// ─── AST checks ──────────────────────────────────────────────────────────────

/// Context for tracking which spec condition we are inside.
enum SpecContext {
    Ensures,
    InvariantUpdate,
    LoopInvariant,
    Other,
}

/// Walk the parsed AST to check spec expression rules.
fn ast_checks(definitions: &[Definition], file_hash: FileHash) -> Diagnostics {
    let mut diags = Diagnostics::new();
    for def in definitions {
        match def {
            Definition::Module(module) => walk_module(module, file_hash, &mut diags),
            Definition::Address(addr_def) => {
                for module in &addr_def.modules {
                    walk_module(module, file_hash, &mut diags);
                }
            },
            Definition::Script(script) => {
                walk_function_body(&script.function.body.value, &mut diags);
                for spec_block in &script.specs {
                    walk_spec_block(spec_block, &mut diags);
                }
            },
        }
    }
    diags
}

fn walk_module(module: &ModuleDefinition, _file_hash: FileHash, diags: &mut Diagnostics) {
    for member in &module.members {
        match member {
            ModuleMember::Spec(spec_block) => walk_spec_block(spec_block, diags),
            ModuleMember::Function(func) => {
                walk_function_body(&func.body.value, diags);
            },
            _ => {},
        }
    }
}

fn walk_function_body(body: &FunctionBody_, diags: &mut Diagnostics) {
    if let FunctionBody_::Defined(seq) = body {
        walk_sequence_for_spec_blocks(seq, diags);
    }
}

fn walk_sequence_for_spec_blocks(seq: &Sequence, diags: &mut Diagnostics) {
    // Walk sequence items
    for item in &seq.1 {
        match &item.value {
            SequenceItem_::Seq(exp) => walk_exp_for_spec_blocks(exp, diags),
            SequenceItem_::Bind(_, _, exp) => walk_exp_for_spec_blocks(exp, diags),
            SequenceItem_::Declare(_, _) => {},
        }
    }
    // Walk trailing expression
    if let Some(exp) = seq.3.as_ref() {
        walk_exp_for_spec_blocks(exp, diags);
    }
}

/// Walk an expression tree looking for inline `spec { ... }` blocks.
fn walk_exp_for_spec_blocks(exp: &Exp, diags: &mut Diagnostics) {
    match &exp.value {
        Exp_::Spec(spec_block) => walk_spec_block(spec_block, diags),
        Exp_::Block(seq) => walk_sequence_for_spec_blocks(seq, diags),
        Exp_::IfElse(c, t, e) => {
            walk_exp_for_spec_blocks(c, diags);
            walk_exp_for_spec_blocks(t, diags);
            if let Some(e) = e {
                walk_exp_for_spec_blocks(e, diags);
            }
        },
        Exp_::While(_, c, body) => {
            walk_exp_for_spec_blocks(c, diags);
            walk_exp_for_spec_blocks(body, diags);
        },
        Exp_::Loop(_, body) => walk_exp_for_spec_blocks(body, diags),
        Exp_::BinopExp(l, _, r) => {
            walk_exp_for_spec_blocks(l, diags);
            walk_exp_for_spec_blocks(r, diags);
        },
        Exp_::UnaryExp(_, e) => walk_exp_for_spec_blocks(e, diags),
        Exp_::Dereference(e) => walk_exp_for_spec_blocks(e, diags),
        Exp_::Borrow(_, e) => walk_exp_for_spec_blocks(e, diags),
        Exp_::Dot(e, _) => walk_exp_for_spec_blocks(e, diags),
        Exp_::Index(e1, e2) => {
            walk_exp_for_spec_blocks(e1, diags);
            walk_exp_for_spec_blocks(e2, diags);
        },
        Exp_::Cast(e, _) | Exp_::Annotate(e, _) => walk_exp_for_spec_blocks(e, diags),
        Exp_::Call(_, _, _, args) => {
            for arg in &args.value {
                walk_exp_for_spec_blocks(arg, diags);
            }
        },
        Exp_::ExpList(es) => {
            for e in es {
                walk_exp_for_spec_blocks(e, diags);
            }
        },
        Exp_::Assign(l, _, r) => {
            walk_exp_for_spec_blocks(l, diags);
            walk_exp_for_spec_blocks(r, diags);
        },
        Exp_::Return(Some(e)) | Exp_::Abort(e) => walk_exp_for_spec_blocks(e, diags),
        Exp_::Lambda(_, e, _, _) => walk_exp_for_spec_blocks(e, diags),
        Exp_::Match(e, arms) => {
            walk_exp_for_spec_blocks(e, diags);
            for arm in arms {
                if let Some(guard) = &arm.value.1 {
                    walk_exp_for_spec_blocks(guard, diags);
                }
                walk_exp_for_spec_blocks(&arm.value.2, diags);
            }
        },
        _ => {},
    }
}

fn walk_spec_block(block: &SpecBlock, diags: &mut Diagnostics) {
    let is_code_block = matches!(block.value.target.value, SpecBlockTarget_::Code);
    for member in &block.value.members {
        match &member.value {
            SpecBlockMember_::Condition {
                kind,
                exp,
                additional_exps,
                ..
            } => {
                let ctx = match &kind.value {
                    SpecConditionKind_::Ensures => SpecContext::Ensures,
                    SpecConditionKind_::InvariantUpdate(_) => SpecContext::InvariantUpdate,
                    SpecConditionKind_::Invariant(_) if is_code_block => SpecContext::LoopInvariant,
                    _ => SpecContext::Other,
                };
                walk_spec_exp(exp, &ctx, diags);
                for e in additional_exps {
                    walk_spec_exp(e, &ctx, diags);
                }
            },
            SpecBlockMember_::Update { lhs, rhs } => {
                walk_spec_exp(lhs, &SpecContext::InvariantUpdate, diags);
                walk_spec_exp(rhs, &SpecContext::InvariantUpdate, diags);
            },
            SpecBlockMember_::Let {
                def, post_state, ..
            } => {
                let ctx = if *post_state {
                    SpecContext::Ensures
                } else {
                    SpecContext::Other
                };
                walk_spec_exp(def, &ctx, diags);
            },
            SpecBlockMember_::Include { exp, .. } => {
                walk_spec_exp(exp, &SpecContext::Other, diags);
            },
            SpecBlockMember_::Apply { exp, .. } => {
                walk_spec_exp(exp, &SpecContext::Other, diags);
            },
            SpecBlockMember_::Variable { init, .. } => {
                if let Some(init_exp) = init {
                    walk_spec_exp(init_exp, &SpecContext::Other, diags);
                }
            },
            SpecBlockMember_::Function { body, .. } => {
                if let FunctionBody_::Defined(seq) = &body.value {
                    walk_spec_sequence(seq, &SpecContext::Other, diags);
                }
            },
            SpecBlockMember_::Pragma { .. } | SpecBlockMember_::Proof { .. } => {},
        }
    }
}

fn walk_spec_sequence(seq: &Sequence, ctx: &SpecContext, diags: &mut Diagnostics) {
    for item in &seq.1 {
        match &item.value {
            SequenceItem_::Seq(exp) => walk_spec_exp(exp, ctx, diags),
            SequenceItem_::Bind(_, _, exp) => walk_spec_exp(exp, ctx, diags),
            SequenceItem_::Declare(_, _) => {},
        }
    }
    if let Some(exp) = seq.3.as_ref() {
        walk_spec_exp(exp, ctx, diags);
    }
}

/// Check whether a NameAccessChain refers to `old`.
fn is_old(name: &legacy_move_compiler::parser::ast::NameAccessChain) -> bool {
    matches!(&name.value, NameAccessChain_::One(n) if n.value.as_str() == "old")
}

/// Check whether an expression is a simple name (a single identifier with no type args).
fn is_simple_name(exp: &Exp) -> bool {
    matches!(&exp.value, Exp_::Name(n, None) if matches!(&n.value, NameAccessChain_::One(_)))
}

fn walk_spec_exp(exp: &Exp, ctx: &SpecContext, diags: &mut Diagnostics) {
    match &exp.value {
        Exp_::Call(name, _, _, args) if is_old(name) => {
            match ctx {
                SpecContext::Ensures | SpecContext::InvariantUpdate => {
                    // OK — old() allowed here
                },
                SpecContext::LoopInvariant => {
                    // In loop invariants, old() only allowed with simple name args
                    for arg in &args.value {
                        if !is_simple_name(arg) {
                            diags.add(Diagnostic::new(
                                TypeSafety::BuiltinOperation,
                                (
                                    exp.loc,
                                    "`old(..)` in a loop invariant can only take a simple \
                                     parameter name as argument",
                                ),
                                std::iter::empty::<(Loc, String)>(),
                                std::iter::empty::<String>(),
                            ));
                        }
                    }
                },
                SpecContext::Other => {
                    diags.add(Diagnostic::new(
                        TypeSafety::BuiltinOperation,
                        (
                            exp.loc,
                            "`old(..)` is not allowed in this spec context; it can only be \
                             used in `ensures`, global update invariants, or loop invariants \
                             (with a simple parameter name)",
                        ),
                        std::iter::empty::<(Loc, String)>(),
                        std::iter::empty::<String>(),
                    ));
                },
            }
            // Recurse into args
            for arg in &args.value {
                walk_spec_exp(arg, ctx, diags);
            }
        },
        Exp_::Dereference(inner) => {
            diags.add(Diagnostic::new(
                TypeSafety::BuiltinOperation,
                (
                    exp.loc,
                    "dereference `*e` is not allowed in spec expressions",
                ),
                std::iter::empty::<(Loc, String)>(),
                std::iter::empty::<String>(),
            ));
            walk_spec_exp(inner, ctx, diags);
        },
        Exp_::Borrow(_, inner) => {
            diags.add(Diagnostic::new(
                TypeSafety::BuiltinOperation,
                (exp.loc, "borrow `&e` is not allowed in spec expressions"),
                std::iter::empty::<(Loc, String)>(),
                std::iter::empty::<String>(),
            ));
            walk_spec_exp(inner, ctx, diags);
        },
        // Recurse into sub-expressions
        Exp_::Call(_, _, _, args) => {
            for arg in &args.value {
                walk_spec_exp(arg, ctx, diags);
            }
        },
        Exp_::BinopExp(l, _, r) => {
            walk_spec_exp(l, ctx, diags);
            walk_spec_exp(r, ctx, diags);
        },
        Exp_::UnaryExp(_, e) => walk_spec_exp(e, ctx, diags),
        Exp_::Dot(e, _) | Exp_::Cast(e, _) | Exp_::Annotate(e, _) => {
            walk_spec_exp(e, ctx, diags);
        },
        Exp_::Index(e1, e2) => {
            walk_spec_exp(e1, ctx, diags);
            walk_spec_exp(e2, ctx, diags);
        },
        Exp_::IfElse(c, t, e) => {
            walk_spec_exp(c, ctx, diags);
            walk_spec_exp(t, ctx, diags);
            if let Some(e) = e {
                walk_spec_exp(e, ctx, diags);
            }
        },
        Exp_::Block(seq) => walk_spec_sequence(seq, ctx, diags),
        Exp_::Spec(spec_block) => walk_spec_block(spec_block, diags),
        Exp_::Quant(_, binds, triggers, where_cond, body) => {
            for bind in &binds.value {
                walk_spec_exp(&bind.value.1, ctx, diags);
            }
            for trigger_group in triggers {
                for trigger in trigger_group {
                    walk_spec_exp(trigger, ctx, diags);
                }
            }
            if let Some(w) = where_cond {
                walk_spec_exp(w, ctx, diags);
            }
            walk_spec_exp(body, ctx, diags);
        },
        Exp_::Lambda(_, e, _, _) => walk_spec_exp(e, ctx, diags),
        Exp_::ExpList(es) => {
            for e in es {
                walk_spec_exp(e, ctx, diags);
            }
        },
        Exp_::Assign(l, _, r) => {
            walk_spec_exp(l, ctx, diags);
            walk_spec_exp(r, ctx, diags);
        },
        // Leaf nodes: Name, Value, Unit, Move, Copy, etc.
        _ => {},
    }
}

// ─── Formatting ──────────────────────────────────────────────────────────────

/// Locate the movefmt binary: $MOVEFMT_EXE → ~/.local/bin/movefmt → PATH.
pub(crate) fn find_movefmt() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("MOVEFMT_EXE") {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var("HOME").ok()?;
    let managed = PathBuf::from(home).join(".local/bin/movefmt");
    if managed.is_file() {
        return Some(managed);
    }
    pathsearch::find_executable_in_path("movefmt")
}

/// Run movefmt on the file in-place. Silently skips if movefmt is unavailable.
/// Set `MOVE_FLOW_NO_FMT=1` to suppress formatting (used by tests for
/// deterministic baselines across platforms).
pub(crate) fn format_file(path: &str) {
    if std::env::var("MOVE_FLOW_NO_FMT").is_ok() {
        return;
    }
    let exe = match find_movefmt() {
        Some(e) => e,
        None => {
            log::info!("edit hook: movefmt not found, skipping format");
            return;
        },
    };
    log::info!("edit hook: formatting `{}` with {}", path, exe.display());
    match Command::new(&exe)
        .arg("--emit=overwrite")
        .arg(format!("--file-path={}", path))
        .output()
    {
        Ok(out) if out.status.success() => {
            log::info!("edit hook: formatted `{}` successfully", path);
        },
        Ok(out) => {
            log::info!(
                "edit hook: movefmt failed on `{}`: {}",
                path,
                String::from_utf8_lossy(&out.stderr)
            );
        },
        Err(e) => {
            log::info!("edit hook: failed to run movefmt: {}", e);
        },
    }
}

// ─── Text checks ─────────────────────────────────────────────────────────────

/// Deprecated Move 1 patterns to flag.
const DEPRECATED_PATTERNS: &[(&str, &str)] = &[
    ("borrow_global<", "use `&T[addr]` instead"),
    ("borrow_global_mut<", "use `&mut T[addr]` instead"),
    ("acquires", "acquires annotations are no longer needed"),
];

/// Return a byte mask for ranges that are not code (comments / string literals).
fn ignored_context_mask(source: &str) -> Vec<bool> {
    let bytes = source.as_bytes();
    let mut ignored = vec![false; bytes.len()];
    let mut i = 0;
    let mut in_line_comment = false;
    let mut block_comment_depth = 0usize;
    let mut in_string = false;

    while i < bytes.len() {
        if in_line_comment {
            if bytes[i] == b'\n' {
                in_line_comment = false;
                i += 1;
                continue;
            }
            ignored[i] = true;
            i += 1;
            continue;
        }

        if block_comment_depth > 0 {
            ignored[i] = true;
            if i + 1 < bytes.len() {
                if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                    ignored[i + 1] = true;
                    block_comment_depth += 1;
                    i += 2;
                    continue;
                }
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    ignored[i + 1] = true;
                    block_comment_depth -= 1;
                    i += 2;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        if in_string {
            ignored[i] = true;
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                ignored[i + 1] = true;
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            ignored[i] = true;
            ignored[i + 1] = true;
            in_line_comment = true;
            i += 2;
            continue;
        }

        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            ignored[i] = true;
            ignored[i + 1] = true;
            block_comment_depth = 1;
            i += 2;
            continue;
        }

        if bytes[i] == b'"' {
            ignored[i] = true;
            in_string = true;
            i += 1;
            continue;
        }

        i += 1;
    }

    ignored
}

/// Scan source text for deprecated Move 1 global-storage operations.
fn text_checks(source: &str, file_hash: FileHash) -> Diagnostics {
    let mut diags = Diagnostics::new();
    let ignored_mask = ignored_context_mask(source);
    for &(pattern, hint) in DEPRECATED_PATTERNS {
        for (start, _) in source.match_indices(pattern) {
            // Skip text matches that are inside comments or string literals.
            if ignored_mask.get(start).copied().unwrap_or(false) {
                continue;
            }

            // For `acquires`, only match if it looks like a keyword (preceded by
            // whitespace or line start, followed by whitespace or `{`).
            if pattern == "acquires" {
                let before_ok = start == 0
                    || source.as_bytes()[start - 1].is_ascii_whitespace()
                    || source.as_bytes()[start - 1] == b')';
                let end = start + pattern.len();
                let after_ok = end >= source.len()
                    || source.as_bytes()[end].is_ascii_whitespace()
                    || source.as_bytes()[end] == b'{';
                if !before_ok || !after_ok {
                    continue;
                }
            }

            let end = start + pattern.len();
            let loc = Loc::new(file_hash, start as u32, end as u32);
            let msg = format!("deprecated Move 1 syntax: `{}`; {}", pattern, hint);
            diags.add(Diagnostic::new(
                Uncategorized::DeprecatedWillBeRemoved,
                (loc, msg),
                std::iter::empty::<(Loc, String)>(),
                std::iter::empty::<String>(),
            ));
        }
    }
    diags
}
