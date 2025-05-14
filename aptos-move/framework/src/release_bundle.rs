// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{built_package::BuiltPackage, natives::code::PackageMetadata, path_in_crate};
use anyhow::Context;
use aptos_types::account_address::AccountAddress;
use move_binary_format::{access::ModuleAccess, errors::PartialVMError, CompiledModule};
use move_command_line_common::files::{extension_equals, find_filenames, MOVE_EXTENSION};
use move_core_types::language_storage::ModuleId;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use aptos_crypto::HashValue;

/// A release bundle consists of a list of release packages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseBundle {
    /// The packages in this release bundle. The order is bottom up regarding dependencies,
    /// such the packages can be deployed in order as given.
    pub packages: Vec<ReleasePackage>,
    /// A set of paths to directories where Move sources constituting this package are found.
    /// This may or not may be populated.
    pub source_dirs: Vec<String>,
}

/// A release package consists of package metadata and the code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleasePackage {
    pub metadata: PackageMetadata,
    code: Vec<Vec<u8>>,
}

impl ReleaseBundle {
    /// Create a bundle from the list of packages. No path information is available.
    /// If the `source_dirs` is empty, the `files` function will not be available for the
    /// bundle.
    pub fn new(packages: Vec<ReleasePackage>, source_dirs: Vec<String>) -> Self {
        Self {
            packages,
            source_dirs,
        }
    }

    /// Read a release bundle from a file.
    pub fn read(file: PathBuf) -> anyhow::Result<ReleaseBundle> {
        let content =
            std::fs::read(&file).with_context(|| format!("while reading `{}`", file.display()))?;
        Ok(bcs::from_bytes::<ReleaseBundle>(&content)?)
    }

    /// Write a release bundle to file
    pub fn write(&self, path: PathBuf) -> anyhow::Result<()> {
        std::fs::write(&path, bcs::to_bytes(self)?)
            .with_context(|| format!("while writing `{}`", path.display()))?;
        Ok(())
    }

    /// Returns a list of all module bytecodes in this bundle.
    pub fn code(&self) -> Vec<&[u8]> {
        let mut result = vec![];
        for pack in &self.packages {
            let mut code = pack.code();
            result.append(&mut code);
        }
        result
    }

    /// Return a list of CompiledModules in this bundle.
    pub fn compiled_modules(&self) -> Vec<CompiledModule> {
        self.code_and_compiled_modules()
            .into_iter()
            .map(|(_, c)| c)
            .collect()
    }

    /// Return a list of bytecode and CompiledModules in this bundle.
    pub fn code_and_compiled_modules(&self) -> Vec<(&[u8], CompiledModule)> {
        self.code()
            .into_iter()
            .map(|bc| (bc, CompiledModule::deserialize(bc).unwrap()))
            .collect()
    }

    /// Some legacy usages of code require a full copy. This is a helper for those cases.
    /// TODO: remove unnecessary use of this function
    pub fn legacy_copy_code(&self) -> Vec<Vec<u8>> {
        self.code().into_iter().map(|v| v.to_vec()).collect()
    }

    /// Returns the Move source file names which are involved in this bundle.
    pub fn files(&self) -> anyhow::Result<Vec<String>> {
        assert!(
            !self.source_dirs.is_empty(),
            "release bundle has no source path information"
        );
        let mut result = vec![];
        for path in &self.source_dirs {
            let path = path_in_crate(path);
            let mut files = find_filenames(&[&path], |p| extension_equals(p, MOVE_EXTENSION))?;
            result.append(&mut files);
        }
        Ok(result)
    }
}

impl ReleasePackage {
    /// Creates a new released package.
    pub fn new(package: BuiltPackage) -> anyhow::Result<Self> {
        // TODO: remove poliocy and put it into toml
        let metadata = package.extract_metadata()?;
        Ok(ReleasePackage {
            metadata,
            code: package.extract_code(),
        })
    }

    /// Returns the name of the package.
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Returns a vector of code slices representing the bytecode of modules in this bundle.
    pub fn code(&self) -> Vec<&[u8]> {
        self.code.iter().map(|v| v.as_slice()).collect()
    }

    /// For a valid index in the code vector, return the `CompiledModule`.
    pub fn compiled_module_at(&self, idx: usize) -> Result<CompiledModule, PartialVMError> {
        CompiledModule::deserialize(&self.code[idx])
    }

    /// Returns the package metadata.
    pub fn package_metadata(&self) -> &PackageMetadata {
        &self.metadata
    }

    /// Returns the package metadata, mutable.
    pub fn package_metadata_mut(&mut self) -> &mut PackageMetadata {
        &mut self.metadata
    }

    /// Returns code and compiled modules, topological sorted regarding dependencies.
    pub fn sorted_code_and_modules(&self) -> Vec<(&[u8], CompiledModule)> {
        let mut map = self
            .code
            .iter()
            .map(|c| {
                let m = CompiledModule::deserialize(c).unwrap();
                (m.self_id(), (c.as_slice(), m))
            })
            .collect::<BTreeMap<_, _>>();
        let mut order = vec![];
        for id in map.keys() {
            sort_by_deps(&map, &mut order, id.clone());
        }
        let mut result = vec![];
        for id in order {
            let (code, module) = map.remove(&id).unwrap();
            result.push((code, module))
        }
        result
    }

    pub fn generate_script_proposal(
        &self,
        for_address: AccountAddress,
        out: PathBuf,
        function_name: String,
    ) -> anyhow::Result<()> {
        self.generate_script_proposal_impl(for_address, out, false, false, None, function_name)
    }

