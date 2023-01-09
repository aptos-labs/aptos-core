// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{KnownAttribute, RuntimeModuleMetadataV1};
use move_core_types::{
    account_address::AccountAddress,
    errmap::{ErrorDescription, ErrorMapping},
    identifier::Identifier,
    language_storage::ModuleId,
};
use move_model::{
    ast::{Attribute, Value},
    model::{
        FunctionEnv, FunctionVisibility, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, Parameter,
        QualifiedId, StructId,
    },
    symbol::Symbol,
    ty::{PrimitiveType, Type},
};
use std::{collections::BTreeMap, rc::Rc};

const INIT_MODULE_FUN: &str = "init_module";
const VIEW_FUN_ATTRIBUTE: &str = "view";
const LEGAC_ENTRY_FUN_ATTRIBUTE: &str = "legacy_entry_fun";
const ERROR_PREFIX: &str = "E";

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
                self.check_init_module(module);
                self.check_entry_functions(module);
                self.check_and_record_view_functions(module);
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
            if fun.visibility() != FunctionVisibility::Private {
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
            if self.has_attribute(fun, LEGAC_ENTRY_FUN_ATTRIBUTE) {
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
            Reference(false, bt) if matches!(bt.as_ref(), Primitive(PrimitiveType::Signer)) => {
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
        matches!(name.as_str(), "0x1::string::String")
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
                        module_id.clone(),
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
            .remove(&module_id)
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
        let addr = AccountAddress::from_hex_literal(&format!("0x{:x}", name.addr())).unwrap();
        let name = Identifier::new(self.name_string(name.name()).to_string()).unwrap();
        ModuleId::new(addr, name)
    }

    fn name_string(&self, symbol: Symbol) -> Rc<String> {
        self.env.symbol_pool().string(symbol)
    }
}
