use crate::cli;
use crate::configuration::{Configuration, IncludeFunctions};
use move_compiler::diagnostics::FilesSourceText;
use move_compiler::typing::ast;
use move_compiler::typing::ast::{Exp, ExpListItem, FunctionBody_, SequenceItem_};
use move_compiler::{expansion, parser};
use std::path::Path;

use crate::mutant::Mutant;
use crate::operator::MutationOp;
use crate::operators::binary::Binary;
use crate::operators::break_continue::BreakContinue;
use crate::operators::unary::Unary;

/// Traverses the AST, identifies places where mutation operators can be applied
/// and returns a list of mutants.
pub fn mutate(
    ast: ast::Program,
    conf: &Configuration,
    files: &FilesSourceText,
) -> anyhow::Result<Vec<Mutant>> {
    trace!("Starting mutation process");
    let mut mutants = ast
        .modules
        .into_iter()
        .map(|module| traverse_module_with_check(module, conf, files))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    mutants.extend(
        ast.scripts
            .into_iter()
            .map(|script| {
                traverse_function((script.1.function_name, script.1.function), conf, files)
            })
            .collect::<Result<Vec<_>, _>>()?
            .concat(),
    );

    trace!("Found {} possible mutations", mutants.len());

    Ok(mutants)
}

/// Traverses a single module and returns a list of mutants - helper function which filter out modules
/// that are not included in the configuration.
#[inline]
fn traverse_module_with_check(
    module: (expansion::ast::ModuleIdent, ast::ModuleDefinition),
    conf: &Configuration,
    files: &FilesSourceText,
) -> anyhow::Result<Vec<Mutant>> {
    if let cli::ModuleFilter::Selected(mods) = &conf.project.mutate_modules {
        if !mods.contains(&module.0.value.module.to_string()) {
            trace!("Skipping module {}", &module.0.value.module.to_string());
            return Ok(vec![]);
        }
    }

    traverse_module(module, conf, files)
}

/// Traverses a single module and returns a list of mutants.
/// Checks all the functions and constants defined in the module.
fn traverse_module(
    module: (expansion::ast::ModuleIdent, ast::ModuleDefinition),
    conf: &Configuration,
    files: &FilesSourceText,
) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing module {}", module.0.value.module.to_string());
    let mut mutants = module
        .1
        .functions
        .into_iter()
        .map(|func| traverse_function(func, conf, files))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    mutants.extend(
        module
            .1
            .constants
            .into_iter()
            .map(|constant| parse_expression_and_find_mutants(constant.1.value))
            .collect::<Result<Vec<_>, _>>()?
            .concat(),
    );

    // Set the module name for all the mutants.
    mutants
        .iter_mut()
        .for_each(|m| m.set_module_name(module.0.value.module.clone()));

    trace!(
        "Found {} possible mutations in module {}",
        mutants.len(),
        module.0.value.module.clone()
    );
    Ok(mutants)
}

/// Traverses a single function and returns a list of mutants.
/// Checks the body of the function by traversing all the sequences.
fn traverse_function(
    function: (parser::ast::FunctionName, ast::Function),
    conf: &Configuration,
    files: &FilesSourceText,
) -> anyhow::Result<Vec<Mutant>> {
    let (filename, _) = files.get(&function.1.body.loc.file_hash()).unwrap(); // File must exist inside the hashmap so it's safe to unwrap.

    // Check if function is included in individual configuration.
    if let Some(ind) = conf.get_file_configuration(Path::new(filename.as_str())) {
        if let IncludeFunctions::Selected(funcs) = &ind.include_functions {
            if !funcs.contains(&function.0 .0.to_string()) {
                trace!("Skipping function {}", &function.0 .0.to_string());
                return Ok(vec![]);
            }
        }
    }

    trace!("Traversing function {}", &function.0 .0.to_string());
    match function.1.body.value {
        FunctionBody_::Defined(elem) => traverse_sequence(elem),
        FunctionBody_::Native => Ok(vec![]),
    }
}

/// Traverses a sequence and returns a list of mutants.
/// Checks all the sequence items by calling `traverse_sequence_item` on them. Sequence can also contain
/// return expression which needs to be also examined if it can be mutated.
fn traverse_sequence(elem: ast::Sequence) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing sequence {elem:?}");

    let mutants = elem
        .into_iter()
        .map(traverse_sequence_item)
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    trace!("Found {} possible mutations in sequence", mutants.len());
    Ok(mutants)
}

/// Traverses a single sequence item and returns a list of mutants.
/// Checks if binds or sequence items contain expressions that can be mutated by calling appropriate function on them..
fn traverse_sequence_item(seq_item: ast::SequenceItem) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing sequence item {:?}", seq_item);
    match seq_item.value {
        SequenceItem_::Bind(_, _, exp) | SequenceItem_::Seq(exp) => {
            parse_expression_and_find_mutants(*exp)
        },
        SequenceItem_::Declare(_bl) => Ok(vec![]),
    }
}

