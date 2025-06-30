// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::publish_util::Package;
use aptos_framework::natives::code::PackageMetadata;
use aptos_sdk::{
    bcs,
    move_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    },
    types::transaction::{EntryFunction, TransactionPayload},
};
use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format::CompiledScript,
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_DEFAULT, VERSION_MAX},
    CompiledModule,
};
use rand::rngs::StdRng;
use std::{fs, path::PathBuf};

pub trait PreBuiltPackages: std::fmt::Debug + Sync + Send {
    fn package_metadata_path(&self, package_name: &str) -> PathBuf;
    fn package_modules_paths(&self, package_name: &str) -> Box<dyn Iterator<Item = PathBuf>>;
    fn package_script_path(&self, package_name: &str) -> PathBuf;

    fn package_metadata(&self, package_name: &str) -> PackageMetadata {
        let path = self.package_metadata_path(package_name);
        let bytes = fs::read(&path).unwrap_or_else(|err| panic!("Failed to read {path:?}: {err}"));
        bcs::from_bytes::<PackageMetadata>(&bytes).expect("Package metadata must deserialize")
    }

    fn package_modules(&self, package_name: &str) -> Vec<(String, CompiledModule, u32)> {
        let paths = self.package_modules_paths(package_name);

        let default_config = DeserializerConfig::new(VERSION_DEFAULT, IDENTIFIER_SIZE_MAX);
        let mut results = vec![];

        for module_path in paths {
            let bytes = fs::read(&module_path)
                .unwrap_or_else(|err| panic!("Cannot read module file {module_path:?}: {err}"));
            let (module, binary_format_version) = if let Ok(module) =
                CompiledModule::deserialize_with_config(&bytes, &default_config)
            {
                (module, VERSION_DEFAULT)
            } else {
                let module =
                    CompiledModule::deserialize(&bytes).expect("Module must always deserialize");
                (module, VERSION_MAX)
            };

            results.push((
                module.self_id().name().to_string(),
                module,
                binary_format_version,
            ));
        }

        results
    }

    fn package_script(&self, package_name: &str) -> Option<CompiledScript> {
        let path = self.package_script_path(package_name);
        fs::read(&path)
            .ok()
            .map(|bytes| CompiledScript::deserialize(&bytes).expect("Script must deserialize"))
    }
}

pub enum MultiSigConfig {
    None,
    Random(usize),
    Publisher,
    FeePayerPublisher,
}

/// Automatic arguments function expects (i.e. signer, or multiple signers, etc)
/// That execution can add before the call.
#[derive(Debug, Copy, Clone)]
pub enum AutomaticArgs {
    None,
    Signer,
    SignerAndMultiSig,
}

pub trait EntryPointTrait: std::fmt::Debug + Sync + Send + CloneEntryPointTrait {
    fn pre_built_packages(&self) -> &'static dyn PreBuiltPackages;

    fn package_name(&self) -> &'static str;

    fn module_name(&self) -> &'static str;

    fn create_payload(
        &self,
        package: &Package,
        module_name: &str,
        rng: Option<&mut StdRng>,
        other: Option<&AccountAddress>,
    ) -> TransactionPayload;

    fn initialize_entry_point(&self) -> Option<Box<dyn EntryPointTrait>> {
        None
    }

    fn multi_sig_additional_num(&self) -> MultiSigConfig {
        MultiSigConfig::None
    }

    fn automatic_args(&self) -> AutomaticArgs {
        AutomaticArgs::None
    }
}

pub fn get_payload(
    module_id: ModuleId,
    func: Identifier,
    args: Vec<Vec<u8>>,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(module_id, func, vec![], args))
}

pub trait CloneEntryPointTrait {
    fn clone_entry_points(&self) -> Box<dyn EntryPointTrait>;
}

impl<T> CloneEntryPointTrait for T
where
    T: EntryPointTrait + Clone + 'static,
{
    fn clone_entry_points(&self) -> Box<dyn EntryPointTrait> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn EntryPointTrait> {
    fn clone(&self) -> Box<dyn EntryPointTrait> {
        self.clone_entry_points()
    }
}
