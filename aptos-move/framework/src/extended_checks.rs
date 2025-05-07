// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm::module_metadata::{
    KnownAttribute, RandomnessAnnotation, ResourceGroupScope, RuntimeModuleMetadataV1,
};
use legacy_move_compiler::shared::known_attributes;
use move_binary_format::file_format::Visibility;
use move_cli::base::test_validation;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    errmap::{ErrorDescription, ErrorMapping},
    identifier::Identifier,
    language_storage::ModuleId,
};
use move_model::{
    ast::{Attribute, AttributeValue, Value},
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, Parameter, QualifiedId,
        StructEnv, StructId,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    stackless_bytecode::{AttrId, Bytecode, Operation},
    stackless_bytecode_generator::StacklessBytecodeGenerator,
};
use num_traits::Signed;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

const ALLOW_UNSAFE_RANDOMNESS_ATTRIBUTE: &str = "lint::allow_unsafe_randomness";
const FMT_SKIP_ATTRIBUTE: &str = "fmt::skip";
const INIT_MODULE_FUN: &str = "init_module";
const LEGACY_ENTRY_FUN_ATTRIBUTE: &str = "legacy_entry_fun";
const ERROR_PREFIX: &str = "E";
const EVENT_STRUCT_ATTRIBUTE: &str = "event";
const MUTATION_SKIP_ATTRIBUTE: &str = "mutation::skip";
const RANDOMNESS_ATTRIBUTE: &str = "randomness";
const RANDOMNESS_MAX_GAS_CLAIM: &str = "max_gas";
const RESOURCE_GROUP: &str = "resource_group";
const RESOURCE_GROUP_MEMBER: &str = "resource_group_member";
const RESOURCE_GROUP_NAME: &str = "group";
const RESOURCE_GROUP_SCOPE: &str = "scope";
const VIEW_FUN_ATTRIBUTE: &str = "view";

const RANDOMNESS_MODULE_NAME: &str = "randomness";

// top-level attribute names, only.
pub fn get_all_attribute_names() -> &'static BTreeSet<String> {
    const ALL_ATTRIBUTE_NAMES: [&str; 9] = [
        ALLOW_UNSAFE_RANDOMNESS_ATTRIBUTE,
        FMT_SKIP_ATTRIBUTE,
        LEGACY_ENTRY_FUN_ATTRIBUTE,
        MUTATION_SKIP_ATTRIBUTE,
        RESOURCE_GROUP,
        RESOURCE_GROUP_MEMBER,
        VIEW_FUN_ATTRIBUTE,
        EVENT_STRUCT_ATTRIBUTE,
        RANDOMNESS_ATTRIBUTE,
    ];

    fn extended_attribute_names() -> BTreeSet<String> {
        ALL_ATTRIBUTE_NAMES
            .into_iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<String>>()
    }

    static KNOWN_ATTRIBUTES_SET: Lazy<BTreeSet<String>> = Lazy::new(|| {
        use known_attributes::AttributeKind;
        let mut attributes = extended_attribute_names();
        known_attributes::KnownAttribute::add_attribute_names(&mut attributes);
        attributes
    });
    &KNOWN_ATTRIBUTES_SET
}

/// Run the extended context checker on target modules in the environment and returns a map
/// from module to extended runtime metadata. Any errors during context checking are reported to
/// `env`. This is invoked after general build succeeds.
pub fn run_extended_checks(env: &GlobalEnv) -> BTreeMap<ModuleId, RuntimeModuleMetadataV1> {
    let mut checker = ExtendedChecker::new(env);
    checker.run();
    checker.output
}

/// Configures the move-cli unit test validation hook to run the extended checker.
pub fn configure_extended_checks_for_unit_test() {
    fn validate(env: &GlobalEnv) {
        run_extended_checks(env);
    }
    test_validation::set_validation_hook(Box::new(validate));
}

#[derive(Debug)]
struct ExtendedChecker<'a> {
    env: &'a GlobalEnv,
    /// Computed runtime metadata
    output: BTreeMap<ModuleId, RuntimeModuleMetadataV1>,
    /// The id of the module defining error categories
    error_category_module: ModuleId,
    /// A cache for functions which are known to call or not call randomness features
    randomness_caller_cache: BTreeMap<QualifiedId<FunId>, bool>,
}

