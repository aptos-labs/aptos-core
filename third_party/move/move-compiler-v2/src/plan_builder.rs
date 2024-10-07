// Copyright (c) Aptos Labs
// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Build a vector of module test plans for a Move program compiled with V2.
//!
//! This reimplements move-compiler/src/unit_test/plan_builder.rs in terms
//! of data structures available in V2's `GlobalEnv` structure after AST processing.
//!
//! Each module containing any labeled `#[test]` functions gets an item in the output list, which
//! includes info about each '#[test]' function: name, arguments to provide, and expected failure or
//! success.

use crate::options::Options;
use codespan_reporting::diagnostic::Severity;
use move_command_line_common::{address::NumericalAddress, parser::NumberFormat};
use move_compiler::{
    shared::known_attributes::{AttributeKind, TestingAttribute},
    unit_test::{ExpectedFailure, ExpectedMoveError, ModuleTestPlan, TestCase},
};
use move_core_types::{
    identifier::Identifier, language_storage::ModuleId, value::MoveValue, vm_status::StatusCode,
};
use move_model::{
    ast::{Address, Attribute, AttributeValue, ModuleName, Value},
    model::{FunctionEnv, GlobalEnv, Loc, ModuleEnv, Parameter},
    symbol::Symbol,
    ty::{PrimitiveType, Type},
};
use num::{BigInt, ToPrimitive};
use std::collections::BTreeMap;

//***************************************************************************
// Test Plan Building
//***************************************************************************

