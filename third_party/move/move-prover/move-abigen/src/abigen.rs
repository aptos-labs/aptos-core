// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
use heck::ToSnakeCase;
#[allow(unused_imports)]
use log::{debug, info, warn};
use move_binary_format::{file_format::Ability, CompiledModule};
use move_bytecode_verifier::script_signature;
use move_command_line_common::files::MOVE_COMPILED_EXTENSION;
use move_core_types::{
    abi::{ArgumentABI, ScriptABI, ScriptFunctionABI, TransactionScriptABI, TypeArgumentABI},
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag},
};
use move_model::{
    ast::Address,
    model::{FunctionEnv, GlobalEnv, ModuleEnv},
    ty,
    ty::ReferenceKind,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read, path::PathBuf};

/// Options passed into the ABI generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AbigenOptions {
    /// Where to find the .mv files of scripts.
    pub compiled_script_directory: String,
    /// Where to get the script bytes if held in memory
    pub in_memory_bytes: Option<BTreeMap<String, Vec<u8>>>,
    /// In which directory to store output.
    pub output_directory: String,
}

impl Default for AbigenOptions {
    fn default() -> Self {
        Self {
            compiled_script_directory: ".".to_string(),
            in_memory_bytes: None,
            output_directory: "abi".to_string(),
        }
    }
}

/// The ABI generator.
pub struct Abigen<'env> {
    /// Options.
    options: &'env AbigenOptions,
    /// Input definitions.
    env: &'env GlobalEnv,
    /// Map from file name to generated script ABI (if any).
    output: BTreeMap<String, ScriptABI>,
}

impl<'env> Abigen<'env> {
    /// Creates a new ABI generator.
    pub fn new(env: &'env GlobalEnv, options: &'env AbigenOptions) -> Self {
        Self {
            options,
            env,
            output: Default::default(),
        }
    }

    /// Returns the result of ABI generation, a vector of pairs of filenames
    /// and JSON content.
    pub fn into_result(mut self) -> Vec<(String, Vec<u8>)> {
        std::mem::take(&mut self.output)
            .into_iter()
            .map(|(path, abi)| {
                let content = bcs::to_bytes(&abi).expect("ABI serialization should not fail");
                (path, content)
            })
            .collect()
    }

    /// Generates ABIs for all script modules in the environment (excluding the dependency set).
    pub fn gen(&mut self) {
        for module in self.env.get_modules() {
            if module.is_primary_target() {
                let mut path = PathBuf::from(&self.options.output_directory);
                // We make a directory for all of the script function ABIs in a module. But, if
                // it's a script, we don't create a directory.
                if !module.is_script_module() {
                    path.push(
                        PathBuf::from(module.get_source_path())
                            .file_stem()
                            .expect("file extension"),
                    )
                }

                for abi in self
                    .compute_abi(&module)
                    .map_err(|err| {
                        format!(
                            "Error while processing file {:?}: {}",
                            module.get_source_path(),
                            err
                        )
                    })
                    .unwrap()
                {
                    // If the module is a script module, then the generated ABI is a transaction
                    // script ABI. If the module is not a script module, then all generated ABIs
                    // are script function ABIs.
                    let mut path = path.clone();
                    path.push(
                        PathBuf::from(abi.name())
                            .with_extension("abi")
                            .file_name()
                            .expect("file name"),
                    );
                    self.output.insert(path.to_str().unwrap().to_string(), abi);
                }
            }
        }
    }

    /// Compute the ABIs of all script functions in a module.
    fn compute_abi(&self, module_env: &ModuleEnv<'env>) -> anyhow::Result<Vec<ScriptABI>> {
        // Get all the script functions in this module
        let script_iter: Vec<_> = if module_env.is_script_module() {
            module_env.get_functions().collect()
        } else {
            let module = Self::get_compiled_module(module_env)?;
            module_env
                .get_functions()
                .filter(|func| {
                    let func_name = module_env.symbol_pool().string(func.get_name());
                    let func_ident = IdentStr::new(&func_name).unwrap();
                    // only pick up script functions that also have a script-callable signature.
                    // and check all arguments have a valid type tag
                    func.is_entry()
                        && script_signature::verify_module_function_signature_by_name(
                            module,
                            func_ident,
                            script_signature::no_additional_script_signature_checks,
                        )
                        .is_ok()
                        && func
                            .get_parameters()
                            .iter()
                            .skip_while(|param| match &param.1 {
                                ty::Type::Primitive(ty::PrimitiveType::Signer) => true,
                                ty::Type::Reference(_, inner) => matches!(
                                    &**inner,
                                    ty::Type::Primitive(ty::PrimitiveType::Signer)
                                ),
                                _ => false,
                            })
                            .all(|param| {
                                matches!(
                                    Self::get_type_tag(&param.1, module_env),
                                    Err(_) | Ok(Some(_))
                                )
                            })
                        && func.get_return_count() == 0
                })
                .collect()
        };

        let mut abis = Vec::new();
        for func in &script_iter {
            abis.push(self.generate_abi_for_function(func, module_env)?);
        }

        Ok(abis)
    }

