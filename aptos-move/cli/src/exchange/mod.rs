// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Backend of the `exchange` command: connects the Move assembler
//! (`move-asm`) and compiler v2 to the Lean formalization of the Move
//! Prover (`third_party/move/lean`).
//!
//! Takes masm source, assembles it with the real assembler, verifies the
//! resulting module, loads it into a `GlobalEnv` (no source needed), lifts
//! every function to stackless bytecode with the real
//! `StacklessBytecodeGenerator`, splits the code into basic blocks in code
//! layout order, and builds a [`move_model_exchange::Module`].  Callers
//! serialize it to JSON with serde; the `move-model-exchange` crate's data
//! structures are the normative schema of that JSON.  The Lean side
//! (`Move/Frontend/` in the Lean project) invokes the command at elaboration
//! time, decoding the JSON into its `IR` representation.
//!
//! The export only contains what the Lean fragment can represent;
//! everything else (generics, enums, vectors, non-u64 integer widths,
//! casts, …) is rejected with an error.  Reference operations are passed
//! through — the Lean model executes them; verifying borrow-based code goes
//! through its reference elimination.

mod model_spec;
mod source;
mod spec;

use crate::exchange::{
    model_spec::{translate_modifies_target, ModelSpecCtx},
    spec::{translate_clauses, FunSpecInput, SpecCtx, TranslatedSpec},
};
use anyhow::{anyhow, bail, Context, Result};
use codespan_reporting::term::termcolor::Buffer;
use either::Either;
use move_asm::{
    assembler::{assemble_unit, diag_to_string, Options},
    syntax::Unit,
};
use move_binary_format::CompiledModule;
use move_model::{
    ast::Address,
    model::{FunId, FunctionEnv, GlobalEnv, ModuleEnv, QualifiedId, StructId},
};
use move_model_exchange as exchange;
use move_stackless_bytecode::{
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{AttrId, Bytecode, Constant, Label, Operation, PropKind},
};
pub use source::{move_file_to_module, move_source_to_module};
use std::collections::{BTreeMap, BTreeSet};

/// Runs the full frontend: masm source to the exchange module.
pub fn masm_to_module(input: &str) -> Result<exchange::Module> {
    let mut ast = match move_asm::syntax::parse_asm(input) {
        Ok(ast) => ast,
        Err(diags) => bail!("{}", diag_to_string("<masm>", input, diags)),
    };
    // The assembler ignores spec clauses; take them (with the names needed
    // to resolve them) out of the AST before assembly consumes it.
    let spec_inputs: Vec<FunSpecInput> = ast.functions.iter_mut().map(FunSpecInput::take).collect();
    let module = assemble_module(input, ast)?;
    move_bytecode_verifier::verify_module(&module)
        .map_err(|e| anyhow!("bytecode verification failed: {:#?}", e))?;
    let mut env = GlobalEnv::new();
    let source_map = env.empty_source_map("<masm>", input.as_bytes());
    let module_id = env.load_compiled_module(/*with_dep_closure*/ true, module, source_map);
    let mut error_writer = Buffer::no_color();
    env.check_diag(
        &mut error_writer,
        codespan_reporting::diagnostic::Severity::Warning,
        "loading compiled module",
    )
    .map_err(|_| {
        anyhow!(
            "loading compiled module failed:\n{}",
            String::from_utf8_lossy(&error_writer.into_inner())
        )
    })?;
    dump_module(&env, module_id, &spec_inputs)
}

/// Assembles a parsed masm unit into a module (scripts are not supported);
/// `input` is the source, for diagnostics.
fn assemble_module(input: &str, ast: Unit) -> Result<CompiledModule> {
    let options = Options::default();
    match assemble_unit(&options, ast, std::iter::empty::<&CompiledModule>()) {
        Ok(Either::Left(module)) => Ok(module),
        Ok(Either::Right(_)) => bail!("scripts are not supported, expected a module"),
        Err(diags) => bail!("{}", diag_to_string("<masm>", input, diags)),
    }
}

