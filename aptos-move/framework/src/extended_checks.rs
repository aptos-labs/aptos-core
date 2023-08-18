// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{KnownAttribute, RuntimeModuleMetadataV1};
use move_binary_format::file_format::{Ability, AbilitySet, Visibility};
use move_compiler::shared::known_attributes;
use move_core_types::{
    account_address::AccountAddress,
    errmap::{ErrorDescription, ErrorMapping},
    identifier::Identifier,
    language_storage::ModuleId,
};
use move_model::{
    ast::{Attribute, AttributeValue, Value},
    model::{
        FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, Parameter, QualifiedId,
        StructEnv, StructId,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    str::FromStr,
};
use thiserror::Error;

const INIT_MODULE_FUN: &str = "init_module";
const LEGACY_ENTRY_FUN_ATTRIBUTE: &str = "legacy_entry_fun";
const ERROR_PREFIX: &str = "E";
const RESOURCE_GROUP: &str = "resource_group";
const RESOURCE_GROUP_MEMBER: &str = "resource_group_member";
const RESOURCE_GROUP_NAME: &str = "group";
const RESOURCE_GROUP_SCOPE: &str = "scope";
const VIEW_FUN_ATTRIBUTE: &str = "view";

// top-level attribute names, only.
pub fn get_all_attribute_names() -> &'static BTreeSet<String> {
    const ALL_ATTRIBUTE_NAMES: [&str; 4] = [
        LEGACY_ENTRY_FUN_ATTRIBUTE,
        RESOURCE_GROUP,
        RESOURCE_GROUP_MEMBER,
        VIEW_FUN_ATTRIBUTE,
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

#[derive(Debug)]
struct ExtendedChecker<'a> {
    env: &'a GlobalEnv,
    /// Computed runtime metadata
    output: BTreeMap<ModuleId, RuntimeModuleMetadataV1>,
    /// The id of the module defining error categories
    error_category_module: ModuleId,
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
        }
    }

    fn run(&mut self) {
        for ref module in self.env.get_modules() {
            if module.is_target() {
                self.check_and_record_resource_groups(module);
                self.check_and_record_resource_group_members(module);
                self.check_and_record_view_functions(module);
                self.check_entry_functions(module);
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
                self.env
                    .error(&fun.get_loc(), "`init_module` function must be private")
            }
            for Parameter(_, ty) in fun.get_parameters() {
                let ok = match ty {
                    Type::Primitive(PrimitiveType::Signer) => true,
                    Type::Reference(_, ty) => matches!(*ty, Type::Primitive(PrimitiveType::Signer)),
                    _ => false,
                };
                if !ok {
                    self.env.error(
                        &fun.get_loc(),
                        "`init_module` function can only take signers as parameters",
                    );
                }
            }
            if fun.get_return_count() > 0 {
                self.env.error(
                    &fun.get_loc(),
                    "`init_module` function cannot return values",
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
                // Skip checking for legacy entries
                continue;
            }
            self.check_transaction_args(&fun.get_loc(), &fun.get_parameter_types());
            if fun.get_return_count() > 0 {
                self.env
                    .error(&fun.get_loc(), "entry function cannot return values")
            }
        }
    }

    fn check_transaction_args(&self, loc: &Loc, arg_tys: &[Type]) {
        for ty in arg_tys {
            self.check_transaction_input_type(loc, ty)
        }
    }

    fn check_transaction_input_type(&self, loc: &Loc, ty: &Type) {
        use Type::*;
        match ty {
            Primitive(_) | TypeParameter(_) => {
                // Any primitive type allowed, any parameter expected to instantiate with primitive
            },
            Reference(ReferenceKind::Immutable, bt)
                if matches!(bt.as_ref(), Primitive(PrimitiveType::Signer)) =>
            {
                // Reference to signer allowed
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
                        "type `{}` is not supported as a parameter type",
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
// View Functions

impl<'a> ExtendedChecker<'a> {
    fn check_and_record_view_functions(&mut self, module: &ModuleEnv) {
        for ref fun in module.get_functions() {
            if !self.has_attribute(fun, VIEW_FUN_ATTRIBUTE) {
                continue;
            }
            self.check_transaction_args(&fun.get_loc(), &fun.get_parameter_types());
            if fun.get_return_count() == 0 {
                self.env
                    .error(&fun.get_loc(), "view function must return values")
            }
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
        fun.get_attributes().iter().any(|attr| {
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
}

// ----------------------------------------------------------------------------------
// Resource Group Container Scope

#[derive(Debug, Eq, PartialEq)]
pub enum ResourceGroupScope {
    Global,
    Address,
    Module,
}

impl ResourceGroupScope {
    pub fn is_less_strict(&self, other: &ResourceGroupScope) -> bool {
        match self {
            ResourceGroupScope::Global => other != self,
            ResourceGroupScope::Address => other == &ResourceGroupScope::Module,
            ResourceGroupScope::Module => false,
        }
    }

    pub fn are_equal_envs(&self, resource: &StructEnv, group: &StructEnv) -> bool {
        match self {
            ResourceGroupScope::Global => true,
            ResourceGroupScope::Address => {
                resource.module_env.get_name().addr() == group.module_env.get_name().addr()
            },
            ResourceGroupScope::Module => {
                resource.module_env.get_name() == group.module_env.get_name()
            },
        }
    }

    pub fn are_equal_module_ids(&self, resource: &ModuleId, group: &ModuleId) -> bool {
        match self {
            ResourceGroupScope::Global => true,
            ResourceGroupScope::Address => resource.address() == group.address(),
            ResourceGroupScope::Module => resource == group,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceGroupScope::Global => "global",
            ResourceGroupScope::Address => "address",
            ResourceGroupScope::Module => "module_",
        }
    }
}

impl FromStr for ResourceGroupScope {
    type Err = ResourceGroupScopeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "global" => Ok(ResourceGroupScope::Global),
            "address" => Ok(ResourceGroupScope::Address),
            "module_" => Ok(ResourceGroupScope::Module),
            _ => Err(ResourceGroupScopeError(s.to_string())),
        }
    }
}

#[derive(Debug, Error)]
#[error("Invalid resource group scope: {0}")]
pub struct ResourceGroupScopeError(String);
