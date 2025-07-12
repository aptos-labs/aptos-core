// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Entry point into the Move assembler ('move-asm').

pub mod compiler;
pub mod module_builder;
pub mod syntax;

use crate::{
    module_builder::ModuleBuilderOptions,
    syntax::{AsmResult, Diag},
};
use anyhow::bail;
use clap::Parser;
use codespan_reporting::{
    files::{Files, SimpleFile},
    term,
    term::termcolor::WriteColor,
};
use either::Either;
use move_binary_format::{file_format::CompiledScript, CompiledModule};
use std::{fs, io::Write, path::PathBuf};

#[derive(Parser, Clone, Debug, Default)]
#[clap(author, version, about)]
pub struct Options {
    /// Options for the module builder
    #[clap(flatten)]
    pub module_builder_options: ModuleBuilderOptions,

    /// Directory where to place assembled code.
    #[clap(short, long, default_value = "")]
    pub output_dir: String,

    /// Input file.
    pub inputs: Vec<String>,
}

/// Assembles source as specified by options.
pub fn run<W>(error_writer: &mut W) -> anyhow::Result<()>
where
    W: Write + WriteColor,
{
    let options = Options::parse();
    if options.inputs.len() != 1 {
        bail!("expected exactly one file name for the assembler source")
    }
    let input_path = options.inputs.first().unwrap();
    let input = fs::read_to_string(input_path)?;

    let result = match assemble(&options, &input) {
        Ok(x) => x,
        Err(diags) => {
            let diag_file = SimpleFile::new(&input_path, &input);
            report_diags(error_writer, &diag_file, diags);
            bail!("exiting with errors")
        },
    };

    let path = PathBuf::from(input_path).with_extension("mv");
    let mut out_path = PathBuf::from(options.output_dir);
    out_path.push(path.file_name().expect("file name available"));
    let mut bytes = vec![];
    match result {
        Either::Left(m) => m
            .serialize_for_version(
                Some(options.module_builder_options.bytecode_version),
                &mut bytes,
            )
            .expect("serialization succeeds"),
        Either::Right(s) => s
            .serialize_for_version(
                Some(options.module_builder_options.bytecode_version),
                &mut bytes,
            )
            .expect("serialization succeeds"),
    }
    if let Err(e) = fs::write(&out_path, &bytes) {
        bail!("failed to write result to `{}`: {}", out_path.display(), e);
    }
    Ok(())
}

pub fn assemble(
    options: &Options,
    input: &str,
) -> AsmResult<Either<CompiledModule, CompiledScript>> {
    let ast = syntax::parse_asm(input)?;
    let result = compiler::compile(
        options.module_builder_options.clone(),
        std::iter::empty(),
        ast,
    )?;
    Ok(result)
}

pub(crate) fn report_diags<'a, W: Write + WriteColor>(
    error_writer: &mut W,
    files: &'a impl Files<'a, FileId = ()>,
    diags: Vec<Diag>,
) {
    for diag in diags {
        term::emit(error_writer, &term::Config::default(), files, &diag)
            .unwrap_or_else(|_| eprintln!("failed to print diagnostics"))
    }
}