/// Name maps from model ids to the dense indices used by the exchange
/// format (declaration order).
struct NameMaps {
    structs: BTreeMap<QualifiedId<StructId>, usize>,
    funs: BTreeMap<QualifiedId<FunId>, usize>,
}

/// Where a function's loop invariants come from.
enum InvariantSource<'a> {
    /// masm mode: translated `invariant` clauses, keyed by the masm
    /// instruction index their label is attached to (code order).
    MasmClauses(&'a [(usize, exchange::SpecExp)]),
    /// Source mode: `Prop(Assert)` instructions in the code whose attr is
    /// in `FunctionData.loop_invariants`, translated via the model.
    ModelProps(&'a ModelSpecCtx<'a>, &'a BTreeSet<AttrId>),
}

impl<'a> InvariantSource<'a> {
    /// The context for translating the loop invariant carried by a
    /// `Prop(Assert)` instruction with the given attribute, if it is one.
    fn invariant_ctx(&self, attr: &AttrId) -> Option<&'a ModelSpecCtx<'a>> {
        match self {
            InvariantSource::ModelProps(ctx, attrs) if attrs.contains(attr) => Some(ctx),
            InvariantSource::ModelProps(..) | InvariantSource::MasmClauses(_) => None,
        }
    }
}

/// Scans the module declarations shared by both modes: dense struct and
/// function ids in declaration order (enums and natives are rejected,
/// compiler-generated struct APIs skipped), the translated structs, the
/// function environments to dump, and their baseline stackless-bytecode
/// targets.  Structs take two passes, so that field types may reference
/// structs declared later.
fn scan_module<'env>(
    module_env: &'env ModuleEnv<'env>,
) -> Result<(
    NameMaps,
    Vec<exchange::Struct>,
    Vec<FunctionEnv<'env>>,
    FunctionTargetsHolder,
)> {
    let pool = module_env.env.symbol_pool();

    let mut struct_map = BTreeMap::new();
    for struct_env in module_env.get_structs() {
        if struct_env.has_variants() {
            bail!(
                "enum `{}` not supported",
                struct_env.get_name().display(pool)
            );
        }
        struct_map.insert(struct_env.get_qualified_id(), struct_map.len());
    }
    let mut structs = vec![];
    for struct_env in module_env.get_structs() {
        let fields = struct_env
            .get_fields()
            .map(|field| {
                Ok(exchange::Field {
                    name: field.get_name().display(pool).to_string(),
                    ty: translate_type(&struct_map, &field.get_type())?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        structs.push(exchange::Struct {
            name: struct_env.get_name().display(pool).to_string(),
            fields,
        });
    }

    let mut fun_map = BTreeMap::new();
    let mut fun_envs = vec![];
    for func_env in module_env.get_functions() {
        if func_env.is_struct_api() {
            continue;
        }
        if func_env.is_native() {
            bail!(
                "native function `{}` not supported",
                func_env.get_name().display(pool)
            );
        }
        fun_map.insert(func_env.get_qualified_id(), fun_envs.len());
        fun_envs.push(func_env);
    }

    let mut targets = FunctionTargetsHolder::default();
    for func_env in &fun_envs {
        targets.add_target(func_env);
    }
    Ok((
        NameMaps {
            structs: struct_map,
            funs: fun_map,
        },
        structs,
        fun_envs,
        targets,
    ))
}

fn dump_module(
    env: &GlobalEnv,
    module_id: move_model::model::ModuleId,
    spec_inputs: &[FunSpecInput],
) -> Result<exchange::Module> {
    let module_env = env.get_module(module_id);
    let pool = env.symbol_pool();
    let (maps, structs, fun_envs, targets) = scan_module(&module_env)?;

    // Struct-name map for the masm spec clauses, derived from the dumped
    // structs (their order is the dense ids).
    let struct_names: BTreeMap<String, usize> = structs
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.clone(), i))
        .collect();

    let mut funs = vec![];
    for func_env in &fun_envs {
        let target = targets.get_target(func_env, &FunctionVariant::Baseline);
        let name = func_env.get_name().display(pool).to_string();
        let fun_input = spec_inputs
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| anyhow!("function `{}` not found in the masm AST", name))?;
        let locals = (0..target.get_local_count())
            .map(|i| translate_type(&maps.structs, target.get_local_type(i)))
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("in function `{}`", name))?;
        let returns = translate_return_types(&maps, func_env)
            .with_context(|| format!("in function `{}`", name))?;
        let ctx = SpecCtx {
            temps: fun_input
                .decls
                .iter()
                .enumerate()
                .map(|(i, n)| (n.clone(), i))
                .collect(),
            structs: struct_names.clone(),
            struct_decls: &structs,
            local_types: &locals,
            num_params: func_env.get_parameter_count(),
            returns: &returns,
        };
        let TranslatedSpec {
            contract,
            invariants,
        } = translate_clauses(&ctx, fun_input)
            .with_context(|| format!("in function `{}`", name))?;
        let fun = dump_function(
            &name,
            func_env.get_parameter_count(),
            locals,
            returns,
            target.get_bytecode(),
            &maps,
            &structs,
            contract,
            InvariantSource::MasmClauses(&invariants),
        )
        .with_context(|| format!("in function `{}`", name))?;
        funs.push(fun);
    }

    Ok(exchange::Module::new(structs, funs))
}