impl<'a> ExtendedChecker<'a> {
    fn new(env: &'a GlobalEnv) -> Self {
        Self {
            env,
            output: BTreeMap::default(),
            error_category_module: ModuleId::new(
                AccountAddress::ONE,
                Identifier::new("error").unwrap(),
            ),
            randomness_caller_cache: BTreeMap::new(),
        }
    }

    fn run(&mut self) {
        for ref module in self.env.get_modules() {
            if module.is_primary_target() {
                self.check_and_record_resource_groups(module);
                self.check_and_record_resource_group_members(module);
                self.check_and_record_view_functions(module);
                self.check_entry_functions(module);
                self.check_and_record_unbiasabale_entry_functions(module);
                self.check_unsafe_randomness_usage(module);
                self.check_and_record_events(module);
                self.check_init_module(module);
                self.build_error_map(module)
            }
        }
    }
}

// ----------------------------------------------------------------------------------
// Module Initialization

impl<'a> ExtendedChecker<'a> {
    fn check_init_module(&self, module: &ModuleEnv) {
        // TODO: also enable init_module by attribute, perhaps deprecate by name
        let init_module_sym = self.env.symbol_pool().make(INIT_MODULE_FUN);
        if let Some(ref fun) = module.find_function(init_module_sym) {
            if fun.visibility() != Visibility::Private {
                self.env.error(
                    &fun.get_id_loc(),
                    &format!("`{}` function must be private", INIT_MODULE_FUN),
                )
            }

            if fun.get_parameter_count() != 1 {
                self.env.error(
                    &fun.get_id_loc(),
                    &format!(
                        "`{}` function can only take a single value of type `signer` \
                         or `&signer` as parameter",
                        INIT_MODULE_FUN
                    ),
                )
            } else {
                let Parameter(_, ty, _) = &fun.get_parameters()[0];
                let ok = match ty {
                    Type::Primitive(PrimitiveType::Signer) => true,
                    Type::Reference(_, ty) => {
                        matches!(ty.as_ref(), Type::Primitive(PrimitiveType::Signer))
                    },
                    _ => false,
                };
                if !ok {
                    self.env.error(
                        &fun.get_id_loc(),
                        &format!(
                            "`{}` function can only take an argument of type `signer` \
                             or `&signer` as a single parameter",
                            INIT_MODULE_FUN
                        ),
                    );
                }
            }

            if fun.get_return_count() > 0 {
                self.env.error(
                    &fun.get_id_loc(),
                    &format!("`{}` function cannot return values", INIT_MODULE_FUN),
                )
            }

            if fun.get_type_parameter_count() > 0 {
                self.env.error(
                    &fun.get_id_loc(),
                    &format!("`{}` function cannot have type parameters", INIT_MODULE_FUN),
                )
            }
        }
    }
}

// ----------------------------------------------------------------------------------
// Entry Functions

impl<'a> ExtendedChecker<'a> {
    fn check_entry_functions(&self, module: &ModuleEnv) {
        for ref fun in module.get_functions() {
            if !fun.is_entry() {
                continue;
            }

            if self.has_attribute(fun, LEGACY_ENTRY_FUN_ATTRIBUTE) {
                // Skip checking for legacy entries.
                continue;
            }

            self.check_transaction_args(&fun.get_parameters());
            self.check_signer_args(&fun.get_parameters());
            if fun.get_return_count() > 0 {
                self.env
                    .error(&fun.get_id_loc(), "entry function cannot return values")
            }
        }
    }

    fn check_transaction_args(&self, arg_tys: &[Parameter]) {
        for Parameter(_sym, ty, param_loc) in arg_tys {
            self.check_transaction_input_type(param_loc, ty)
        }
    }

    fn check_signer_args(&self, arg_tys: &[Parameter]) {
        // All signer args should precede non-signer args, for an entry function to be
        // used as an entry function.
        let mut seen_non_signer = false;
        for Parameter(_, ty, loc) in arg_tys {
            // We assume `&mut signer` are disallowed by checks elsewhere, so it is okay
            // for `skip_reference()` below to skip both kinds of reference.
            let ty_is_signer = ty.skip_reference().is_signer();
            if seen_non_signer && ty_is_signer {
                self.env.warning(
                    loc,
                    "to be used as an entry function, all signers should precede non-signers",
                );
            }
            if !ty_is_signer {
                seen_non_signer = true;
            }
        }
    }

