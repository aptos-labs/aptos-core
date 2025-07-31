// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::publish_util::Package;
use aptos_framework::natives::code::PackageMetadata;
use aptos_sdk::{
    move_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    },
    types::transaction::{EntryFunction, TransactionPayload},
};
use move_binary_format::{file_format::CompiledScript, CompiledModule};
use rand::rngs::StdRng;
use std::path::PathBuf;

pub trait PreBuiltPackages: std::fmt::Debug + Sync + Send {
    fn package_metadata_path(&self, package_name: &str) -> PathBuf;
    fn package_modules_paths(
        &self,
        package_name: &str,
    ) -> anyhow::Result<Box<dyn Iterator<Item = PathBuf>>>;
    fn package_script_path(&self, package_name: &str) -> PathBuf;

    fn package_metadata(&self, package_name: &str) -> PackageMetadata;

    fn package_modules(&self, package_name: &str) -> Vec<(String, CompiledModule, u32)>;

    fn package_script(&self, package_name: &str) -> Option<CompiledScript>;
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
