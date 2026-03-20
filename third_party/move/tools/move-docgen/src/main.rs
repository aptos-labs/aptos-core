// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use clap::Parser;
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{ColorChoice, StandardStream},
};
use move_docgen::{Docgen, DocgenOptions};
use move_model::metadata::LanguageVersion;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[clap(name = "move-docgen", about = "Move documentation generator")]
struct Cli {
    /// Move source files to document
    #[clap(required = true)]
    sources: Vec<String>,

    /// Dependency directories
    #[clap(long = "dependency", short = 'd', action = clap::ArgAction::Append)]
    deps: Vec<String>,

    /// Named address values (e.g., std=0x1)
    #[clap(long = "named-addresses", short = 'a', num_args = 0..)]
    named_addresses: Vec<String>,

    /// Language version
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion))]
    language_version: Option<LanguageVersion>,

    /// Skip attribute checks
    #[clap(long)]
    skip_attribute_checks: bool,

    #[clap(flatten)]
    docgen: DocgenOptions,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    let compiler_options = move_compiler_v2::Options {
        sources: cli.sources,
        dependencies: cli.deps,
        named_address_mapping: cli.named_addresses,
        language_version: cli.language_version,
        skip_attribute_checks: cli.skip_attribute_checks,
        compile_verify_code: true,
        ..Default::default()
    };
    let model =
        move_compiler_v2::run_move_compiler_for_analysis(&mut error_writer, compiler_options)?;

    let generator = Docgen::new(&model, &cli.docgen);
    for (file, content) in generator.r#gen() {
        let path = PathBuf::from(&file);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path.as_path(), content)?;
    }

    model.report_diag(&mut error_writer, Severity::Warning);
    if model.has_errors() {
        Err(anyhow!("documentation generation failed"))
    } else {
        Ok(())
    }
}