/// Translates a move-model type into the exchange fragment.
fn translate_type(
    structs: &BTreeMap<QualifiedId<StructId>, usize>,
    ty: &move_model::ty::Type,
) -> Result<exchange::Type> {
    use move_model::ty::{PrimitiveType, ReferenceKind, Type};
    match ty {
        Type::Primitive(PrimitiveType::Bool) => Ok(exchange::Type::Bool),
        Type::Primitive(PrimitiveType::U64) => Ok(exchange::Type::U64),
        Type::Primitive(PrimitiveType::Address) => Ok(exchange::Type::Address),
        Type::Primitive(PrimitiveType::Signer) => Ok(exchange::Type::Signer),
        Type::Struct(mid, sid, targs) => {
            if !targs.is_empty() {
                bail!("generic struct types not supported");
            }
            let idx = structs
                .get(&mid.qualified(*sid))
                .copied()
                .ok_or_else(|| anyhow!("struct of foreign module not supported"))?;
            Ok(exchange::Type::Struct(idx))
        },
        Type::Reference(kind, inner) => {
            let inner = Box::new(translate_type(structs, inner)?);
            Ok(match kind {
                ReferenceKind::Immutable => exchange::Type::Ref(inner),
                ReferenceKind::Mutable => exchange::Type::MutRef(inner),
            })
        },
        ty => bail!("unsupported type {:?}", ty),
    }
}

/// The translated return types of a function (a tuple result flattens to
/// one entry per component).
fn translate_return_types(
    maps: &NameMaps,
    func_env: &move_model::model::FunctionEnv,
) -> Result<Vec<exchange::Type>> {
    match func_env.get_result_type() {
        move_model::ty::Type::Tuple(ts) => ts
            .iter()
            .map(|t| translate_type(&maps.structs, t))
            .collect(),
        ty => Ok(vec![translate_type(&maps.structs, &ty)?]),
    }
}

