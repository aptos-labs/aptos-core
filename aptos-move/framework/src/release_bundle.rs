// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::built_package::BuiltPackage;
use crate::natives::code::PackageMetadata;
use crate::path_in_crate;
use anyhow::Context;
use aptos_types::account_address::AccountAddress;
use move_deps::move_binary_format::access::ModuleAccess;
use move_deps::move_binary_format::errors::PartialVMError;
use move_deps::move_binary_format::CompiledModule;
use move_deps::move_command_line_common::files::{
    extension_equals, find_filenames, MOVE_EXTENSION,
};
use move_deps::move_core_types::language_storage::ModuleId;
use move_deps::move_model::code_writer::CodeWriter;
use move_deps::move_model::model::Loc;
use move_deps::move_model::{emit, emitln};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

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

/// A release package consists of package metdata and the code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleasePackage {
    metadata: PackageMetadata,
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
            self.sort_by_deps(&map, &mut order, id.clone());
        }
        let mut result = vec![];
        for id in order {
            let (code, module) = map.remove(&id).unwrap();
            result.push((code, module))
        }
        result
    }

    fn sort_by_deps(
        &self,
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
                self.sort_by_deps(map, order, dep);
            }
        }
        order.push(id)
    }

    pub fn generate_script_proposal(
        &self,
        for_address: AccountAddress,
        out: PathBuf,
    ) -> anyhow::Result<()> {
        let writer = CodeWriter::new(Loc::default());
        emitln!(
            writer,
            "// Upgrade proposal for package `{}`\n",
            self.metadata.name
        );
        emitln!(writer, "// source digest: {}", self.metadata.source_digest);
        emitln!(writer, "script {");
        writer.indent();
        emitln!(writer, "use std::vector;");
        emitln!(writer, "use aptos_framework::aptos_governance;");
        emitln!(writer, "use aptos_framework::code;\n");
        emitln!(writer, "fun main(proposal_id: u64){");
        writer.indent();

        emitln!(
            writer,
            "let framework_signer = aptos_governance::resolve(proposal_id, @{});",
            for_address
        );
        emit!(writer, "let code = ");
        Self::generate_blobs(&writer, &self.code);
        emitln!(writer, ";");

        // The package metadata can be larger than 64k, which is the max for Move constants.
        // We therefore have to split it into chunks. Three chunks should be large enough
        // to cover any current and future needs. We then dynamically append them to obtain
        // the result.
        let mut metadata = bcs::to_bytes(&self.metadata)?;
        let chunk_size = metadata.len() / 3;
        for i in 1..4 {
            let to_drain = if i == 3 { metadata.len() } else { chunk_size };
            let chunk = metadata.drain(0..to_drain).collect::<Vec<_>>();
            emit!(writer, "let chunk{} = ", i);
            Self::generate_blob(&writer, &chunk);
            emitln!(writer, ";")
        }
        emitln!(writer, "vector::append(&mut chunk1, chunk2);");
        emitln!(writer, "vector::append(&mut chunk1, chunk3);");
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

    fn generate_blobs(writer: &CodeWriter, blobs: &[Vec<u8>]) {
        emitln!(writer, "vector[");
        writer.indent();
        for blob in blobs {
            Self::generate_blob(writer, blob);
            emitln!(writer, ",")
        }
        writer.unindent();
        emit!(writer, "]");
    }

    fn generate_blob(writer: &CodeWriter, data: &[u8]) {
        emitln!(writer, "vector[");
        writer.indent();
        for (i, b) in data.iter().enumerate() {
            if (i + 1) % 20 == 0 {
                emitln!(writer);
            }
            emit!(writer, "{}u8,", b);
        }
        emitln!(writer);
        writer.unindent();
        emit!(writer, "]")
    }
}