/// Helper function that parses a list of expressions and returns a list of mutants.
fn parse_expressions(exp: Vec<Exp>) -> anyhow::Result<Vec<Mutant>> {
    trace!("Parsing expressions {exp:?}");
    Ok(exp
        .into_iter()
        .map(parse_expression_and_find_mutants)
        .collect::<Result<Vec<_>, _>>()?
        .concat())
}

/// This function does the actual parsing of the expression and checks if any of the mutation operators
/// can be applied to it.
/// In case if the expression contains another expressions, it calls itself recursively.
/// When Move language is extended with new expressions, this function needs to be updated to support them.
fn parse_expression_and_find_mutants(exp: Exp) -> anyhow::Result<Vec<Mutant>> {
    trace!("Parsing expression {exp:?}");
    match exp.exp.value {
        ast::UnannotatedExp_::BinopExp(left, binop, _type, right) => {
            // Parse left and right side of the operator as they are expressions and may contain
            // another things to mutate.
            let mut mutants = parse_expression_and_find_mutants(*left)?;
            mutants.extend(parse_expression_and_find_mutants(*right)?);

            // Add the mutation operator to the list of mutants.
            mutants.push(Mutant::new(MutationOp::BinaryOp(Binary::new(binop)), None));

            trace!("Found possible mutation in BinaryExp {binop:?}");

            Ok(mutants)
        },
        ast::UnannotatedExp_::UnaryExp(unop, exp) => {
            // Parse the expression as it may contain another things to mutate.
            let mut mutants = parse_expression_and_find_mutants(*exp)?;

            // Add the mutation operator to the list of mutants.
            mutants.push(Mutant::new(MutationOp::UnaryOp(Unary::new(unop)), None));

            trace!("Found possible mutation in UnaryExp {unop:?}");

            Ok(mutants)
        },
        ast::UnannotatedExp_::Assign(_, _, exp) => {
            let mutants = parse_expression_and_find_mutants(*exp)?;
            Ok(mutants)
        },
        ast::UnannotatedExp_::While(exp1, exp2) | ast::UnannotatedExp_::Mutate(exp1, exp2) => {
            let mut mutants = parse_expression_and_find_mutants(*exp1)?;
            mutants.extend(parse_expression_and_find_mutants(*exp2)?);
            Ok(mutants)
        },
        ast::UnannotatedExp_::Block(seq) => traverse_sequence(seq),
        ast::UnannotatedExp_::Pack(_, _, _, exps) => {
            let exps = exps
                .into_iter()
                .map(|(_, exp)| exp.1 .1)
                .collect::<Vec<Exp>>();
            parse_expressions(exps)
        },
        ast::UnannotatedExp_::Vector(_, _, _, exp) => parse_expression_and_find_mutants(*exp),
        ast::UnannotatedExp_::ExpList(exps) => {
            let exps = exps
                .into_iter()
                .map(|exp| match exp {
                    ExpListItem::Single(exp, _) => exp,
                    ExpListItem::Splat(_, exp, _) => exp,
                })
                .collect::<Vec<Exp>>();
            parse_expressions(exps)
        },
        ast::UnannotatedExp_::IfElse(exp1, exp2, exp3) => {
            let mut mutants = parse_expression_and_find_mutants(*exp1)?;
            mutants.extend(parse_expression_and_find_mutants(*exp2)?);
            mutants.extend(parse_expression_and_find_mutants(*exp3)?);
            Ok(mutants)
        },
        ast::UnannotatedExp_::Break | ast::UnannotatedExp_::Continue => {
            let mut mutants = vec![];
            mutants.push(Mutant::new(
                MutationOp::BreakContinue(BreakContinue::new(exp)),
                None,
            ));
            Ok(mutants)
        },
        ast::UnannotatedExp_::Abort(exp)
        | ast::UnannotatedExp_::Annotate(exp, _)
        | ast::UnannotatedExp_::Borrow(_, exp, _)
        | ast::UnannotatedExp_::Cast(exp, _)
        | ast::UnannotatedExp_::Dereference(exp)
        | ast::UnannotatedExp_::Lambda(_, exp)
        | ast::UnannotatedExp_::Return(exp)
        | ast::UnannotatedExp_::VarCall(_, exp)
        | ast::UnannotatedExp_::Builtin(_, exp)
        | ast::UnannotatedExp_::TempBorrow(_, exp) => parse_expression_and_find_mutants(*exp),
        ast::UnannotatedExp_::Loop { has_break: _, body } => {
            parse_expression_and_find_mutants(*body)
        },
        ast::UnannotatedExp_::Unit { .. }
        | ast::UnannotatedExp_::Copy { .. }
        | ast::UnannotatedExp_::Move { .. }
        | ast::UnannotatedExp_::Use(_)
        | ast::UnannotatedExp_::Constant(_, _)
        | ast::UnannotatedExp_::ModuleCall(_)
        | ast::UnannotatedExp_::BorrowLocal(_, _)
        | ast::UnannotatedExp_::Spec(_)
        | ast::UnannotatedExp_::Value(_)
        | ast::UnannotatedExp_::UnresolvedError => Ok(vec![]),
    }
}