/// Dumps a module using the model's specifications (source mode): contracts
/// from `FunctionEnv::get_spec()` and loop invariants from the `Prop`
/// instructions at loop headers.  Typing assumptions are not synthesized:
/// consumers derive well-formedness from the declared types.
pub fn dump_module_from_model(
    env: &GlobalEnv,
    module_id: move_model::model::ModuleId,
) -> Result<exchange::Module> {
    use move_model::ast::ConditionKind;

    let module_env = env.get_module(module_id);
    let pool = env.symbol_pool();
    let (maps, structs, fun_envs, targets) = scan_module(&module_env)?;
    let spec_ctx = ModelSpecCtx {
        env,
        structs: &maps.structs,
    };

    let mut funs = vec![];
    for func_env in &fun_envs {
        let name = func_env.get_name().display(pool).to_string();
        let data = targets
            .get_data(&func_env.get_qualified_id(), &FunctionVariant::Baseline)
            .ok_or_else(|| anyhow!("no function target for `{}`", name))?;
        let contract = (|| -> Result<exchange::Contract> {
            let spec = func_env.get_spec();
            let mut requires = vec![];
            let mut aborts_if = vec![];
            let mut ensures = vec![];
            for cond in &spec.conditions {
                let mut binders = vec![];
                match &cond.kind {
                    ConditionKind::Requires => requires.push(model_spec::translate_exp(
                        &spec_ctx,
                        &mut binders,
                        false,
                        cond.exp.as_ref(),
                    )?),
                    ConditionKind::AbortsIf => {
                        if !cond.additional_exps.is_empty() {
                            bail!("`aborts_if .. with ..` not supported");
                        }
                        aborts_if.push(model_spec::translate_exp(
                            &spec_ctx,
                            &mut binders,
                            false,
                            cond.exp.as_ref(),
                        )?)
                    },
                    ConditionKind::Ensures => ensures.push(model_spec::translate_exp(
                        &spec_ctx,
                        &mut binders,
                        false,
                        cond.exp.as_ref(),
                    )?),
                    kind => bail!("unsupported condition kind {:?}", kind),
                }
            }
            let modifies = match &spec.frame_spec {
                Some(fs) => {
                    if fs.modifies_all {
                        bail!("`modifies *` not supported");
                    }
                    fs.modifies_targets
                        .iter()
                        .map(|e| translate_modifies_target(&spec_ctx, e))
                        .collect::<Result<Vec<_>>>()?
                },
                None => vec![],
            };
            Ok(exchange::Contract {
                requires,
                aborts_if,
                ensures,
                modifies,
            })
        })()
        .with_context(|| format!("in the specification of `{}`", name))?;
        let locals = data
            .local_types
            .iter()
            .map(|ty| translate_type(&maps.structs, ty))
            .collect::<Result<Vec<_>>>()
            .with_context(|| format!("in function `{}`", name))?;
        let returns = translate_return_types(&maps, func_env)
            .with_context(|| format!("in function `{}`", name))?;
        let fun = dump_function(
            &name,
            func_env.get_parameter_count(),
            locals,
            returns,
            &data.code,
            &maps,
            &structs,
            contract,
            InvariantSource::ModelProps(&spec_ctx, &data.loop_invariants),
        )
        .with_context(|| format!("in function `{}`", name))?;
        funs.push(fun);
    }

    Ok(exchange::Module::new(structs, funs))
}