    /// Note: this should be kept up in sync with `is_valid_txn_arg` in
    /// aptos-move/aptos-vm/src/verifier/transaction_arg_validation.rs
    fn check_transaction_input_type(&self, loc: &Loc, ty: &Type) {
        use Type::*;
        match ty {
            Primitive(_) | TypeParameter(_) => {
                // Any primitive type allowed, any parameter expected to instantiate with primitive
            },
            Reference(ReferenceKind::Immutable, bt)
                if matches!(bt.as_ref(), Primitive(PrimitiveType::Signer)) =>
            {
                // Immutable reference to signer allowed
            },
            Vector(ety) => {
                // Vectors are allowed if element type is allowed
                self.check_transaction_input_type(loc, ety)
            },
            Struct(mid, sid, _) if self.is_allowed_input_struct(mid.qualified(*sid)) => {
                // Specific struct types are allowed
            },
            _ => {
                // Everything else is disallowed.
                self.env.error(
                    loc,
                    &format!(
                        "type `{}` is not supported as a transaction parameter type",
                        ty.display(&self.env.get_type_display_ctx())
                    ),
                );
            },
        }
    }

    fn is_allowed_input_struct(&self, qid: QualifiedId<StructId>) -> bool {
        let name = self.env.get_struct(qid).get_full_name_with_address();
        // TODO(gerben) find a nice way to keep this in sync with allowed_structs in aptos-vm
        matches!(
            name.as_str(),
            "0x1::string::String"
                | "0x1::object::Object"
                | "0x1::option::Option"
                | "0x1::fixed_point32::FixedPoint32"
                | "0x1::fixed_point64::FixedPoint64"
        )
    }
}

// ----------------------------------------------------------------------------------
// Resource Group Functions

impl<'a> ExtendedChecker<'a> {
    // A entry in a resource group should contain the resource group attribute and a parameter that
    // points to a resource group container.
    fn check_and_record_resource_group_members(&mut self, module: &ModuleEnv) {
        let module_id = self.get_runtime_module_id(module);

        for ref struct_ in module.get_structs() {
            let resource_group = struct_.get_attributes().iter().find(|attr| {
                if let Attribute::Apply(_, name, _) = attr {
                    self.env.symbol_pool().string(*name).as_str() == RESOURCE_GROUP_MEMBER
                } else {
                    false
                }
            });

            if let Some(Attribute::Apply(_, _, attributes)) = resource_group {
                if !struct_.get_abilities().has_ability(Ability::Key) {
                    self.env
                        .error(&struct_.get_loc(), "resource_group should have key ability");
                    continue;
                }

                if attributes.len() != 1 {
                    self.env.error(
                        &struct_.get_loc(),
                        "resource_group_member must contain 1 parameters",
                    );
                }

                let value = if let Attribute::Assign(_, name, value) = &attributes[0] {
                    if self.env.symbol_pool().string(*name).as_str() != RESOURCE_GROUP_NAME {
                        self.env.error(
                            &struct_.get_loc(),
                            "resource_group_member lacks 'group' parameter",
                        );
                    }
                    value
                } else {
                    self.env.error(
                        &struct_.get_loc(),
                        "resource_group_member lacks 'group' parameter",
                    );
                    continue;
                };

                let (module_name, container_name) =
                    if let AttributeValue::Name(_, Some(module), name) = value {
                        (module, name)
                    } else {
                        self.env.error(
                            &struct_.get_loc(),
                            "resource_group_member lacks 'group' parameter",
                        );
                        continue;
                    };

                let module = if let Some(module) = self.env.find_module(module_name) {
                    module
                } else {
                    self.env
                        .error(&struct_.get_loc(), "unable to find resource_group module");
                    continue;
                };

                let container = if let Some(container) = module.find_struct(*container_name) {
                    container
                } else {
                    self.env
                        .error(&struct_.get_loc(), "unable to find resource_group struct");
                    continue;
                };

                if let Some(scope) = self.get_resource_group(&container) {
                    if !scope.are_equal_envs(struct_, &container) {
                        self.env
                            .error(&struct_.get_loc(), "resource_group scope mismatch");
                        continue;
                    }

                    self.output
                        .entry(module_id.clone())
                        .or_default()
                        .struct_attributes
                        .entry(
                            self.env
                                .symbol_pool()
                                .string(struct_.get_name())
                                .to_string(),
                        )
                        .or_default()
                        .push(KnownAttribute::resource_group_member(
                            container.get_full_name_with_address(),
                        ));
                } else {
                    self.env.error(
                        &struct_.get_loc(),
                        "container is not a resource_group_container",
                    );
                }
            }
        }
    }

