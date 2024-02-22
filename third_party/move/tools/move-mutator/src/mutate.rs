use crate::cli;
use crate::configuration::{Configuration, IncludeFunctions};
use move_compiler::diagnostics::FilesSourceText;
use move_compiler::parser::ast::{ConstantName, FunctionName, ModuleName};
use move_compiler::shared::unique_map::UniqueMap;
use move_compiler::typing::ast;
use move_compiler::typing::ast::{
    Constant, Exp, ExpListItem, Function, FunctionBody_, ModuleDefinition, SequenceItem_,
};
use move_compiler::{expansion, parser};
use move_package::source_package::layout::SourcePackageLayout;
use std::path::Path;

use crate::mutant::Mutant;
use crate::operator::MutationOp;
use crate::operators::binary::Binary;
use crate::operators::binary_swap::BinarySwap;
use crate::operators::break_continue::BreakContinue;
use crate::operators::delete_stmt::DeleteStmt;
use crate::operators::ifelse::IfElse;
use crate::operators::literal::Literal;
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
        .map(|module| traverse_module_with_check(&module, conf, files))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    mutants.extend(
        ast.scripts
            .into_values()
            .map(|script| traverse_function((script.function_name, script.function), conf, files))
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
    module: &(expansion::ast::ModuleIdent, ast::ModuleDefinition),
    conf: &Configuration,
    files: &FilesSourceText,
) -> anyhow::Result<Vec<Mutant>> {
    // We need to check if module comes from our source tree or from the deps, as we don't want to traverse
    // all the dependencies. That's a bit tricky as global deps are easy to identify but local deps can be
    // anywhere near the project tree.
    let (ident, _) = &module;
    let (filename, _) = files.get(&ident.loc.file_hash()).unwrap(); // File must exist inside the hashmap so it's safe to unwrap.
    let filename_path = Path::new(filename.as_str());

    if !conf.project.move_sources.is_empty()
        && !conf
            .project
            .move_sources
            .contains(&filename_path.to_path_buf())
    {
        trace!(
            "Skipping module {} as it does not come from source project",
            filename
        );
        return Ok(vec![]);
    }

    if conf.project.move_sources.is_empty() {
        let test_root = SourcePackageLayout::try_find_root(&filename_path.canonicalize()?)?;
        if let Some(project_path) = &conf.project_path {
            let project_path = project_path.canonicalize()?;
            if test_root != project_path {
                trace!(
                    "Skipping module: \n {} \n root: {} \n as it does not come from source project {}",
                    filename_path.to_string_lossy(), test_root.to_string_lossy(),
                    project_path.to_string_lossy()
                );
                return Ok(vec![]);
            }
        }
    }

    // Now we need to check if the module is included in the configuration.
    let module_name = extract_module_name(module);
    if let cli::ModuleFilter::Selected(mods) = &conf.project.mutate_modules {
        if !mods.contains(&module_name.to_string()) {
            trace!("Skipping module {}", module_name.to_string());
            return Ok(vec![]);
        }
    }

    traverse_module(module, conf, files)
}

/// Internal helper function that returns a reference to the functions defined in the module.
fn functions_of(
    module: &(expansion::ast::ModuleIdent, ast::ModuleDefinition),
) -> &UniqueMap<FunctionName, Function> {
    let (_, module) = module;
    let ModuleDefinition { functions, .. } = module;
    functions
}

/// Internal helper function that returns a reference to the constants defined in the module.
fn constants_of(
    module: &(expansion::ast::ModuleIdent, ast::ModuleDefinition),
) -> &UniqueMap<ConstantName, Constant> {
    let (_, module) = module;
    let ModuleDefinition { constants, .. } = module;
    constants
}

/// Extracts the module name from the module declaration.
fn extract_module_name(
    module: &(expansion::ast::ModuleIdent, ast::ModuleDefinition),
) -> ModuleName {
    let (module_ident, _) = module;
    module_ident.value.module
}