fn dump_function(
    name: &str,
    num_params: usize,
    mut locals: Vec<exchange::Type>,
    returns: Vec<exchange::Type>,
    code: &[Bytecode],
    maps: &NameMaps,
    structs: &[exchange::Struct],
    contract: exchange::Contract,
    invariants: InvariantSource,
) -> Result<exchange::Function> {
    // Block boundaries: code offset 0, every label, and every offset
    // following an always-branching instruction.
    let label_offsets = Bytecode::label_offsets(code);
    let mut starts = BTreeSet::new();
    starts.insert(0usize);
    for (i, bc) in code.iter().enumerate() {
        if let Bytecode::Label(..) = bc {
            starts.insert(i);
        }
        if bc.is_always_branching() && i + 1 < code.len() {
            starts.insert(i + 1);
        }
    }
    let starts: Vec<usize> = starts.into_iter().collect();
    let block_of_offset = |off: usize| -> usize {
        match starts.binary_search(&off) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    };
    let block_of_label = |l: &Label| -> Result<usize> {
        let off = label_offsets
            .get(l)
            .ok_or_else(|| anyhow!("undefined label"))?;
        Ok(block_of_offset(*off as usize))
    };

    let mut blocks = vec![];
    let mut edges: Vec<(usize, usize)> = vec![];
    // Source mode: invariants found as `Prop` instructions, per block.
    let mut block_invariants: BTreeMap<usize, Vec<exchange::SpecExp>> = BTreeMap::new();
    for (bi, &start) in starts.iter().enumerate() {
        let end = starts.get(bi + 1).copied().unwrap_or(code.len());
        let mut instrs: Vec<exchange::Instr> = vec![];
        let mut term: Option<exchange::Term> = None;
        for bc in &code[start..end] {
            match bc {
                Bytecode::Label(..) | Bytecode::Nop(..) => {},
                Bytecode::Prop(attr, PropKind::Assert, exp) => {
                    match invariants.invariant_ctx(attr) {
                        Some(ctx) => {
                            let mut binders = vec![];
                            block_invariants
                                .entry(bi)
                                .or_default()
                                .push(model_spec::translate_exp(ctx, &mut binders, false, exp)?);
                        },
                        None => bail!("unsupported bytecode {:?}", bc),
                    }
                },
                Bytecode::Load(_, dst, cons) => {
                    instrs.push(exchange::Instr::Load(*dst, constant_value(cons)?));
                },
                Bytecode::Assign(_, dst, src, _) => {
                    instrs.push(exchange::Instr::Assign(*dst, *src));
                },
                Bytecode::Call(_, dsts, op, srcs, abort_action) => {
                    if abort_action.is_some() {
                        bail!("unexpected abort action in baseline bytecode");
                    }
                    translate_call(&mut instrs, dsts, op, srcs, maps, &mut locals)?;
                },
                Bytecode::Ret(_, srcs) => {
                    term = Some(exchange::Term::Ret(srcs.clone()));
                },
                Bytecode::Abort(_, code_temp, _msg) => {
                    term = Some(exchange::Term::Abort(*code_temp));
                },
                Bytecode::Jump(_, l) => {
                    let b = block_of_label(l)?;
                    edges.push((bi, b));
                    term = Some(exchange::Term::Jump(b));
                },
                Bytecode::Branch(_, then_l, else_l, cond) => {
                    let tb = block_of_label(then_l)?;
                    let eb = block_of_label(else_l)?;
                    edges.push((bi, tb));
                    edges.push((bi, eb));
                    term = Some(exchange::Term::Branch(*cond, tb, eb));
                },
                bc => bail!("unsupported bytecode {:?}", bc),
            }
        }
        let term = match term {
            Some(t) => t,
            None => {
                // Fall-through into the next block.
                if bi + 1 >= starts.len() {
                    bail!("function `{}` falls off the end of the code", name);
                }
                edges.push((bi, bi + 1));
                exchange::Term::Jump(bi + 1)
            },
        };
        blocks.push(exchange::Block { instrs, term });
    }

    let mut loops = collect_loops(&blocks, &edges)?;
    match invariants {
        InvariantSource::MasmClauses(invariants) if !invariants.is_empty() => {
            // Group the invariant clauses by their (masm) label position;
            // both the labels and the detected loop headers are in code
            // order, so they match up positionally.
            let mut grouped: Vec<(usize, Vec<exchange::SpecExp>)> = vec![];
            for (pos, exp) in invariants {
                match grouped.last_mut() {
                    Some((last, exps)) if last == pos => exps.push(exp.clone()),
                    _ => grouped.push((*pos, vec![exp.clone()])),
                }
            }
            if grouped.len() != loops.len() {
                bail!(
                    "function `{}` has {} loop header(s) but invariants for {} label(s); \
                     when any invariant is given, every loop header needs one",
                    name,
                    loops.len(),
                    grouped.len()
                );
            }
            for (lp, (_, exps)) in loops.iter_mut().zip(grouped) {
                lp.invariants = exps;
            }
        },
        InvariantSource::MasmClauses(_) => {},
        InvariantSource::ModelProps(..) => {
            for lp in loops.iter_mut() {
                if let Some(exps) = block_invariants.remove(&lp.header) {
                    lp.invariants = exps;
                }
            }
            if let Some((b, _)) = block_invariants.iter().next() {
                bail!(
                    "function `{}` has loop invariants at block {} which is \
                     not a loop header",
                    name,
                    b
                );
            }
        },
    }

    let fun = exchange::Function {
        name: name.to_string(),
        params: num_params,
        locals,
        returns,
        blocks,
        loops,
        spec: contract,
    };
    // Reject ill-scoped or ill-sorted clauses: consumers *assume*
    // `requires` at entry, so a stuck clause would make verification
    // vacuous instead of failing.
    exchange::check::SpecCheckCtx {
        structs,
        locals: &fun.locals,
        num_params: fun.params,
        returns: &fun.returns,
    }
    .check_function(&fun)
    .map_err(|e| anyhow!("{}", e))?;
    Ok(fun)
}