    // A resource group container should be a unit struct with no attributes or type parameters
    fn check_and_record_resource_groups(&mut self, module: &ModuleEnv) {
        let module_id = self.get_runtime_module_id(module);

        for ref struct_ in module.get_structs() {
            if let Some(scope) = self.get_resource_group(struct_) {
                self.output
                    .entry(module_id.clone())
                    .or_default()
                    .struct_attributes
                    .entry(
                        self.env
                            .symbol_pool()
                            .string(struct_.get_name())
                            .to_string(),
                    )
                    .or_default()
                    .push(KnownAttribute::resource_group(scope));
            }
        }
    }

    fn get_resource_group(&mut self, struct_: &StructEnv) -> Option<ResourceGroupScope> {
        let container = struct_.get_attributes().iter().find(|attr| {
            if let Attribute::Apply(_, name, _) = attr {
                self.name_string(*name).as_ref() == RESOURCE_GROUP
            } else {
                false
            }
        })?;

        // Every struct contains the "dummy_field".
        if struct_.get_field_count() == 1 {
            let name = self.name_string(struct_.get_fields().next().unwrap().get_name());
            // TODO: check that the type is bool
            if *name != "dummy_field" {
                self.env
                    .error(&struct_.get_loc(), "resource_group should not have fields");
                return None;
            };
        }

        if struct_.get_field_count() > 1 {
            self.env
                .error(&struct_.get_loc(), "resource_group should not have fields");
            return None;
        }

        if struct_.get_abilities() != AbilitySet::EMPTY {
            self.env.error(
                &struct_.get_loc(),
                "resource_group should not have abilities",
            );
            return None;
        }

        if !struct_.get_type_parameters().is_empty() {
            self.env.error(
                &struct_.get_loc(),
                "resource_group should not have type parameters",
            );
            return None;
        }

        if let Attribute::Apply(_, _, attributes) = container {
            if attributes.len() != 1 {
                self.env.error(
                    &struct_.get_loc(),
                    "resource_group_container must contain 1 parameters",
                );
                return None;
            }

            let value = if let Attribute::Assign(_, name, value) = &attributes[0] {
                if self.name_string(*name).as_str() != RESOURCE_GROUP_SCOPE {
                    self.env.error(
                        &struct_.get_loc(),
                        "resource_group_container lacks 'scope' parameter",
                    );
                }
                value
            } else {
                self.env.error(
                    &struct_.get_loc(),
                    "resource_group_container lacks 'scope' parameter",
                );
                return None;
            };

            if let AttributeValue::Name(_, _, name) = value {
                let result = str::parse(&self.name_string(*name));
                if result.is_err() {
                    self.env.error(
                        &struct_.get_loc(),
                        &format!(
                            "resource_group_container invalid 'scope': {}",
                            self.name_string(*name)
                        ),
                    );
                }
                result.ok()
            } else {
                self.env.error(
                    &struct_.get_loc(),
                    "resource_group lacks 'container' parameter",
                );
                None
            }
        } else {
            None
        }
    }
}

// ----------------------------------------------------------------------------------
// Unbiasable entry functions

impl<'a> ExtendedChecker<'a> {
    fn check_and_record_unbiasabale_entry_functions(&mut self, module: &ModuleEnv) {
        for ref fun in module.get_functions() {
            let maybe_randomness_annotation = match self.get_randomness_max_gas_declaration(fun) {
                Ok(x) => x,
                Err(msg) => {
                    self.env.error(&fun.get_id_loc(), msg.as_str());
                    continue;
                },
            };

            let Some(randomness_annotation) = maybe_randomness_annotation else {
                continue;
            };

            if !fun.is_entry() || fun.visibility().is_public() {
                self.env.error(
                    &fun.get_id_loc(),
                    "only private or public(friend) entry functions can have #[randomness] attribute",
                )
            }

            // Record functions which use randomness.
            let module_id = self.get_runtime_module_id(module);
            self.output
                .entry(module_id)
                .or_default()
                .fun_attributes
                .entry(fun.get_simple_name_string().to_string())
                .or_default()
                .push(KnownAttribute::randomness(randomness_annotation.max_gas));
        }
    }

