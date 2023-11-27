use move_compiler::parser;
use move_compiler::parser::ast::{Exp, FunctionBody_, SequenceItem_};

use crate::mutant::Mutant;
use crate::operator::MutationOperator;
use move_compiler::parser::ast::{
    Definition::{Address, Module, Script},
    ModuleMember,
};

/// Traverses the AST, identifies places where mutation operators can be applied
/// and returns a list of mutants.
pub fn mutate(ast: parser::ast::Program) -> anyhow::Result<Vec<Mutant>> {
    let mutants = ast
        .source_definitions
        .into_iter()
        .flat_map(|package| match package.def {
            Address(addr) => addr
                .modules
                .into_iter()
                .map(traverse_module)
                .collect::<Vec<Result<Vec<_>, _>>>(),
            Module(module) => vec![traverse_module(module)],
            Script(script) => vec![traverse_function(script.function)],
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
            ModuleMember::Constant(constant) => Some(parse_expression(constant.value)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    Ok(mutants)
}

fn traverse_function(function: parser::ast::Function) -> anyhow::Result<Vec<Mutant>> {
    match function.body.value {
        FunctionBody_::Defined(elem) => traverse_sequence(elem),
        _ => Ok(vec![]),
    }
}

fn traverse_sequence(elem: parser::ast::Sequence) -> anyhow::Result<Vec<Mutant>> {
    let (_, seq, _, exp) = elem;
    let mut mutants = seq
        .into_iter()
        .map(traverse_sequence_item)
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    // exp represents the return expression so we need to remember to parse it
    if let Some(exp) = *exp {
        mutants.extend(parse_expression(exp)?);
    }

    Ok(mutants)
}

fn traverse_sequence_item(seq_item: parser::ast::SequenceItem) -> anyhow::Result<Vec<Mutant>> {
    match seq_item.value {
        SequenceItem_::Bind(_, _, exp) | SequenceItem_::Seq(exp) => parse_expression(*exp),
        SequenceItem_::Declare(_bl, _type) => Ok(vec![]),
    }
}

fn parse_expressions(exp: Vec<parser::ast::Exp>) -> anyhow::Result<Vec<Mutant>> {
    Ok(exp
        .into_iter()
        .map(parse_expression)
        .collect::<Result<Vec<_>, _>>()?
        .concat())
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
        parser::ast::Exp_::Assign(exp1, exp2) | parser::ast::Exp_::While(exp1, exp2) => {
            let mut mutants = parse_expression(*exp1)?;
            mutants.extend(parse_expression(*exp2)?);
            Ok(mutants)
        },
        parser::ast::Exp_::Block(seq) => traverse_sequence(seq),
        parser::ast::Exp_::Pack(_, _, exps) => {
            let exps = exps.into_iter().map(|(_, exp)| exp).collect::<Vec<Exp>>();
            parse_expressions(exps)
        },
        parser::ast::Exp_::Call(_, _, _, exps) | parser::ast::Exp_::Vector(_, _, exps) => parse_expressions(exps.value),
        parser::ast::Exp_::ExpList(exps) => parse_expressions(exps),
        parser::ast::Exp_::IfElse(exp1, exp2, exp3) => {
            let mut mutants = parse_expression(*exp1)?;
            mutants.extend(parse_expression(*exp2)?);
            if let Some(exp3) = exp3 {
                mutants.extend(parse_expression(*exp3)?);
            }
            Ok(mutants)
        },
        parser::ast::Exp_::Quant(_, _, vexp, lexp, exp) => {
            let mut mutants = vec![];
            for exp in vexp {
                let muts = parse_expressions(exp)?;
                mutants.extend(muts);
            }
            if let Some(lexp) = lexp {
                mutants.extend(parse_expression(*lexp)?);
            }
            mutants.extend(parse_expression(*exp)?);
            Ok(mutants)
        },
        parser::ast::Exp_::Return(Some(exp)) => parse_expression(*exp),
        parser::ast::Exp_::Abort(exp)
        | parser::ast::Exp_::Annotate(exp, _)
        | parser::ast::Exp_::Borrow(_, exp)
        | parser::ast::Exp_::Cast(exp, _)
        | parser::ast::Exp_::Dereference(exp)
        | parser::ast::Exp_::Dot(exp, _)
        | parser::ast::Exp_::Loop(exp)
        | parser::ast::Exp_::Lambda(_, exp) => parse_expression(*exp),
        _ => Ok(vec![]),
    }
}
