use move_compiler::parser::ast;
use move_compiler::parser::ast::{
    Definition::{Address, Module, Script},
    Exp, FunctionBody_, ModuleMember, SequenceItem_,
};

use crate::mutant::Mutant;
use crate::operator::MutationOperator;
use crate::operators::binary::BinaryOperator;
use crate::operators::unary::UnaryOperator;

/// Traverses the AST, identifies places where mutation operators can be applied
/// and returns a list of mutants.
pub fn mutate(ast: ast::Program) -> anyhow::Result<Vec<Mutant>> {
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
fn traverse_module(module: ast::ModuleDefinition) -> anyhow::Result<Vec<Mutant>> {
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
fn traverse_function(function: ast::Function) -> anyhow::Result<Vec<Mutant>> {
    trace!("Traversing function {}", function.name);
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
    let (_, seq, _, exp) = elem;
    let mut mutants = seq
        .into_iter()
        .map(traverse_sequence_item)
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    // exp represents the return expression so we need to remember to parse it.
    if let Some(exp) = *exp {
        mutants.extend(parse_expression_and_find_mutants(exp)?);
    }

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
        SequenceItem_::Declare(_bl, _type) => Ok(vec![]),
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
    match exp.value {
        ast::Exp_::BinopExp(left, binop, right) => {
            // Parse left and right side of the operator as they are expressions and may contain
            // another things to mutate.
            let mut mutants = parse_expression_and_find_mutants(*left)?;
            mutants.extend(parse_expression_and_find_mutants(*right)?);

            // Add the mutation operator to the list of mutants.
            mutants.push(Mutant::new(MutationOperator::BinaryOperator(
                BinaryOperator::new(binop),
            )));

            trace!("Found possible mutation in BinaryExp {binop:?}");

            Ok(mutants)
        },
        ast::Exp_::UnaryExp(unop, exp) => {
            // Parse the expression as it may contain another things to mutate.
            let mut mutants = parse_expression_and_find_mutants(*exp)?;

            // Add the mutation operator to the list of mutants.
            mutants.push(Mutant::new(MutationOperator::UnaryOperator(
                UnaryOperator::new(unop),
            )));

            trace!("Found possible mutation in UnaryExp {unop:?}");

            Ok(mutants)
        },
        ast::Exp_::Assign(exp1, exp2) | ast::Exp_::While(exp1, exp2) => {
            let mut mutants = parse_expression_and_find_mutants(*exp1)?;
            mutants.extend(parse_expression_and_find_mutants(*exp2)?);
            Ok(mutants)
        },
        ast::Exp_::Block(seq) => traverse_sequence(seq),
        ast::Exp_::Pack(_, _, exps) => {
            let exps = exps.into_iter().map(|(_, exp)| exp).collect::<Vec<Exp>>();
            parse_expressions(exps)
        },
        ast::Exp_::Call(_, _, _, exps) | ast::Exp_::Vector(_, _, exps) => {
            parse_expressions(exps.value)
        },
        ast::Exp_::ExpList(exps) => parse_expressions(exps),
        ast::Exp_::IfElse(exp1, exp2, exp3) => {
            let mut mutants = parse_expression_and_find_mutants(*exp1)?;
            mutants.extend(parse_expression_and_find_mutants(*exp2)?);
            if let Some(exp3) = exp3 {
                mutants.extend(parse_expression_and_find_mutants(*exp3)?);
            }
            Ok(mutants)
        },
        ast::Exp_::Quant(_, _, vexp, lexp, exp) => {
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
        ast::Exp_::Abort(exp)
        | ast::Exp_::Annotate(exp, _)
        | ast::Exp_::Borrow(_, exp)
        | ast::Exp_::Cast(exp, _)
        | ast::Exp_::Dereference(exp)
        | ast::Exp_::Dot(exp, _)
        | ast::Exp_::Loop(exp)
        | ast::Exp_::Lambda(_, exp)
        | ast::Exp_::Return(Some(exp)) => parse_expression_and_find_mutants(*exp),
        ast::Exp_::Value(_)
        | ast::Exp_::Move(_)
        | ast::Exp_::Copy(_)
        | ast::Exp_::Name(_, _)
        | ast::Exp_::Unit
        | ast::Exp_::Break
        | ast::Exp_::Continue
        | ast::Exp_::Spec(_)
        | ast::Exp_::Index(_, _)
        | ast::Exp_::UnresolvedError
        | ast::Exp_::Return(None) => Ok(vec![]),
    }
}