    fn get_randomness_max_gas_declaration(
        &self,
        fun: &FunctionEnv,
    ) -> Result<Option<RandomnessAnnotation>, String> {
        for attr in fun.get_attributes() {
            let Attribute::Apply(_, name, sub_attrs) = attr else {
                continue;
            };
            if self.env.symbol_pool().string(*name).as_str() != RANDOMNESS_ATTRIBUTE {
                continue;
            }
            if sub_attrs.len() >= 2 {
                return Err(
                    "Randomness attribute should only have one `max_gas` property.".to_string(),
                );
            }
            if let Some(sub_attr) = sub_attrs.first() {
                let Attribute::Assign(_, key_symbol, attr_val) = sub_attr else {
                    return Err(
                        "Randomness attribute should only have one `max_gas` property.".to_string(),
                    );
                };
                let key = self.env.symbol_pool().string(*key_symbol);
                if key.as_str() != RANDOMNESS_MAX_GAS_CLAIM {
                    return Err(format!(
                        "Unknown key for randomness attribute: {}",
                        key.as_str()
                    ));
                }

                let msg =
                    "Property `max_gas` should be a positive integer less than 2^64".to_string();
                let AttributeValue::Value(_, Value::Number(val)) = attr_val else {
                    return Err(msg);
                };

                // Ensure the value is non-negative.
                if val.abs() != *val {
                    return Err(msg);
                }

                let (_, digits) = val.to_u64_digits();

                if digits.len() >= 2 {
                    return Err(msg);
                }

                let max_gas_declaration_seen = Some(digits.first().copied().unwrap_or(0)); // If no digits, assuming the value is 0.

                return Ok(Some(RandomnessAnnotation::new(max_gas_declaration_seen)));
            } else {
                return Ok(Some(RandomnessAnnotation::default()));
            }
        }
        Ok(None)
    }
}

// ----------------------------------------------------------------------------------
// Checks for unsafe usage of randomness

impl<'a> ExtendedChecker<'a> {
    /// Checks unsafe usage of the randomness feature for the given module.
    ///
    /// 1. Checks that no public function in the module calls randomness features. An
    ///    attribute can be used to override this check.
    /// 2. Check that every private entry function which uses randomness
    ///    features has the #[randomness] attribute
    fn check_unsafe_randomness_usage(&mut self, module: &ModuleEnv) {
        for ref fun in module.get_functions() {
            let fun_id = fun.module_env.get_id().qualified(fun.get_id());
            // Check condition (2)
            if !fun.visibility().is_public() && fun.is_entry() {
                if !self.has_attribute(fun, RANDOMNESS_ATTRIBUTE) && self.calls_randomness(fun_id) {
                    self.env.error(
                        &fun.get_id_loc(),
                        "entry function calling randomness features must \
                    use the `#[randomness]` attribute.",
                    )
                }
                continue;
            }
            // Check condition (1) only if function is public and if the check is not disabled.
            // Also, only check functions which are not declared in the framework.
            if self.is_framework_function(fun_id)
                || !fun.visibility().is_public()
                || self.has_attribute(fun, ALLOW_UNSAFE_RANDOMNESS_ATTRIBUTE)
            {
                continue;
            }
            if self.calls_randomness(fun_id) {
                self.env.error(
                    &fun.get_id_loc(),
                    &format!(
                        "public function exposes functionality of the `randomness` module \
                    which can be unsafe. Consult the randomness documentation for an explanation \
                    of this error. To skip this check, add \
                    attribute `#[{}]`.",
                        ALLOW_UNSAFE_RANDOMNESS_ATTRIBUTE
                    ),
                )
            }
        }
    }