fn constant_value(cons: &Constant) -> Result<exchange::Value> {
    match cons {
        Constant::Bool(b) => Ok(exchange::Value::Bool(*b)),
        Constant::U64(x) => Ok(exchange::Value::U64(*x)),
        Constant::Address(Address::Numerical(a)) => {
            Ok(exchange::Value::Address(a.to_hex_literal()))
        },
        cons => bail!("unsupported constant {:?}", cons),
    }
}

/// Translates one stackless `Call` into (usually one) exchange
/// instruction(s).  `gt`/`ge` become `lt`/`le` with swapped operands;
/// `neq` is synthesized as `eq` + `not` through a fresh temporary;
/// `drop`/`release` are value/borrow bookkeeping without effect in the
/// Lean model and become nothing.
fn translate_call(
    instrs: &mut Vec<exchange::Instr>,
    dsts: &[usize],
    op: &Operation,
    srcs: &[usize],
    maps: &NameMaps,
    locals: &mut Vec<exchange::Type>,
) -> Result<()> {
    use Operation::*;
    let simple = |op: exchange::Oper| exchange::Instr::Call(dsts.to_vec(), op, srcs.to_vec());
    let resource = |m: &move_model::model::ModuleId, s: &StructId| -> Result<usize> {
        maps.structs
            .get(&m.qualified(*s))
            .copied()
            .ok_or_else(|| anyhow!("resource of foreign module not supported"))
    };
    let no_generics = |tys: &[move_model::ty::Type]| -> Result<()> {
        if tys.is_empty() {
            Ok(())
        } else {
            bail!("generics not supported")
        }
    };
    let instr = match op {
        Add => simple(exchange::Oper::Add),
        Sub => simple(exchange::Oper::Sub),
        Mul => simple(exchange::Oper::Mul),
        Div => simple(exchange::Oper::Div),
        Mod => simple(exchange::Oper::Mod),
        Lt => simple(exchange::Oper::Lt),
        Le => simple(exchange::Oper::Le),
        Eq => simple(exchange::Oper::Eq),
        And => simple(exchange::Oper::And),
        Or => simple(exchange::Oper::Or),
        Not => simple(exchange::Oper::Not),
        Gt => exchange::Instr::Call(
            dsts.to_vec(),
            exchange::Oper::Lt,
            srcs.iter().rev().copied().collect(),
        ),
        Ge => exchange::Instr::Call(
            dsts.to_vec(),
            exchange::Oper::Le,
            srcs.iter().rev().copied().collect(),
        ),
        Neq => {
            // Synthesized temporary, declared alongside the source locals.
            let tmp = locals.len();
            locals.push(exchange::Type::Bool);
            instrs.push(exchange::Instr::Call(
                vec![tmp],
                exchange::Oper::Eq,
                srcs.to_vec(),
            ));
            exchange::Instr::Call(dsts.to_vec(), exchange::Oper::Not, vec![tmp])
        },
        Pack(_, _, tys) => {
            no_generics(tys)?;
            simple(exchange::Oper::Pack)
        },
        Unpack(_, _, tys) => {
            no_generics(tys)?;
            simple(exchange::Oper::Unpack)
        },
        GetField(_, _, tys, offset) => {
            no_generics(tys)?;
            simple(exchange::Oper::GetField(*offset))
        },
        GetGlobal(m, s, tys) => {
            no_generics(tys)?;
            simple(exchange::Oper::GetGlobal(resource(m, s)?))
        },
        MoveTo(m, s, tys) => {
            no_generics(tys)?;
            simple(exchange::Oper::MoveTo(resource(m, s)?))
        },
        MoveFrom(m, s, tys) => {
            no_generics(tys)?;
            simple(exchange::Oper::MoveFrom(resource(m, s)?))
        },
        Exists(m, s, tys) => {
            no_generics(tys)?;
            simple(exchange::Oper::Exists(resource(m, s)?))
        },
        Function(m, f, tys) => {
            no_generics(tys)?;
            let idx = maps
                .funs
                .get(&m.qualified(*f))
                .copied()
                .ok_or_else(|| anyhow!("call to foreign or unsupported function"))?;
            simple(exchange::Oper::Function(idx))
        },
        BorrowLoc => simple(exchange::Oper::BorrowLoc),
        BorrowField(_, _, tys, offset) => {
            no_generics(tys)?;
            simple(exchange::Oper::BorrowField(*offset))
        },
        BorrowGlobal(m, s, tys) => {
            no_generics(tys)?;
            simple(exchange::Oper::BorrowGlobal(resource(m, s)?))
        },
        ReadRef => simple(exchange::Oper::ReadRef),
        WriteRef => simple(exchange::Oper::WriteRef),
        FreezeRef(_) => simple(exchange::Oper::FreezeRef),
        Drop | Release => return Ok(()),
        op => bail!("unsupported operation {:?}", op),
    };
    instrs.push(instr);
    Ok(())
}