/// Traverses a single module and returns a list of mutants.
/// Checks all the functions and constants defined in the module.
#[allow(clippy::unnecessary_to_owned)]
fn traverse_module(
    module: &(expansion::ast::ModuleIdent, ast::ModuleDefinition),
    conf: &Configuration,
    files: &FilesSourceText,
) -> anyhow::Result<Vec<Mutant>> {
    let module_name = extract_module_name(module);
    trace!("Traversing module {}", &module_name.to_string());
    let mut mutants = functions_of(module)
        .to_owned()
        .into_iter()
        .map(|func| traverse_function(func, conf, files))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    mutants.extend(
        constants_of(module)
            .to_owned()
            .into_iter()
            .map(|(_, constant)| parse_expression_and_find_mutants(constant.value))
            .collect::<Result<Vec<_>, _>>()?
            .concat(),
    );

    // Set the module name for all the mutants.
    mutants
        .iter_mut()
        .for_each(|m| m.set_module_name(module_name));

    trace!(
        "Found {} possible mutations in module {}",
        mutants.len(),
        module_name
    );
    Ok(mutants)
}

/// Extracts the function name from the function declaration.
fn extract_function_name(function: &(FunctionName, Function)) -> String {
    let (function_name, _) = function;
    let FunctionName(name) = function_name;
    name.to_string()
}

/// Traverses a single function and returns a list of mutants.
/// Checks the body of the function by traversing all the sequences.
fn traverse_function(
    function: (parser::ast::FunctionName, ast::Function),
    conf: &Configuration,
    files: &FilesSourceText,
) -> anyhow::Result<Vec<Mutant>> {
    let function_name = extract_function_name(&function);
    let (_, function) = function;
    let (filename, _) = files.get(&function.body.loc.file_hash()).unwrap(); // File must exist inside the hashmap so it's safe to unwrap.

    // Check if function is included in individual configuration.
    if let Some(ind) = conf.get_file_configuration(Path::new(filename.as_str())) {
        if let IncludeFunctions::Selected(funcs) = &ind.include_functions {
            if !funcs.contains(&function_name) {
                trace!("Skipping function {}", &function_name);
                return Ok(vec![]);
            }
        }
    }

    trace!("Traversing function {}", &function_name);
    match function.body.value {
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
    match exp.clone().exp.value {
        ast::UnannotatedExp_::BinopExp(left, binop, _type, right) => {
            // Parse left and right side of the operator as they are expressions and may contain
            // another things to mutate.
            let mut mutants = parse_expression_and_find_mutants(*left.clone())?;
            mutants.extend(parse_expression_and_find_mutants(*right.clone())?);

            // Add the mutation operator to the list of mutants.
            mutants.push(Mutant::new(MutationOp::BinaryOp(Binary::new(binop)), None));
            mutants.push(Mutant::new(
                MutationOp::BinarySwap(BinarySwap::new(binop, *left, *right)),
                None,
            ));

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
        ast::UnannotatedExp_::ExpList(exps) => {
            let exps = exps
                .into_iter()
                .map(|exp| match exp {
                    ExpListItem::Single(exp, _) | ExpListItem::Splat(_, exp, _) => exp,
                })
                .collect::<Vec<Exp>>();
            parse_expressions(exps)
        },
        ast::UnannotatedExp_::IfElse(exp1, exp2, exp3) => {
            let mut mutants = parse_expression_and_find_mutants(*exp1)?;
            mutants.extend(parse_expression_and_find_mutants(*exp2)?);
            mutants.extend(parse_expression_and_find_mutants(*exp3)?);
            mutants.push(Mutant::new(MutationOp::IfElse(IfElse::new(exp)), None));
            Ok(mutants)
        },
        ast::UnannotatedExp_::Break | ast::UnannotatedExp_::Continue => Ok(vec![Mutant::new(
            MutationOp::BreakContinue(BreakContinue::new(exp)),
            None,
        )]),
        ast::UnannotatedExp_::Value(val) => {
            let mutants = vec![Mutant::new(MutationOp::Literal(Literal::new(val)), None)];
            Ok(mutants)
        },
        ast::UnannotatedExp_::Builtin(_, expr) => {
            let mut mutants = parse_expression_and_find_mutants(*expr)?;
            mutants.push(Mutant::new(
                MutationOp::DeleteStmt(DeleteStmt::new(exp.clone())),
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
        | ast::UnannotatedExp_::Vector(_, _, _, exp)
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
        | ast::UnannotatedExp_::UnresolvedError => Ok(vec![]),
    }
}