    /// Checks whether given function calls randomness functionality. This walks the call graph
    /// recursively and stops as soon as a random call is found.
    fn calls_randomness(&mut self, fun: QualifiedId<FunId>) -> bool {
        if let Some(is_caller) = self.randomness_caller_cache.get(&fun) {
            return *is_caller;
        }
        // For building a fixpoint on cycles, set the value initially to false
        self.randomness_caller_cache.insert(fun, false);
        let mut is_caller = false;
        for callee in self
            .env
            .get_function(fun)
            .get_called_functions()
            .expect("callees defined")
        {
            if self.is_randomness_fun(*callee) || self.calls_randomness(*callee) {
                is_caller = true;
                break;
            }
        }
        if is_caller {
            // If we found a randomness call, update the cache
            self.randomness_caller_cache.insert(fun, true);
        }
        is_caller
    }

    /// Check whether function belongs to the randomness feature. This is done by
    /// checking the well-known module name.
    fn is_randomness_fun(&self, fun: QualifiedId<FunId>) -> bool {
        let module_name = self.env.get_function(fun).module_env.get_name().clone();
        module_name.addr().expect_numerical() == AccountAddress::ONE
            && *self.env.symbol_pool().string(module_name.name()) == RANDOMNESS_MODULE_NAME
    }
}

// ----------------------------------------------------------------------------------
// View Functions

impl<'a> ExtendedChecker<'a> {
    fn check_and_record_view_functions(&mut self, module: &ModuleEnv) {
        for ref fun in module.get_functions() {
            if !self.has_attribute(fun, VIEW_FUN_ATTRIBUTE) {
                continue;
            }
            self.check_transaction_args(&fun.get_parameters());
            if fun.get_return_count() == 0 {
                self.env
                    .error(&fun.get_id_loc(), "`#[view]` function must return values")
            }

            fun.get_parameters()
                .iter()
                .for_each(
                    |Parameter(_sym, parameter_type, param_loc)| match parameter_type {
                        Type::Primitive(inner) => {
                            if inner == &PrimitiveType::Signer {
                                self.env.error(
                                    param_loc,
                                    "`#[view]` function cannot use a `signer` parameter",
                                )
                            }
                        },
                        Type::Reference(mutability, inner) => {
                            if let Type::Primitive(inner) = inner.as_ref() {
                                if inner == &PrimitiveType::Signer
                                // Avoid a redundant error message for `&mut signer`, which is
                                // always disallowed for transaction entries, not just for
                                // `#[view]`.
                                    && mutability == &ReferenceKind::Immutable
                                {
                                    self.env.error(
                                        param_loc,
                                        "`#[view]` function cannot use the `&signer` parameter",
                                    )
                                }
                            }
                        },
                        _ => (),
                    },
                );

            // Remember the runtime info that this is a view function
            let module_id = self.get_runtime_module_id(module);
            self.output
                .entry(module_id)
                .or_default()
                .fun_attributes
                .entry(fun.get_simple_name_string().to_string())
                .or_default()
                .push(KnownAttribute::view_function());
        }
    }
}

// ----------------------------------------------------------------------------------
// Events

impl<'a> ExtendedChecker<'a> {
    fn check_and_record_events(&mut self, module: &ModuleEnv) {
        for ref struct_ in module.get_structs() {
            if self.has_attribute_iter(struct_.get_attributes().iter(), EVENT_STRUCT_ATTRIBUTE) {
                let module_id = self.get_runtime_module_id(module);
                // Remember the runtime info that this is a event struct.
                self.output
                    .entry(module_id)
                    .or_default()
                    .struct_attributes
                    .entry(
                        self.env
                            .symbol_pool()
                            .string(struct_.get_name())
                            .to_string(),
                    )
                    .or_default()
                    .push(KnownAttribute::event());
            }
        }
        for fun in module.get_functions() {
            if fun.is_inline() || fun.is_native() {
                continue;
            }
            // Holder for stackless function data
            let data = self.get_stackless_data(&fun);
            // Handle to work with stackless functions -- function targets.
            let target = FunctionTarget::new(&fun, &data);
            // Now check for event emit calls.
            for bc in target.get_bytecode() {
                if let Bytecode::Call(attr_id, _, Operation::Function(mid, fid, type_inst), _, _) =
                    bc
                {
                    self.check_emit_event_call(
                        &module.get_id(),
                        &target,
                        *attr_id,
                        mid.qualified(*fid),
                        type_inst,
                    );
                }
            }
        }
    }