/// The dominator sets of the block graph (entry = block 0), by iterative
/// dataflow: `dom(entry) = {entry}`, `dom(n) = {n} ∪ ⋂ dom(preds n)`.
fn dominators(num_blocks: usize, preds: &BTreeMap<usize, Vec<usize>>) -> Vec<BTreeSet<usize>> {
    let all: BTreeSet<usize> = (0..num_blocks).collect();
    let mut dom: Vec<BTreeSet<usize>> = (0..num_blocks)
        .map(|n| {
            if n == 0 {
                BTreeSet::from([0])
            } else {
                all.clone()
            }
        })
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for n in 1..num_blocks {
            let mut new: Option<BTreeSet<usize>> = None;
            for p in preds.get(&n).into_iter().flatten() {
                new = Some(match new {
                    None => dom[*p].clone(),
                    Some(acc) => acc.intersection(&dom[*p]).copied().collect(),
                });
            }
            let mut new = new.unwrap_or_default();
            new.insert(n);
            if new != dom[n] {
                dom[n] = new;
                changed = true;
            }
        }
    }
    dom
}

/// Whether the graph restricted to `edges` (with the accepted back edges
/// removed) is acyclic, by depth-first search from every node.
fn is_acyclic(
    num_blocks: usize,
    edges: &[(usize, usize)],
    removed: &BTreeSet<(usize, usize)>,
) -> bool {
    let mut succs: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for e in edges {
        if !removed.contains(e) {
            succs.entry(e.0).or_default().push(e.1);
        }
    }
    // 0 = unvisited, 1 = on stack, 2 = done
    let mut state = vec![0u8; num_blocks];
    fn dfs(n: usize, succs: &BTreeMap<usize, Vec<usize>>, state: &mut Vec<u8>) -> bool {
        state[n] = 1;
        for m in succs.get(&n).into_iter().flatten() {
            let st = state[*m];
            if st == 1 || (st == 0 && !dfs(*m, succs, state)) {
                return false;
            }
        }
        state[n] = 2;
        true
    }
    (0..num_blocks).all(|n| state[n] != 0 || dfs(n, &succs, &mut state))
}