    pub fn generate_script_proposal_testnet(
        &self,
        for_address: AccountAddress,
        out: PathBuf,
        function_name: String,
    ) -> anyhow::Result<()> {
        self.generate_script_proposal_impl(for_address, out, true, false, None, function_name)
    }

    pub fn generate_script_proposal_multi_step(
        &self,
        for_address: AccountAddress,
        out: PathBuf,
        next_execution_hash: Option<HashValue>,
        function_name: String
    ) -> anyhow::Result<()> {
        self.generate_script_proposal_impl(for_address, out, true, true, next_execution_hash, function_name)
    }

    fn generate_script_proposal_impl(
        &self,
        for_address: AccountAddress,
        out: PathBuf,
        is_testnet: bool,
        is_multi_step: bool,
        next_execution_hash: Option<HashValue>,
        function_name: String,
    ) -> anyhow::Result<()> {
        let writer = CodeWriter::new(Loc::default());
        emitln!(
            writer,
            "// Upgrade proposal for package `{}`\n",
            self.metadata.name
        );

        // The Sha2-256 digest here is the combined hash of all the hashes of the `.move` files and
        // the manifest file(Move.toml) in the source package.
        // Check [move_package::resolution::digest::compile_digest]
        emitln!(writer, "// source package's SHA2-256 digest: {}", self.metadata.source_digest);
        emitln!(writer, "script {");
        writer.indent();
        emitln!(writer, "use std::vector;");
        emitln!(writer, "use supra_framework::supra_governance;");
        emitln!(writer, "use supra_framework::code;\n");

        if is_testnet && !is_multi_step {
            emitln!(writer, "fun {function_name} (core_resources: &signer) {{");
            writer.indent();
            emitln!(
                writer,
                "let framework_signer = supra_governance::get_signer_testnet_only(core_resources, @{});",
                for_address
            );
        } else if !is_multi_step {
            emitln!(writer, "fun {} (proposal_id: u64) {{", function_name);
            writer.indent();
            emitln!(
                writer,
                "let framework_signer = supra_governance::supra_resolve(proposal_id, @{});",
                for_address
            );
        } else {
            emitln!(writer, "fun {} (proposal_id: u64) {{", function_name);
            writer.indent();
            Self::generate_next_execution_hash_blob(&writer, for_address, next_execution_hash);
        }

        emitln!(writer, "let code = vector::empty();");

        for i in 0..self.code.len() {
            emitln!(writer, "let chunk{} = ", i);
            Self::generate_blob_as_hex_string(&writer, &self.code[i]);
            emitln!(writer, ";");
            emitln!(writer, "vector::push_back(&mut code, chunk{});", i);
        }

        // The package metadata can be larger than 64k, which is the max for Move constants.
        // We therefore have to split it into chunks. Three chunks should be large enough
        // to cover any current and future needs. We then dynamically append them to obtain
        // the result.
        let mut metadata = bcs::to_bytes(&self.metadata)?;
        let chunk_size = (u16::MAX / 2) as usize;
        let num_of_chunks = (metadata.len() / chunk_size) + 1;

        for i in 1..num_of_chunks + 1 {
            let to_drain = if i == num_of_chunks {
                metadata.len()
            } else {
                chunk_size
            };
            let chunk = metadata.drain(0..to_drain).collect::<Vec<_>>();
            emit!(writer, "let chunk{} = ", i);
            Self::generate_blob_as_hex_string(&writer, &chunk);
            emitln!(writer, ";")
        }

        for i in 2..num_of_chunks + 1 {
            emitln!(writer, "vector::append(&mut chunk1, chunk{});", i);
        }

        emitln!(
            writer,
            "code::publish_package_txn(&framework_signer, chunk1, code)"
        );
        writer.unindent();
        emitln!(writer, "}");
        writer.unindent();
        emitln!(writer, "}");
        writer.process_result(|s| std::fs::write(&out, s))?;
        Ok(())
    }

    fn generate_blob_as_hex_string(writer: &CodeWriter, data: &[u8]) {
        emit!(writer, "x\"");
        for b in data.iter() {
            emit!(writer, "{:02x}", b);
        }
        emit!(writer, "\"");
    }

    fn generate_next_execution_hash_blob(
        writer: &CodeWriter,
        for_address: AccountAddress,
        next_execution_hash: Option<HashValue>,
    ) {
        if let Some(hash) = next_execution_hash {
            emitln!(
                writer,
                "let framework_signer = supra_governance::resolve_supra_multi_step_proposal("
            );
            writer.indent();
            emitln!(writer, "proposal_id,");
            emitln!(writer, "@{},", for_address);
            emitln!(writer, "x\"{:x}\"", hash);
            writer.unindent();
            emitln!(writer, ");");
        } else {
            emitln!(
                writer,
                "let framework_signer = supra_governance::resolve_supra_multi_step_proposal(proposal_id, @{}, {});\n",
                for_address,
                "vector::empty<u8>()",
            );
        }
    }
}

fn sort_by_deps(
    map: &BTreeMap<ModuleId, (&[u8], CompiledModule)>,
    order: &mut Vec<ModuleId>,
    id: ModuleId,
) {
    if order.contains(&id) {
        return;
    }
    let compiled = &map.get(&id).unwrap().1;
    for dep in compiled.immediate_dependencies() {
        // Only consider deps which are actually in this package. Deps for outside
        // packages are considered fine because of package deployment order. Note
        // that because of this detail, we can't use existing topsort from Move utils.
        if map.contains_key(&dep) {
            sort_by_deps(map, order, dep);
        }
    }
    order.push(id)
}
