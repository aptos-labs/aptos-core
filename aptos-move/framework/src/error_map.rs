// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module contains a generator for the Move core type `ErrorMapping`. This generator
//! is small enough that is copied over from the Move repo.
//!
//! This code, based on the `move-model` crate, is also a good example how the model can be
//! leveraged to do whole program traversals and analysis.

use crate::built_package::BuildOptions;
use move_deps::move_core_types::account_address::AccountAddress;
use move_deps::move_core_types::errmap::{ErrorDescription, ErrorMapping};
use move_deps::move_core_types::identifier::Identifier;
use move_deps::move_core_types::language_storage::ModuleId;
use move_deps::move_model::ast::Value;
use move_deps::move_model::model::{GlobalEnv, ModuleEnv, NamedConstantEnv};
use move_deps::move_model::symbol::Symbol;
use move_deps::move_package::{BuildConfig, ModelConfig};
use std::path::Path;
use std::{convert::TryFrom, rc::Rc};

const ERROR_PREFIX: &str = "E";

pub(crate) fn generate_error_map(
    package_path: &Path,
    options: &BuildOptions,
) -> Option<ErrorMapping> {
    let build_config = BuildConfig {
        dev_mode: false,
        additional_named_addresses: options.named_addresses.clone(),
        architecture: None,
        generate_abis: false,
        generate_docs: false,
        install_dir: None,
        test_mode: false,
        force_recompilation: false,
        fetch_deps_only: false,
        fetch_latest_git_deps: false,
    };
    if let Ok(model) = build_config.move_model_for_package(
        package_path,
        ModelConfig {
            target_filter: None,
            all_files_as_targets: true,
        },
    ) {
        let mut gen = ErrorMapGenerator::new(&model);
        gen.gen();
        Some(gen.finish())
    } else {
        None
    }
}

struct ErrorMapGenerator<'env> {
    /// Input definitions
    env: &'env GlobalEnv,
    /// Output error mapping
    output: ErrorMapping,
    /// The id of the module defining error categories
    error_category_module: ModuleId,
}

impl<'env> ErrorMapGenerator<'env> {
    fn new(env: &'env GlobalEnv) -> Self {
        Self {
            env,
            output: ErrorMapping::default(),
            error_category_module: ModuleId::new(
                AccountAddress::ONE,
                Identifier::new("error").unwrap(),
            ),
        }
    }

    fn finish(self) -> ErrorMapping {
        self.output
    }

    fn gen(&mut self) {
        for module in self.env.get_modules() {
            if !module.is_script_module() {
                self.build_error_map(&module)
            }
        }
    }

    fn build_error_map(&mut self, module: &ModuleEnv<'_>) {
        let module_id = self.get_module_id_for_name(module);
        if module_id == self.error_category_module {
            self.build_error_categories(module)
        } else {
            self.build_error_map_for_module(&module_id, module)
        }
    }

    fn build_error_categories(&mut self, module: &ModuleEnv<'_>) {
        for named_constant in module.get_named_constants() {
            let name = self.name_string(named_constant.get_name());
            if let Some(error_category) = self.get_abort_code(&named_constant) {
                // If an error is returned (because of duplicate entry) ignore it.
                let _ = self.output.add_error_category(
                    error_category,
                    ErrorDescription {
                        code_name: name.trim().to_string(),
                        code_description: named_constant.get_doc().trim().to_string(),
                    },
                );
            }
        }
    }

    fn build_error_map_for_module(&mut self, module_id: &ModuleId, module: &ModuleEnv<'_>) {
        for named_constant in module.get_named_constants() {
            let name = self.name_string(named_constant.get_name());
            if name.starts_with(ERROR_PREFIX) {
                if let Some(abort_code) = self.get_abort_code(&named_constant) {
                    // If an error is returned (because of duplicate entry) ignore it.
                    let _ = self.output.add_module_error(
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
    }

    fn get_abort_code(&self, constant: &NamedConstantEnv<'_>) -> Option<u64> {
        match constant.get_value() {
            Value::Number(big_int) => u64::try_from(big_int).ok(),
            _ => None,
        }
    }

    fn get_module_id_for_name(&self, module: &ModuleEnv<'_>) -> ModuleId {
        let name = module.get_name();
        let addr = AccountAddress::from_hex_literal(&format!("0x{:x}", name.addr())).unwrap();
        let name = Identifier::new(self.name_string(name.name()).to_string()).unwrap();
        ModuleId::new(addr, name)
    }

    fn name_string(&self, symbol: Symbol) -> Rc<String> {
        self.env.symbol_pool().string(symbol)
    }
}
