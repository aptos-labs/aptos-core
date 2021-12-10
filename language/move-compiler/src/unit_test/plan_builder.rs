// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cfgir::ast as G,
    diag,
    expansion::ast::{self as E, Address, ModuleIdent, ModuleIdent_},
    shared::{
        known_attributes::{KnownAttribute, TestingAttribute},
        CompilationEnv, Identifier, NumericalAddress,
    },
    unit_test::{ExpectedFailure, ModuleTestPlan, TestCase},
};
use move_core_types::{account_address::AccountAddress as MoveAddress, value::MoveValue};
use move_ir_types::location::Loc;
use move_symbol_pool::Symbol;
use std::collections::BTreeMap;

struct Context<'env> {
    env: &'env mut CompilationEnv,
}

impl<'env> Context<'env> {
    fn new(compilation_env: &'env mut CompilationEnv) -> Self {
        Self {
            env: compilation_env,
        }
    }

    fn resolve_address(&mut self, addr: &Address) -> NumericalAddress {
        (*addr).into_addr_bytes(self.env.named_address_mapping())
    }
}

//***************************************************************************
// Test Plan Building
//***************************************************************************

// Constructs a test plan for each module in `prog`. This also validates the structure of the
// attributes as the test plan is constructed.
pub fn construct_test_plan(
    compilation_env: &mut CompilationEnv,
    prog: &G::Program,
) -> Option<Vec<ModuleTestPlan>> {
    if !compilation_env.flags().is_testing() {
        return None;
    }
    let mut context = Context::new(compilation_env);
    Some(
        prog.modules
            .key_cloned_iter()
            .flat_map(|(module_ident, module_def)| {
                construct_module_test_plan(&mut context, module_ident, module_def)
            })
            .collect(),
    )
}

fn construct_module_test_plan(
    context: &mut Context,
    module_ident: ModuleIdent,
    module: &G::ModuleDefinition,
) -> Option<ModuleTestPlan> {
    let tests: BTreeMap<_, _> = module
        .functions
        .iter()
        .filter_map(|(loc, fn_name, func)| {
            build_test_info(context, loc, fn_name, func)
                .map(|test_case| (fn_name.to_string(), test_case))
        })
        .collect();

    if tests.is_empty() {
        None
    } else {
        let sp!(_, ModuleIdent_ { address, module }) = &module_ident;
        let addr_bytes = context.resolve_address(address);
        Some(ModuleTestPlan::new(&addr_bytes, &module.0.value, tests))
    }
}

