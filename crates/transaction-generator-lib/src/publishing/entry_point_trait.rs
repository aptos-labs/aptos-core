// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::publish_util::Package;
use crate::publishing::prebuild_packages::PrebuiltPackagesBundle;
use aptos_framework::natives::code::PackageMetadata;
use aptos_sdk::{
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

pub trait PreBuiltPackages: std::fmt::Debug + Sync + Send {
    fn package_bundle(&self) -> &PrebuiltPackagesBundle;

    fn package_metadata(&self, package_name: &str) -> PackageMetadata {
        self.package_bundle()
            .get_package(package_name)
            .metadata
            .clone()
    }

    fn package_modules(&self, package_name: &str) -> Vec<(String, CompiledModule, u32)> {
        let mut results = vec![];
        let default_config = DeserializerConfig::new(VERSION_DEFAULT, IDENTIFIER_SIZE_MAX);

        let modules = &self.package_bundle().get_package(package_name).modules;
        for (module_name, bytes) in modules {
            let (module, binary_format_version) = if let Ok(module) =
                CompiledModule::deserialize_with_config(bytes, &default_config)
            {
                (module, VERSION_DEFAULT)
            } else {
                let module =
                    CompiledModule::deserialize(bytes).expect("Module must always deserialize");
                (module, VERSION_MAX)
            };
            results.push((module_name.to_owned(), module, binary_format_version));
        }

        results
    }

    fn package_script(&self, package_name: &str) -> Option<CompiledScript> {
        let scripts = &self.package_bundle().get_package(package_name).scripts;
        assert!(
            scripts.len() <= 1,
            "Only single script per package is supported"
        );

        scripts
            .last()
            .map(|bytes| CompiledScript::deserialize(bytes).expect("Script must deserialize"))
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