// Constructs a test plan for each module in `env.target`. This also validates the structure of the
// attributes as the test plan is constructed.
pub fn construct_test_plan(
    env: &GlobalEnv,
    package_filter: Option<Symbol>,
) -> Option<Vec<ModuleTestPlan>> {
    let options = env.get_extension::<Options>().expect("options");
    if !options.compile_test_code {
        return None;
    }

    Some(
        env.get_modules()
            .filter_map(|module| {
                if module.is_primary_target() {
                    construct_module_test_plan(env, package_filter, module)
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn construct_module_test_plan(
    env: &GlobalEnv,
    _package_filter: Option<Symbol>,
    module: ModuleEnv,
) -> Option<ModuleTestPlan> {
    // TODO (#12885): what is a package?  Do we need this code?
    // if package_filter.is_some() && module.package_name != package_filter {
    // return None;
    // }

    let current_module = module.get_name();
    let tests: BTreeMap<_, _> = module
        .get_functions()
        .filter_map(|func| {
            let func_name = func.get_name_str();
            build_test_info(env, current_module, func)
                .map(|test_case| (func_name.clone(), test_case))
        })
        .collect();

    let module_id = module.get_identifier();
    if tests.is_empty() {
        None
    } else {
        let module_name = module.get_name();
        let addr = module_name.addr();
        let name_sym = module_name.name();
        let name_str = env.symbol_pool().string(name_sym).to_string();
        if let Some(module_identifier) = module_id {
            let name_id = Identifier::new(name_str.clone()).expect("name is valid for identifier");
            assert!(name_id == module_identifier);
        }
        let optional_num_addr: Option<move_core_types::account_address::AccountAddress> = match addr
        {
            Address::Numerical(num_addr) => Some(*num_addr),
            Address::Symbolic(sym) => env.resolve_address_alias(*sym),
        };
        optional_num_addr.map(|addr_bytes| {
            ModuleTestPlan::new(
                &NumericalAddress::new(*addr_bytes, NumberFormat::Hex),
                &name_str,
                tests,
            )
        })
    }
}

fn build_test_info(
    env: &GlobalEnv,
    current_module: &ModuleName,
    function: FunctionEnv,
) -> Option<TestCase> {
    let fn_name_str = function.get_name_str();
    let fn_id_loc = function.get_id_loc();

    let attrs = function.get_attributes();
    let expected_failure_name = env.symbol_pool().make(TestingAttribute::EXPECTED_FAILURE);
    let test_name = env.symbol_pool().make(TestingAttribute::TEST);
    let test_only_name = env.symbol_pool().make(TestingAttribute::TEST_ONLY);

    let test_attribute_opt = attrs.iter().find(|a| a.name() == test_name);
    let abort_attribute_opt = attrs.iter().find(|a| a.name() == expected_failure_name);

    let test_attribute = match test_attribute_opt {
        None => {
            // expected failures cannot be annotated on non-#[test] functions
            if let Some(abort_attribute) = abort_attribute_opt {
                let fn_msg = "Only functions defined as a test with #[test] can also have an \
                              #[expected_failure] attribute";
                let abort_msg = "Attributed as #[expected_failure] here";
                let abort_id = abort_attribute.node_id();
                let abort_loc = env.get_node_loc(abort_id);
                env.error_with_labels(&fn_id_loc, fn_msg, vec![(abort_loc, abort_msg.to_string())]);
            }
            return None;
        },
        Some(test_attribute) => test_attribute,
    };

    let test_attribute_id = test_attribute.node_id();
    let test_attribute_loc = env.get_node_loc(test_attribute_id);

    let test_only_attribute_opt = attrs.iter().find(|a| a.name() == test_only_name);

    // A #[test] function cannot also be annotated #[test_only]
    if let Some(test_only_attribute) = test_only_attribute_opt {
        let msg = "Function annotated as both #[test(...)] and #[test_only]. You need to declare \
                   it as either one or the other";
        let test_only_id = test_only_attribute.node_id();
        let test_only_loc = env.get_node_loc(test_only_id);
        env.error_with_labels(&fn_id_loc, "invalid usage of known attribute", vec![
            (test_only_loc, msg.to_string()),
            (
                test_attribute_loc.clone(),
                "Previously annotated here".to_string(),
            ),
        ]);
    }

    let test_annotation_params = parse_test_attribute(env, test_attribute, 0);

    let mut arguments = Vec::new();
    for param in function.get_parameters_ref() {
        let Parameter(var, ty, var_loc) = &param;

        match test_annotation_params.get(var) {
            Some(MoveValue::Address(addr)) => arguments.push(match ty {
                Type::Primitive(PrimitiveType::Signer) => MoveValue::Signer(*addr),
                Type::Reference(_, inner) if **inner == Type::Primitive(PrimitiveType::Signer) => {
                    MoveValue::Signer(*addr)
                },
                _ => MoveValue::Address(*addr),
            }),
            Some(value) => arguments.push(value.clone()),
            None => {
                let missing_param_msg = "Missing test parameter assignment in test. Expected a \
                                         parameter to be assigned in this attribute";
                let invalid_test = "unable to generate test";
                env.error_with_labels(&fn_id_loc, invalid_test, vec![
                    (test_attribute_loc.clone(), missing_param_msg.to_string()),
                    (
                        var_loc.clone(),
                        "Corresponding to this parameter".to_string(),
                    ),
                ]);
            },
        }
    }

    let expected_failure = match abort_attribute_opt {
        None => None,
        Some(abort_attribute) => parse_failure_attribute(env, current_module, abort_attribute),
    };

    Some(TestCase {
        test_name: fn_name_str.to_string(),
        arguments,
        expected_failure,
    })
}

//***************************************************************************
// Attribute parsers
//***************************************************************************

fn parse_test_attribute(
    env: &GlobalEnv,
    test_attribute: &Attribute,
    depth: usize,
) -> BTreeMap<Symbol, MoveValue> {
    match test_attribute {
        Attribute::Apply(id, _, _) if depth > 0 => {
            let aloc = env.get_node_loc(*id);
            env.error(&aloc, "Unexpected nested attribute in test declaration");
            BTreeMap::new()
        },
        Attribute::Apply(_id, sym, vec) => {
            assert!(
                *TestingAttribute::TEST == env.symbol_pool().string(*sym).to_string(),
                "ICE: We should only be parsing a raw test attribute"
            );
            vec.iter()
                .flat_map(|attr| parse_test_attribute(env, attr, depth + 1))
                .collect()
        },
        Attribute::Assign(id, sym, val) => {
            if depth != 1 {
                let aloc = env.get_node_loc(*id);
                env.error(&aloc, "Unexpected nested attribute in test declaration");
                return BTreeMap::new();
            }

            let value = match convert_attribute_value_to_move_value(env, val) {
                Some(move_value) => move_value,
                None => {
                    let aloc = env.get_node_loc(*id);
                    let assign_loc = env.get_node_loc(*id);
                    env.error_with_labels(&assign_loc, "Unsupported attribute value", vec![(
                        aloc,
                        "Assigned in this attribute".to_string(),
                    )]);
                    return BTreeMap::new();
                },
            };

            let mut args = BTreeMap::new();
            args.insert(*sym, value);
            args
        },
    }
}

fn parse_failure_attribute(
    env: &GlobalEnv,
    current_module: &ModuleName,
    expected_attr: &Attribute,
) -> Option<ExpectedFailure> {
    match expected_attr {
        Attribute::Assign(id, _sym, _val) => {
            let assign_loc = env.get_node_loc(*id);
            let invalid_assignment_msg = "Invalid expected failure code assignment";
            let expected_msg =
                "Expect an #[expected_failure(...)] attribute for error specification";
            env.error_with_labels(&assign_loc, invalid_assignment_msg, vec![(
                assign_loc.clone(),
                expected_msg.to_string(),
            )]);
            None
        },
        Attribute::Apply(id, sym, attrs) => {
            assert!(
                TestingAttribute::EXPECTED_FAILURE == env.symbol_pool().string(*sym).to_string(),
                "ICE: We should only be parsing a raw expected failure attribute"
            );
            if attrs.is_empty() {
                return Some(ExpectedFailure::Expected);
            };
            let mut attrs: BTreeMap<String, Attribute> = attrs
                .iter()
                .map(|attr| {
                    (
                        env.symbol_pool().string(attr.name()).to_string(),
                        attr.clone(),
                    )
                })
                .collect();
            let mut expected_failure_kind_vec = TestingAttribute::expected_failure_cases()
                .iter()
                .filter_map(|k| {
                    let k = k.to_string();
                    let attr = attrs.remove(&k)?;
                    Some((k, attr))
                })
                .collect::<Vec<_>>();
            if expected_failure_kind_vec.len() != 1 {
                let invalid_attr_msg = format!(
                    "Invalid #[expected_failure(...)] attribute, expected 1 failure kind but found {}. Expected one of: {}",
                    expected_failure_kind_vec.len(),
                    TestingAttribute::expected_failure_cases().to_vec().join(", ")
                );
                let aloc = env.get_node_loc(*id);
                env.error(&aloc, &invalid_attr_msg);
                return None;
            }
            let (expected_failure_kind, attr) = expected_failure_kind_vec.pop().unwrap();
            let location_opt = attrs.remove(TestingAttribute::ERROR_LOCATION);
            let attr_loc = env.get_node_loc(attr.node_id());
            let (status_code, sub_status_code, location) = match expected_failure_kind.as_str() {
                TestingAttribute::ABORT_CODE_NAME => {
                    let (_value_name_loc, attr_value) =
                        get_assigned_attribute(env, TestingAttribute::ABORT_CODE_NAME, attr)?;
                    let (value_loc, opt_const_module_id, u) =
                        convert_constant_value_u64_constant_or_value(
                            env,
                            current_module,
                            &attr_value,
                        )?;
                    let location = if let Some(location_attr) = location_opt {
                        convert_location(env, location_attr)?
                    } else if let Some(location) = opt_const_module_id {
                        location
                    } else {
                        let tip = format!(
                            "Replace value with constant from expected module or add `{}=...` \
                            attribute.",
                            TestingAttribute::ERROR_LOCATION
                        );
                        env.diag_with_labels(
                            Severity::Warning,
                            &attr_loc,
                            "Test will pass on an abort from *any* module.",
                            vec![(value_loc, tip)],
                        );
                        return Some(ExpectedFailure::ExpectedWithCodeDEPRECATED(u));
                    };
                    (StatusCode::ABORTED, Some(u), location)
                },
                TestingAttribute::ARITHMETIC_ERROR_NAME => {
                    check_attribute_unassigned(env, TestingAttribute::ARITHMETIC_ERROR_NAME, attr)?;
                    let location_attr = check_location(
                        env,
                        attr_loc,
                        TestingAttribute::ARITHMETIC_ERROR_NAME,
                        location_opt,
                    )?;
                    let location = convert_location(env, location_attr)?;
                    (StatusCode::ARITHMETIC_ERROR, None, location)
                },
                TestingAttribute::OUT_OF_GAS_NAME => {
                    check_attribute_unassigned(env, TestingAttribute::OUT_OF_GAS_NAME, attr)?;
                    let location_attr = check_location(
                        env,
                        attr_loc,
                        TestingAttribute::OUT_OF_GAS_NAME,
                        location_opt,
                    )?;
                    let location = convert_location(env, location_attr)?;
                    (StatusCode::OUT_OF_GAS, None, location)
                },
                TestingAttribute::VECTOR_ERROR_NAME => {
                    check_attribute_unassigned(env, TestingAttribute::VECTOR_ERROR_NAME, attr)?;
                    let minor_attr_opt = attrs.remove(TestingAttribute::MINOR_STATUS_NAME);
                    let minor_status = if let Some(minor_attr) = minor_attr_opt {
                        let (_minor_value_loc, minor_value) = get_assigned_attribute(
                            env,
                            TestingAttribute::MINOR_STATUS_NAME,
                            minor_attr,
                        )?;
                        let (_, _, minor_status) = convert_constant_value_u64_constant_or_value(
                            env,
                            current_module,
                            &minor_value,
                        )?;
                        Some(minor_status)
                    } else {
                        None
                    };
                    let location_attr = check_location(
                        env,
                        attr_loc,
                        TestingAttribute::VECTOR_ERROR_NAME,
                        location_opt,
                    )?;
                    let location = convert_location(env, location_attr)?;
                    (StatusCode::VECTOR_OPERATION_ERROR, minor_status, location)
                },
                TestingAttribute::MAJOR_STATUS_NAME => {
                    let (value_name_loc, attr_value) =
                        get_assigned_attribute(env, TestingAttribute::MAJOR_STATUS_NAME, attr)?;
                    let (major_value_loc, _, major_status_u64) =
                        convert_constant_value_u64_constant_or_value(
                            env,
                            current_module,
                            &attr_value,
                        )?;
                    let major_status = if let Ok(c) = StatusCode::try_from(major_status_u64) {
                        c
                    } else {
                        let bad_value = format!(
                            "Invalid value for `{}`",
                            TestingAttribute::MAJOR_STATUS_NAME,
                        );
                        let no_code =
                            format!("No status code associated with value `{major_status_u64}`");
                        env.error_with_labels(&value_name_loc, &bad_value, vec![(
                            major_value_loc,
                            no_code,
                        )]);
                        return None;
                    };
                    let minor_attr_opt = attrs.remove(TestingAttribute::MINOR_STATUS_NAME);
                    let minor_status = if let Some(minor_attr) = minor_attr_opt {
                        let (_minor_value_loc, minor_value) = get_assigned_attribute(
                            env,
                            TestingAttribute::MINOR_STATUS_NAME,
                            minor_attr,
                        )?;
                        let (_, _, minor_status) = convert_constant_value_u64_constant_or_value(
                            env,
                            current_module,
                            &minor_value,
                        )?;
                        Some(minor_status)
                    } else {
                        None
                    };
                    let location_attr = check_location(
                        env,
                        attr_loc,
                        TestingAttribute::MAJOR_STATUS_NAME,
                        location_opt,
                    )?;
                    let location = convert_location(env, location_attr)?;
                    (major_status, minor_status, location)
                },
                _ => unreachable!(),
            };
            // warn for any remaining attrs
            for (_, attr) in attrs {
                let loc = env.get_node_loc(attr.node_id());
                let msg = format!(
                    "Unused attribute for {}",
                    TestingAttribute::ExpectedFailure.name()
                );
                env.diag(Severity::Warning, &loc, &msg);
            }
            Some(ExpectedFailure::ExpectedWithError(ExpectedMoveError(
                status_code,
                sub_status_code,
                move_binary_format::errors::Location::Module(location),
                None,
            )))
        },
    }
}

fn check_attribute_unassigned(env: &GlobalEnv, kind: &str, attr: Attribute) -> Option<()> {
    match attr {
        Attribute::Apply(id, sym, vec) => {
            assert!(env.symbol_pool().string(sym).to_string() == kind);
            if !vec.is_empty() {
                let msg = format!(
                    "Expected no parameters for for expected failure attribute `{}`",
                    kind
                );
                let attr_loc = env.get_node_loc(id);
                env.error(&attr_loc, &msg);
                None
            } else {
                Some(())
            }
        },
        Attribute::Assign(id, sym, _) => {
            assert!(env.symbol_pool().string(sym).to_string() == kind);
            let msg = format!(
                "Expected no assigned value, e.g. `{}`, for expected failure attribute",
                kind
            );
            let attr_loc = env.get_node_loc(id);
            env.error(&attr_loc, &msg);
            None
        },
    }
}

fn get_assigned_attribute(
    env: &GlobalEnv,
    kind: &str,
    attr: Attribute,
) -> Option<(Loc, AttributeValue)> {
    match attr {
        Attribute::Assign(id, sym, value) => {
            assert!(env.symbol_pool().string(sym).to_string() == kind);
            let loc = env.get_node_loc(id);
            Some((loc, value))
        },
        Attribute::Apply(id, _sym, _vec) => {
            let loc = env.get_node_loc(id);
            let msg = format!(
                "Expected assigned value, e.g. `{}=...`, for expected failure attribute",
                kind
            );
            env.error(&loc, &msg);
            None
        },
    }
}

fn convert_location(env: &GlobalEnv, attr: Attribute) -> Option<ModuleId> {
    let (loc, value) = get_assigned_attribute(env, TestingAttribute::ERROR_LOCATION, attr)?;
    match value {
        AttributeValue::Name(id, opt_module_name, sym) => {
            let vloc = env.get_node_loc(id);
            let module_id_opt = convert_module_id(env, vloc.clone(), opt_module_name);
            if !sym.display(env.symbol_pool()).to_string().is_empty() || module_id_opt.is_none() {
                env.error_with_labels(&loc, "invalid attribute value", vec![(
                    vloc,
                    "Expected a module identifier, e.g. 'std::vector'".to_string(),
                )]);
            }
            module_id_opt
        },
        AttributeValue::Value(id, _val) => {
            let vloc = env.get_node_loc(id);
            env.error_with_labels(&loc, "invalid attribute value", vec![(
                vloc,
                "Expected a module identifier, e.g. 'std::vector'".to_string(),
            )]);
            None
        },
    }
}

fn convert_constant_value_u64_constant_or_value(
    env: &GlobalEnv,
    current_module: &ModuleName,
    value: &AttributeValue,
) -> Option<(Loc, Option<ModuleId>, u64)> {
    let (vloc, opt_module_name, member) = match value {
        AttributeValue::Value(id, val) => {
            let loc = env.get_node_loc(*id);
            if let Some((vloc, u)) = convert_model_ast_value_u64(env, loc, val) {
                return Some((vloc, None, u));
            } else {
                return None;
            }
        },
        AttributeValue::Name(id, opt_module_name, sym) => {
            let vloc = env.get_node_loc(*id);
            (vloc, opt_module_name, sym)
        },
    };
    let module_env: ModuleEnv = if let Some(module_name) = opt_module_name {
        if let Some(module_env) = env.find_module(module_name) {
            module_env
        } else {
            env.error(
                &vloc,
                &format!(
                    "Unbound module `{}` in constant",
                    module_name.display_full(env)
                ),
            );
            return None;
        }
    } else {
        env.find_module(current_module)
            .expect("current module exists")
    };
    let module_name = opt_module_name.as_ref().unwrap_or(current_module).clone();
    let named_constant_env =
        if let Some(named_constant_env) = module_env.find_named_constant(*member) {
            named_constant_env
        } else {
            env.error(
                &vloc,
                &format!(
                    "Unbound constant `{}` in module `{}`",
                    member.display(env.symbol_pool()),
                    module_name.display_full(env)
                ),
            );
            return None;
        };
    let ty = named_constant_env.get_type();
    let value = named_constant_env.get_value();
    let mod_id: Option<ModuleId> = convert_module_id(env, vloc.clone(), opt_module_name.clone());
    let (severity, message) = match value {
        Value::Number(u) => match ty {
            Type::Primitive(PrimitiveType::U64) => {
                if u <= BigInt::from(std::u64::MAX) {
                    return Some((vloc, mod_id, u.to_u64().unwrap()));
                } else {
                    (
                        Severity::Bug,
                        format!(
                            "Constant `{}::{}` value is out of range for u64",
                            module_name.display_full(env),
                            named_constant_env.get_name().display(env.symbol_pool())
                        ),
                    )
                }
            },
            Type::Primitive(PrimitiveType::Num) => {
                if u <= BigInt::from(std::u64::MAX) {
                    return Some((vloc, mod_id, u.to_u64().unwrap()));
                } else {
                    (
                        Severity::Error,
                        format!(
                            "Constant `{}::{}` value is out of range for u64",
                            module_name.display_full(env),
                            named_constant_env.get_name().display(env.symbol_pool())
                        ),
                    )
                }
            },
            _ => (
                Severity::Error,
                format!(
                    "Constant `{}::{}` has a non-u64 value.  Only `u64` values are permitted",
                    module_name.display_full(env),
                    named_constant_env.get_name().display(env.symbol_pool())
                ),
            ),
        },
        _ => (
            Severity::Error,
            format!(
                "Constant `{}::{}` has a non-numeric value.  Only `u64` values are permitted",
                module_name.display_full(env),
                named_constant_env.get_name().display(env.symbol_pool())
            ),
        ),
    };
    env.diag(severity, &vloc, &message);
    None
}

fn convert_module_id(env: &GlobalEnv, _vloc: Loc, module: Option<ModuleName>) -> Option<ModuleId> {
    if let Some(module_name) = module {
        let addr = module_name.addr();
        let sym = module_name.name();
        let sym_rc_str = env.symbol_pool().string(sym).to_string();
        let sym_core_id = Identifier::new(sym_rc_str).unwrap();
        match addr {
            Address::Numerical(addr) => Some(*addr),
            Address::Symbolic(sym) => env.resolve_address_alias(*sym),
        }
        .map(|account_address| ModuleId::new(account_address, sym_core_id))
    } else {
        None
    }
}

fn convert_model_ast_value_u64(env: &GlobalEnv, loc: Loc, value: &Value) -> Option<(Loc, u64)> {
    match value {
        Value::Number(u) => {
            if u <= &BigInt::from(std::u64::MAX) {
                Some((loc, u.to_u64().unwrap()))
            } else {
                env.error(
                    &loc,
                    "Invalid attribute value: only u64 literal values permitted",
                );
                None
            }
        },
        _ => {
            env.error(
                &loc,
                "Invalid attribute value: only u64 literal values permitted",
            );
            None
        },
    }
}

fn convert_attribute_value_to_move_value(
    env: &GlobalEnv,
    value: &AttributeValue,
) -> Option<MoveValue> {
    // Only addresses are allowed
    match value {
        AttributeValue::Value(_id, Value::Address(addr)) => match addr {
            Address::Numerical(num) => Some(*num),
            Address::Symbolic(sym) => env.resolve_address_alias(*sym),
        }
        .map(MoveValue::Address),
        _ => None,
    }
}

fn check_location<T>(env: &GlobalEnv, loc: Loc, attr: &str, location: Option<T>) -> Option<T> {
    if location.is_none() {
        let msg = format!(
            "Expected `{}` following `{}`",
            TestingAttribute::ERROR_LOCATION,
            attr
        );
        env.error(&loc, &msg)
    }
    location
}