fn build_test_info<'func>(
    context: &mut Context,
    fn_loc: Loc,
    fn_name: &str,
    function: &'func G::Function,
) -> Option<TestCase> {
    let get_attrs = |attr: TestingAttribute| -> Option<&'func E::Attribute> {
        function
            .attributes
            .get_(&E::AttributeName_::Known(KnownAttribute::Testing(attr)))
    };

    const PREVIOUSLY_ANNOTATED_MSG: &str = "Previously annotated here";
    const IN_THIS_TEST_MSG: &str = "Error found in this test";

    let test_attribute_opt = get_attrs(TestingAttribute::Test);
    let abort_attribute_opt = get_attrs(TestingAttribute::ExpectedFailure);
    let test_only_attribute_opt = get_attrs(TestingAttribute::TestOnly);

    let test_attribute = match test_attribute_opt {
        None => {
            // expected failures cannot be annotated on non-#[test] functions
            if let Some(abort_attribute) = abort_attribute_opt {
                let fn_msg = "Only functions defined as a test with #[test] can also have an \
                              #[expected_failure] attribute";
                let abort_msg = "Attributed as #[expected_failure] here";
                context.env.add_diag(diag!(
                    Attributes::InvalidUsage,
                    (fn_loc, fn_msg),
                    (abort_attribute.loc, abort_msg),
                ))
            }
            return None;
        }
        Some(test_attribute) => test_attribute,
    };

    // A #[test] function cannot also be annotated #[test_only]
    if let Some(test_only_attribute) = test_only_attribute_opt {
        let msg = "Function annotated as both #[test(...)] and #[test_only]. You need to declare \
                   it as either one or the other";
        context.env.add_diag(diag!(
            Attributes::InvalidUsage,
            (test_only_attribute.loc, msg),
            (test_attribute.loc, PREVIOUSLY_ANNOTATED_MSG),
            (fn_loc, IN_THIS_TEST_MSG),
        ))
    }

    let test_annotation_params = parse_test_attribute(context, test_attribute, 0);
    let mut arguments = Vec::new();
    for (var, _) in &function.signature.parameters {
        match test_annotation_params.get(&var.value()) {
            Some(value) => arguments.push(value.clone()),
            None => {
                let missing_param_msg = "Missing test parameter assignment in test. Expected a \
                                         parameter to be assigned in this attribute";
                context.env.add_diag(diag!(
                    Attributes::InvalidTest,
                    (test_attribute.loc, missing_param_msg),
                    (var.loc(), "Corresponding to this parameter"),
                    (fn_loc, IN_THIS_TEST_MSG),
                ))
            }
        }
    }

    let expected_failure = match abort_attribute_opt {
        None => None,
        Some(abort_attribute) => parse_failure_attribute(context, abort_attribute),
    };

    Some(TestCase {
        test_name: fn_name.to_string(),
        arguments,
        expected_failure,
    })
}

//***************************************************************************
// Attribute parsers
//***************************************************************************

fn parse_test_attribute(
    context: &mut Context,
    sp!(aloc, test_attribute): &E::Attribute,
    depth: usize,
) -> BTreeMap<Symbol, MoveValue> {
    use E::Attribute_ as EA;

    match test_attribute {
        EA::Name(_) | EA::Parameterized(_, _) if depth > 0 => {
            context.env.add_diag(diag!(
                Attributes::InvalidTest,
                (*aloc, "Unexpected nested attribute in test declaration"),
            ));
            BTreeMap::new()
        }
        EA::Name(nm) => {
            assert!(
                nm.value.as_str() == TestingAttribute::Test.name() && depth == 0,
                "ICE: We should only be parsing a raw test attribute"
            );
            BTreeMap::new()
        }
        EA::Assigned(nm, attr_value) => {
            if depth != 1 {
                context.env.add_diag(diag!(
                    Attributes::InvalidTest,
                    (*aloc, "Unexpected nested attribute in test declaration"),
                ));
                return BTreeMap::new();
            }
            let sp!(assign_loc, attr_value) = &**attr_value;
            let value = match convert_attribute_value_to_move_value(context, attr_value) {
                Some(move_value) => move_value,
                None => {
                    context.env.add_diag(diag!(
                        Attributes::InvalidValue,
                        (*assign_loc, "Unsupported attribute value"),
                        (*aloc, "Assigned in this attribute"),
                    ));
                    return BTreeMap::new();
                }
            };

            let mut args = BTreeMap::new();
            args.insert(nm.value, value);
            args
        }
        EA::Parameterized(nm, attributes) => {
            assert!(
                nm.value.as_str() == TestingAttribute::Test.name() && depth == 0,
                "ICE: We should only be parsing a raw test attribute"
            );
            attributes
                .iter()
                .flat_map(|(_, _, attr)| parse_test_attribute(context, attr, depth + 1))
                .collect()
        }
    }
}

