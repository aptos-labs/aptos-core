use move_compiler::parser;
use move_compiler::parser::ast::{FunctionBody_, SequenceItem_};

use crate::mutant::Mutant;
use crate::operator::MutationOperator;
use move_compiler::parser::ast::{Definition::Module, ModuleMember};

/// Traverses the AST, identifies places where mutation operators can be applied
/// and returns a list of mutants.
pub fn mutate(ast: parser::ast::Program) -> anyhow::Result<Vec<Mutant>> {
    let mutants = ast
        .source_definitions
        .into_iter()
        .filter_map(|package| match package.def {
            Module(module) => Some(traverse_module(module)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    Ok(mutants)
}

fn traverse_module(module: parser::ast::ModuleDefinition) -> anyhow::Result<Vec<Mutant>> {
    let mutants = module
        .members
        .into_iter()
        .filter_map(|member| match member {
            ModuleMember::Function(func) => Some(traverse_function(func)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    Ok(mutants)
}

fn traverse_function(function: parser::ast::Function) -> anyhow::Result<Vec<Mutant>> {
    match function.body.value {
        FunctionBody_::Defined(elem) => {
            let (_, seq, _, exp) = elem;
            let mut mutants = seq
                .into_iter()
                .map(traverse_sequence_item)
                .collect::<Result<Vec<_>, _>>()?
                .concat();

            // exp represents the return expression so we need to remember to parse it
            mutants.extend(parse_expression(exp.unwrap())?);
            Ok(mutants)
        },
        _ => Ok(vec![]),
    }
}

fn traverse_sequence_item(seq_item: parser::ast::SequenceItem) -> anyhow::Result<Vec<Mutant>> {
    match seq_item.value {
        SequenceItem_::Bind(_, _, exp) | SequenceItem_::Seq(exp) => parse_expression(*exp),
        SequenceItem_::Declare(_bl, _type) => Ok(vec![]),
    }
}

fn parse_expression(exp: parser::ast::Exp) -> anyhow::Result<Vec<Mutant>> {
    match exp.value {
        parser::ast::Exp_::BinopExp(left, binop, right) => {
            // Parse left and right side of the operator as they are expressions and may contain
            // another things to mutate
            let mut mutants = parse_expression(*left)?;
            mutants.extend(parse_expression(*right)?);

            // Add the mutation operator to the list of mutants
            mutants.push(Mutant::new(MutationOperator::BinaryOperator(binop)));

            Ok(mutants)
        },
        parser::ast::Exp_::UnaryExp(unop, exp) => {
            // Parse the expression as it may contain another things to mutate
            let mut mutants = parse_expression(*exp)?;

            // Add the mutation operator to the list of mutants
            mutants.push(Mutant::new(MutationOperator::UnaryOperator(unop)));

            Ok(mutants)
        },
        _ => Ok(vec![]),
    }
}