/// Detects natural loops and computes their metadata: member blocks (nodes
/// reaching a back-edge source without passing the header), written
/// locals, and written resources.  A back edge is an edge to a
/// lower-or-equal block id (code layout order, per the exchange format's
/// convention) *whose target dominates its source* — layout back edges
/// without dominance are ordinary forward control flow, not loops.  Cycles
/// not covered by such back edges (rotated loops entered below their
/// lowest block, irreducible control flow) are outside the supported
/// fragment and rejected.
fn collect_loops(
    blocks: &[exchange::Block],
    edges: &[(usize, usize)],
) -> Result<Vec<exchange::Loop>> {
    use exchange::Oper;
    let mut preds: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (from, to) in edges {
        preds.entry(*to).or_default().push(*from);
    }
    let dom = dominators(blocks.len(), &preds);
    // header -> set of back-edge sources
    let mut back: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut back_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
    for (from, to) in edges {
        if *to <= *from && dom[*from].contains(to) {
            back.entry(*to).or_default().insert(*from);
            back_edges.insert((*from, *to));
        }
    }
    if !is_acyclic(blocks.len(), edges, &back_edges) {
        bail!(
            "unsupported loop shape: a cycle is not headed by its \
             lowest-numbered block (rotated or irreducible control flow)"
        );
    }
    let mut loops = vec![];
    for (header, sources) in back {
        let mut members = BTreeSet::new();
        members.insert(header);
        let mut work: Vec<usize> = sources.into_iter().collect();
        while let Some(n) = work.pop() {
            if members.insert(n) {
                for p in preds.get(&n).into_iter().flatten() {
                    work.push(*p);
                }
            }
        }
        // Written temporaries and resources of the member blocks.
        let mut val_targets = BTreeSet::new();
        let mut mem_targets = BTreeSet::new();
        for m in &members {
            for instr in &blocks[*m].instrs {
                match instr {
                    exchange::Instr::Load(dst, _) | exchange::Instr::Assign(dst, _) => {
                        val_targets.insert(*dst);
                    },
                    exchange::Instr::Call(dsts, op, _) => {
                        val_targets.extend(dsts.iter().copied());
                        match op {
                            Oper::MoveTo(r) | Oper::MoveFrom(r) | Oper::WriteGlobal(r) => {
                                mem_targets.insert(*r);
                            },
                            // A call may modify anything; recognized when
                            // spec support lands. For now, reject.
                            Oper::Function(_) => bail!(
                                "calls inside loops not yet supported by loop-target collection"
                            ),
                            // A reference write may target any borrowed
                            // local or resource; attributing it needs the
                            // borrow analysis. Reject, like calls.
                            Oper::WriteRef => bail!(
                                "writes through references inside loops not yet supported \
                                 by loop-target collection"
                            ),
                            Oper::Add
                            | Oper::Sub
                            | Oper::Mul
                            | Oper::Div
                            | Oper::Mod
                            | Oper::Lt
                            | Oper::Le
                            | Oper::Eq
                            | Oper::And
                            | Oper::Or
                            | Oper::Not
                            | Oper::Pack
                            | Oper::Unpack
                            | Oper::GetField(_)
                            | Oper::UpdateField(_)
                            | Oper::VecPack
                            | Oper::VecLen
                            | Oper::VecGet
                            | Oper::VecSet
                            | Oper::VecPush
                            | Oper::VecPop
                            | Oper::GetGlobal(_)
                            | Oper::Exists(_)
                            | Oper::BorrowLoc
                            | Oper::BorrowField(_)
                            | Oper::BorrowGlobal(_)
                            | Oper::BorrowVecElem
                            | Oper::ReadRef
                            | Oper::FreezeRef => {},
                        }
                    },
                    exchange::Instr::Nop => {},
                }
            }
        }
        loops.push(exchange::Loop {
            header,
            members: members.into_iter().collect(),
            val_targets: val_targets.into_iter().collect(),
            mem_targets: mem_targets.into_iter().collect(),
            invariants: vec![],
        });
    }
    Ok(loops)
}