fn parse_failure_attribute(
    context: &mut Context,
    sp!(aloc, expected_attr): &E::Attribute,
) -> Option<ExpectedFailure> {
    use E::{AttributeValue_ as EAV, Attribute_ as EA, Value_ as EV};
    match expected_attr {
        EA::Name(nm) => {
            assert!(
                nm.value.as_str() == TestingAttribute::ExpectedFailure.name(),
                "ICE: We should only be parsing a raw expected failure attribute"
            );
            Some(ExpectedFailure::Expected)
        }
        EA::Assigned(_, value) => {
            let assign_loc = value.loc;
            let invalid_assignment_msg = "Invalid expected failure code assignment";
            let expected_msg = format!(
                "Expect an #[expected_failure({}=...)] attribute for abort code assignment",
                TestingAttribute::CODE_ASSIGNMENT_NAME
            );
            context.env.add_diag(diag!(
                Attributes::InvalidValue,
                (assign_loc, invalid_assignment_msg),
                (*aloc, expected_msg),
            ));
            None
        }
        EA::Parameterized(sp!(_, nm), attrs) => {
            let mut attrs_vec = attrs.iter().map(|(_, _, attr)| attr).collect::<Vec<_>>();
            if attrs_vec.len() != 1 {
                let invalid_attr_msg = format!(
                    "Invalid #[expected_failure(...)] attribute, expected 1 argument but found {}",
                    attrs_vec.len()
                );
                context
                    .env
                    .add_diag(diag!(Attributes::InvalidValue, (*aloc, invalid_attr_msg)));
                return None;
            }
            assert!(
                nm.as_str() == TestingAttribute::ExpectedFailure.name(),
                "ICE: expected failure attribute must have the right name"
            );
            let attr = attrs_vec.pop().unwrap();
            match attr {
                sp!(assign_loc, EA::Assigned(sp!(_, nm), value))
                    if nm.as_str() == TestingAttribute::CODE_ASSIGNMENT_NAME =>
                {
                    match &**value {
                        sp!(_, EAV::Value(sp!(_, EV::InferredNum(u))))
                            if *u <= std::u64::MAX as u128 =>
                        {
                            Some(ExpectedFailure::ExpectedWithCode(*u as u64))
                        }
                        sp!(_, EAV::Value(sp!(_, EV::U64(u)))) => {
                            Some(ExpectedFailure::ExpectedWithCode(*u))
                        }
                        sp!(vloc, EAV::Value(sp!(_, EV::U8(_))))
                        | sp!(vloc, EAV::Value(sp!(_, EV::U128(_)))) => {
                            let msg = "Invalid value in expected failure code assignment";
                            context.env.add_diag(diag!(
                                Attributes::InvalidValue,
                                (*assign_loc, msg),
                                (*vloc, "Annotated non-u64 literals are not permitted"),
                            ));
                            None
                        }
                        sp!(vloc, _) => {
                            context.env.add_diag(diag!(
                                Attributes::InvalidValue,
                                (*vloc, "Invalid value in expected failure code assignment"),
                                (*assign_loc, "Unsupported value in this assignment"),
                            ));
                            None
                        }
                    }
                }
                sp!(assign_loc, EA::Assigned(sp!(nmloc, _), _)) => {
                    let invalid_name_msg = format!(
                        "Invalid name in expected failure code assignment. Did you mean to use \
                         '{}'?",
                        TestingAttribute::CODE_ASSIGNMENT_NAME
                    );
                    context.env.add_diag(diag!(
                        Attributes::InvalidName,
                        (*nmloc, invalid_name_msg),
                        (*assign_loc, "Invalid name in this assignment"),
                    ));
                    None
                }
                sp!(loc, _) => {
                    let msg = "Unsupported attribute value for expected failure attribute";
                    context.env.add_diag(diag!(
                        Attributes::InvalidValue,
                        (*aloc, msg),
                        (*loc, "Unsupported value in this assignment")
                    ));
                    None
                }
            }
        }
    }
}

fn convert_attribute_value_to_move_value(
    context: &mut Context,
    value: &E::AttributeValue_,
) -> Option<MoveValue> {
    use E::{AttributeValue_ as EAV, Value_ as EV};
    match value {
        // Only addresses are allowed
        EAV::Value(sp!(_, EV::Address(a))) => Some(MoveValue::Address(MoveAddress::new(
            context.resolve_address(a).into_bytes(),
        ))),
        _ => None,
    }
}
