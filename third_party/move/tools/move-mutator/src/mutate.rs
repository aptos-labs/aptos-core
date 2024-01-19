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
    trace!("Starting mutation process");
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

    trace!("Found {} possible mutations", mutants.len());

    Ok(mutants)
}

/// Traverses a single module and returns a list of mutants.
/// Checks all the functions and constants defined in the module.
fn traverse_module(module: parser::ast::ModuleDefinition) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing module {}", module.name);
    let mutants = module
        .members
        .into_iter()
        .filter_map(|member| match member {
            ModuleMember::Function(func) => Some(traverse_function(func)),
            ModuleMember::Constant(constant) => {
                Some(parse_expression_and_find_mutants(constant.value))
            },
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    trace!(
        "Found {} possible mutations in module {}",
        mutants.len(),
        module.name
    );
    Ok(mutants)
}

/// Traverses a single function and returns a list of mutants.
/// Checks the body of the function by traversing all the sequences.
fn traverse_function(function: parser::ast::Function) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing function {}", function.name);
    match function.body.value {
        FunctionBody_::Defined(elem) => traverse_sequence(elem),
        FunctionBody_::Native => Ok(vec![]),
    }
}

/// Traverses a sequence and returns a list of mutants.
/// Checks all the sequence items by calling `traverse_sequence_item` on them. Sequence can also contain
/// return expression which needs to be also examined if it can be mutated..
fn traverse_sequence(elem: parser::ast::Sequence) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing sequence {:?}", elem);
    let (_, seq, _, exp) = elem;
    let mut mutants = seq
        .into_iter()
        .map(traverse_sequence_item)
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    // exp represents the return expression so we need to remember to parse it
    if let Some(exp) = *exp {
        mutants.extend(parse_expression_and_find_mutants(exp)?);
    }

    trace!("Found {} possible mutations in sequence", mutants.len());
    Ok(mutants)
}

/// Traverses a single sequence item and returns a list of mutants.
/// Checks if binds or sequence items contain expressions that can be mutated by calling appropriate function on them..
fn traverse_sequence_item(seq_item: parser::ast::SequenceItem) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing sequence item {:?}", seq_item);
    match seq_item.value {
        SequenceItem_::Bind(_, _, exp) | SequenceItem_::Seq(exp) => {
            parse_expression_and_find_mutants(*exp)
        },
        SequenceItem_::Declare(_bl, _type) => Ok(vec![]),
    }
}

/// Helper function that parses a list of expressions and returns a list of mutants.
fn parse_expressions(exp: Vec<parser::ast::Exp>) -> anyhow::Result<Vec<Mutant>> {
    trace!("Parsing expressions {:?}", exp);
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
fn parse_expression_and_find_mutants(exp: parser::ast::Exp) -> anyhow::Result<Vec<Mutant>> {
    trace!("Parsing expression {:?}", exp);
    match exp.value {
        parser::ast::Exp_::BinopExp(left, binop, right) => {
            // Parse left and right side of the operator as they are expressions and may contain
            // another things to mutate
            let mut mutants = parse_expression_and_find_mutants(*left)?;
            mutants.extend(parse_expression_and_find_mutants(*right)?);

            // Add the mutation operator to the list of mutants
            mutants.push(Mutant::new(MutationOperator::BinaryOperator(binop)));

            trace!("Found possible mutation in BinaryExp {:?}", binop);

            Ok(mutants)
        },
        parser::ast::Exp_::UnaryExp(unop, exp) => {
            // Parse the expression as it may contain another things to mutate
            let mut mutants = parse_expression_and_find_mutants(*exp)?;

            // Add the mutation operator to the list of mutants
            mutants.push(Mutant::new(MutationOperator::UnaryOperator(unop)));

            trace!("Found possible mutation in UnaryExp {:?}", unop);

            Ok(mutants)
        },
        parser::ast::Exp_::Assign(exp1, exp2) | parser::ast::Exp_::While(exp1, exp2) => {
            let mut mutants = parse_expression_and_find_mutants(*exp1)?;
            mutants.extend(parse_expression_and_find_mutants(*exp2)?);
            Ok(mutants)
        },
        parser::ast::Exp_::Block(seq) => traverse_sequence(seq),
        parser::ast::Exp_::Pack(_, _, exps) => {
            let exps = exps.into_iter().map(|(_, exp)| exp).collect::<Vec<Exp>>();
            parse_expressions(exps)
        },
        parser::ast::Exp_::Call(_, _, _, exps) | parser::ast::Exp_::Vector(_, _, exps) => {
            parse_expressions(exps.value)
        },
        parser::ast::Exp_::ExpList(exps) => parse_expressions(exps),
        parser::ast::Exp_::IfElse(exp1, exp2, exp3) => {
            let mut mutants = parse_expression_and_find_mutants(*exp1)?;
            mutants.extend(parse_expression_and_find_mutants(*exp2)?);
            if let Some(exp3) = exp3 {
                mutants.extend(parse_expression_and_find_mutants(*exp3)?);
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
                mutants.extend(parse_expression_and_find_mutants(*lexp)?);
            }
            mutants.extend(parse_expression_and_find_mutants(*exp)?);
            Ok(mutants)
        },
        parser::ast::Exp_::Abort(exp)
        | parser::ast::Exp_::Annotate(exp, _)
        | parser::ast::Exp_::Borrow(_, exp)
        | parser::ast::Exp_::Cast(exp, _)
        | parser::ast::Exp_::Dereference(exp)
        | parser::ast::Exp_::Dot(exp, _)
        | parser::ast::Exp_::Loop(exp)
        | parser::ast::Exp_::Lambda(_, exp)
        | parser::ast::Exp_::Return(Some(exp)) => parse_expression_and_find_mutants(*exp),
        parser::ast::Exp_::Value(_)
        | parser::ast::Exp_::Move(_)
        | parser::ast::Exp_::Copy(_)
        | parser::ast::Exp_::Name(_, _)
        | parser::ast::Exp_::Unit
        | parser::ast::Exp_::Break
        | parser::ast::Exp_::Continue
        | parser::ast::Exp_::Spec(_)
        | parser::ast::Exp_::Index(_, _)
        | parser::ast::Exp_::UnresolvedError
        | parser::ast::Exp_::Return(None) => Ok(vec![]),
    }
}