    fn generate_abi_for_function(
        &self,
        func: &FunctionEnv<'env>,
        module_env: &ModuleEnv<'env>,
    ) -> anyhow::Result<ScriptABI> {
        let symbol_pool = module_env.symbol_pool();
        let name = symbol_pool.string(func.get_name()).to_string();
        let doc = func.get_doc().to_string();
        let ty_args = func
            .get_type_parameters()
            .iter()
            .map(|ty_param| {
                TypeArgumentABI::new(symbol_pool.string(ty_param.0).to_string().to_snake_case())
            })
            .collect();
        let args = func
            .get_parameters()
            .iter()
            .filter(|param| match &param.1 {
                ty::Type::Primitive(ty::PrimitiveType::Signer) => false,
                ty::Type::Reference(ReferenceKind::Immutable, inner) => {
                    !matches!(&**inner, ty::Type::Primitive(ty::PrimitiveType::Signer))
                },
                ty::Type::Struct(module_id, struct_id, _) => {
                    let struct_module_env = module_env.env.get_module(*module_id);
                    let abilities = struct_module_env.get_struct(*struct_id).get_abilities();
                    abilities.has_ability(Ability::Copy) && !abilities.has_ability(Ability::Key)
                },
                _ => true,
            })
            .map(|param| {
                let tag = Self::get_type_tag(&param.1, module_env)?.unwrap();
                Ok(ArgumentABI::new(
                    symbol_pool.string(param.0).to_string(),
                    tag,
                ))
            })
            .collect::<anyhow::Result<_>>()?;

        // This is a transaction script, so include the code, but no module ID
        if module_env.is_script_module() {
            let code = self.load_compiled_bytes(module_env)?.to_vec();
            Ok(ScriptABI::TransactionScript(TransactionScriptABI::new(
                name, doc, code, ty_args, args,
            )))
        } else {
            // This is a script function, so no code. But we need to include the module ID
            let module = Self::get_compiled_module(module_env)?;
            Ok(ScriptABI::ScriptFunction(ScriptFunctionABI::new(
                name,
                module.self_id(),
                doc,
                ty_args,
                args,
            )))
        }
    }

    fn load_compiled_bytes(&self, module_env: &ModuleEnv<'env>) -> anyhow::Result<Vec<u8>> {
        match &self.options.in_memory_bytes {
            Some(map) => {
                let path =
                    PathBuf::from(module_env.get_source_path().to_string_lossy().to_string())
                        .file_stem()
                        .expect("file stem")
                        .to_string_lossy()
                        .to_string();
                Ok(map.get(&path).unwrap().clone())
            },
            None => {
                let mut path = PathBuf::from(&self.options.compiled_script_directory);
                path.push(
                    PathBuf::from(module_env.get_source_path())
                        .with_extension(MOVE_COMPILED_EXTENSION)
                        .file_name()
                        .expect("file name"),
                );
                let mut f = match std::fs::File::open(path.clone()) {
                    Ok(f) => f,
                    Err(error) => bail!("Failed to open compiled file {:?}: {}", path, error),
                };
                let mut bytes = Vec::new();
                f.read_to_end(&mut bytes)?;
                Ok(bytes)
            },
        }
    }

    fn get_type_tag(
        ty0: &ty::Type,
        module_env: &ModuleEnv<'env>,
    ) -> anyhow::Result<Option<TypeTag>> {
        use ty::Type::*;
        let tag = match ty0 {
            Primitive(prim) => {
                use ty::PrimitiveType::*;
                match prim {
                    Bool => TypeTag::Bool,
                    U8 => TypeTag::U8,
                    U16 => TypeTag::U16,
                    U32 => TypeTag::U32,
                    U64 => TypeTag::U64,
                    U128 => TypeTag::U128,
                    U256 => TypeTag::U256,
                    Address => TypeTag::Address,
                    Signer => TypeTag::Signer,
                    Num | Range | EventStore => {
                        bail!("Type {:?} is not allowed in scripts.", ty0)
                    },
                }
            },
            Vector(ty) => {
                let tag = match Self::get_type_tag(ty, module_env)? {
                    Some(tag) => tag,
                    None => return Ok(None),
                };
                TypeTag::Vector(Box::new(tag))
            },
            Struct(module_id, struct_id, vec_type) => {
                let struct_module_env = module_env.env.get_module(*module_id);
                let abilities = struct_module_env.get_struct(*struct_id).get_abilities();
                if abilities.has_ability(Ability::Copy) && !abilities.has_ability(Ability::Key) {
                    let mut type_args = vec![];
                    for e in vec_type {
                        let type_arg = match Self::get_type_tag(e, module_env)? {
                            Some(type_param) => type_param,
                            None => return Ok(None),
                        };
                        type_args.push(type_arg);
                    }
                    let address = if let Address::Numerical(a) = &struct_module_env.self_address() {
                        *a
                    } else {
                        bail!("expected no symbolic addresses")
                    };
                    TypeTag::Struct(Box::new(StructTag {
                        address,
                        module: struct_module_env
                            .get_identifier()
                            .ok_or_else(|| anyhow!("expected compiled module"))?,
                        name: struct_module_env
                            .get_struct(*struct_id)
                            .get_identifier()
                            .unwrap_or_else(|| {
                                panic!("type {:?} is not allowed in entry function", ty0)
                            }),
                        type_args,
                    }))
                } else {
                    return Ok(None);
                }
            },
            Tuple(_)
            | TypeParameter(_)
            | Fun(..)
            | TypeDomain(_)
            | ResourceDomain(..)
            | Error
            | Var(_)
            | Reference(_, _) => return Ok(None),
        };
        Ok(Some(tag))
    }

    fn get_compiled_module<'a>(
        module_env: &'a ModuleEnv<'a>,
    ) -> anyhow::Result<&'a CompiledModule> {
        module_env
            .get_verified_module()
            .ok_or_else(|| anyhow!("no attached compiled module"))
    }
}