    fn check_emit_event_call(
        &mut self,
        module_id: &move_model::model::ModuleId,
        target: &FunctionTarget,
        attr_id: AttrId,
        callee: QualifiedId<FunId>,
        type_inst: &[Type],
    ) {
        if !self.is_function(callee, "0x1::event::emit") {
            return;
        }
        // We are looking at `0x1::event::emit<T>` and extracting the `T`
        let event_type = &type_inst[0];
        // Now check whether this type has the event attribute
        let type_ok = match event_type {
            Type::Struct(mid, sid, _) => {
                let struct_ = self.env.get_struct(mid.qualified(*sid));
                // The struct must be defined in the current module.
                module_id == mid
                    && self
                        .has_attribute_iter(struct_.get_attributes().iter(), EVENT_STRUCT_ATTRIBUTE)
            },
            _ => false,
        };
        if !type_ok {
            let loc = target.get_bytecode_loc(attr_id);
            self.env.error(&loc,
                           &format!("`0x1::event::emit` called with type `{}` which is not a struct type defined in the same module with `#[event]` attribute",
                                    event_type.display(&self.env.get_type_display_ctx())));
        }
    }
}

// ----------------------------------------------------------------------------------
// Error Map

impl<'a> ExtendedChecker<'a> {
    fn build_error_map(&mut self, module: &ModuleEnv<'_>) {
        // Compute the error map, we are using the `ErrorMapping` type from Move which
        // is more general as we need as it works for multiple modules.
        let module_id = self.get_runtime_module_id(module);
        if module_id == self.error_category_module {
            return;
        }
        let mut error_map = ErrorMapping::default();
        for named_constant in module.get_named_constants() {
            let name = self.name_string(named_constant.get_name());
            if name.starts_with(ERROR_PREFIX) {
                if let Some(abort_code) = self.get_abort_code(&named_constant) {
                    // If an error is returned (because of duplicate entry) ignore it.
                    let _ = error_map.add_module_error(
                        &module_id.to_string(),
                        abort_code,
                        ErrorDescription {
                            code_name: name.trim().to_string(),
                            code_description: named_constant.get_doc().trim().to_string(),
                        },
                    );
                }
            }
        }
        // Inject it into runtime info
        self.output.entry(module_id).or_default().error_map = error_map
            .module_error_maps
            .remove(&module_id.to_string())
            .unwrap_or_default();
    }

    fn get_abort_code(&self, constant: &NamedConstantEnv<'_>) -> Option<u64> {
        match constant.get_value() {
            Value::Number(big_int) => u64::try_from(big_int).ok(),
            _ => None,
        }
    }
}

// ----------------------------------------------------------------------------------
// Helpers

impl<'a> ExtendedChecker<'a> {
    fn has_attribute(&self, fun: &FunctionEnv, attr_name: &str) -> bool {
        self.has_attribute_iter(fun.get_attributes().iter(), attr_name)
    }

    fn has_attribute_iter(
        &self,
        mut attrs: impl Iterator<Item = &'a Attribute>,
        attr_name: &str,
    ) -> bool {
        attrs.any(|attr| {
            if let Attribute::Apply(_, name, _) = attr {
                self.env.symbol_pool().string(*name).as_str() == attr_name
            } else {
                false
            }
        })
    }

    fn get_runtime_module_id(&self, module: &ModuleEnv<'_>) -> ModuleId {
        let name = module.get_name();
        let addr =
            AccountAddress::from_hex_literal(&format!("0x{:x}", name.addr().expect_numerical()))
                .unwrap();
        let name = Identifier::new(self.name_string(name.name()).to_string()).unwrap();
        ModuleId::new(addr, name)
    }

    fn name_string(&self, symbol: Symbol) -> Rc<String> {
        self.env.symbol_pool().string(symbol)
    }

    fn get_stackless_data(&self, fun: &FunctionEnv) -> FunctionData {
        StacklessBytecodeGenerator::new(fun).generate_function()
    }

    fn is_function(&self, id: QualifiedId<FunId>, full_name_str: &str) -> bool {
        let fun = &self.env.get_function(id);
        fun.get_full_name_with_address() == full_name_str
    }

    fn is_framework_function(&self, id: QualifiedId<FunId>) -> bool {
        self.env
            .get_function(id)
            .module_env
            .get_name()
            .addr()
            .expect_numerical()
            == AccountAddress::ONE
    }
}
