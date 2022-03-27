// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! # Code generator for Move script builders
//!
//! '''bash
//! cargo run -p transaction-builder-generator -- --help
//! '''

use serde_generate as serdegen;
use serde_reflection::Registry;
use std::path::PathBuf;
use structopt::{clap::arg_enum, StructOpt};
use transaction_builder_generator as buildgen;

arg_enum! {
#[derive(Debug, StructOpt)]
enum Language {
    Python3,
    Rust,
    Cpp,
    Java,
    Csharp,
    Go,
    TypeScript,
    Swift,
}
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Transaction builder generator",
    about = "Generate code for Move script builders"
)]
struct Options {
    /// Path to the directory containing ABI files in BCS encoding.
    abi_directories: Vec<PathBuf>,

    /// Language for code generation.
    #[structopt(long, possible_values = &Language::variants(), case_insensitive = true, default_value = "Python3")]
    language: Language,

    /// Directory where to write generated modules (otherwise print code on stdout).
    #[structopt(long)]
    target_source_dir: Option<PathBuf>,

    /// Also install the aptos types described by the given YAML file, along with the BCS runtime.
    #[structopt(long)]
    with_aptos_types: Option<PathBuf>,

    /// Module name for the transaction builders installed in the `target_source_dir`.
    /// * Rust crates may contain a version number, e.g. "test:1.2.0".
    /// * In Java, this is expected to be a package name, e.g. "com.test" to create Java files in `com/test`.
    /// * In Go, this is expected to be of the format "go_module/path/go_package_name",
    /// and `aptos_types` is assumed to be in "go_module/path/aptos_types".
    #[structopt(long)]
    module_name: Option<String>,

    /// Optional package name (Python) or module path (Go) of the Serde and BCS runtime dependencies.
    #[structopt(long)]
    serde_package_name: Option<String>,

    /// Optional version number for the `aptos_types` module (useful in Rust).
    /// If `--with-aptos-types` is passed, this will be the version of the generated `aptos_types` module.
    #[structopt(long, default_value = "0.1.0")]
    aptos_version_number: String,

    /// Optional package name (Python) or module path (Go) of the `aptos_types` dependency.
    #[structopt(long)]
    package_name: Option<String>,

    /// Read custom code for Aptos containers from the given file paths. Containers will be matched with file stems.
    /// (e.g. `AddressAccount` <- `path/to/AddressAccount.py`)
    #[structopt(long)]
    with_custom_aptos_code: Vec<PathBuf>,
}

fn main() {
    let options = Options::from_args();
    let abis =
        buildgen::read_abis(&options.abi_directories).expect("Failed to read ABI in directory");

    let install_dir = match options.target_source_dir {
        None => {
            // Nothing to install. Just print to stdout.
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            match options.language {
                Language::Python3 => buildgen::python3::output(
                    &mut out,
                    options.serde_package_name.clone(),
                    options.package_name.clone(),
                    &abis,
                )
                .unwrap(),
                Language::Rust => {
                    buildgen::rust::output(&mut out, &abis, /* local types */ false).unwrap()
                }
                Language::Cpp => {
                    buildgen::cpp::output(&mut out, &abis, options.module_name.as_deref()).unwrap()
                }
                Language::Java => {
                    panic!("Code generation in Java requires --target_source_dir");
                }
                Language::Go => {
                    buildgen::golang::output(
                        &mut out,
                        options.serde_package_name.clone(),
                        options.package_name.clone(),
                        options.module_name.as_deref().unwrap_or("main").to_string(),
                        &abis,
                    )
                    .unwrap();
                }
                Language::TypeScript => {
                    buildgen::typescript::output(&mut out, &abis).unwrap();
                }
                Language::Swift => {
                    buildgen::swift::output(&mut out, &abis).unwrap();
                }
                Language::Csharp => {
                    panic!("Code generation in C# requires --target_source_dir");
                }
            }
            return;
        }
        Some(dir) => dir,
    };

    // Aptos types
    if let Some(registry_file) = options.with_aptos_types {
        let installer: Box<dyn serdegen::SourceInstaller<Error = Box<dyn std::error::Error>>> =
            match options.language {
                Language::Python3 => Box::new(serdegen::python3::Installer::new(
                    install_dir.clone(),
                    options.serde_package_name.clone(),
                )),
                Language::Rust => Box::new(serdegen::rust::Installer::new(install_dir.clone())),
                Language::Cpp => Box::new(serdegen::cpp::Installer::new(install_dir.clone())),
                Language::Java => Box::new(serdegen::java::Installer::new(install_dir.clone())),
                Language::Csharp => Box::new(serdegen::csharp::Installer::new(install_dir.clone())),
                Language::TypeScript => {
                    Box::new(serdegen::typescript::Installer::new(install_dir.clone()))
                }
                Language::Swift => Box::new(serdegen::swift::Installer::new(install_dir.clone())),
                Language::Go => Box::new(serdegen::golang::Installer::new(
                    install_dir.clone(),
                    options.serde_package_name.clone(),
                )),
            };

        match options.language {
            // In Rust and Go, runtimes are deployed using a global package manager.
            Language::Rust | Language::Go => (),
            _ => {
                installer.install_serde_runtime().unwrap();
                installer.install_bcs_runtime().unwrap();
            }
        }
        let content =
            std::fs::read_to_string(registry_file).expect("registry file must be readable");
        let mut registry = serde_yaml::from_str::<Registry>(content.as_str()).unwrap();
        // update the registry to prevent language keyword being used
        match options.language {
            Language::TypeScript => buildgen::typescript::replace_keywords(&mut registry),
            Language::Rust => buildgen::rust::replace_keywords(&mut registry),
            _ => (),
        }
        if let Language::TypeScript = options.language {
            buildgen::typescript::replace_keywords(&mut registry);
        };
        let (package_name, package_path) = match options.language {
            Language::Rust => (
                if options.aptos_version_number == "0.1.0" {
                    "aptos-types".to_string()
                } else {
                    format!("aptos-types:{}", options.aptos_version_number)
                },
                vec!["aptos-types"],
            ),
            Language::Java => ("com.aptos.types".to_string(), vec!["com", "aptos", "types"]),
            Language::Csharp => ("Aptos.Types".to_string(), vec!["Aptos", "Types"]),
            Language::Go => ("aptostypes".to_string(), vec!["aptostypes"]),
            Language::TypeScript => ("aptosTypes".to_string(), vec!["aptosTypes"]),
            Language::Swift => ("AptosTypes".to_string(), vec!["AptosTypes"]),
            _ => ("aptos_types".to_string(), vec!["aptos_types"]),
        };
        let custom_aptos_code = buildgen::read_custom_code_from_paths(
            &package_path,
            options.with_custom_aptos_code.into_iter(),
        );
        let config = serdegen::CodeGeneratorConfig::new(package_name)
            .with_encodings(vec![serdegen::Encoding::Bcs])
            .with_custom_code(custom_aptos_code);
        installer.install_module(&config, &registry).unwrap();
    }

    // Transaction builders
    let installer: Box<dyn buildgen::SourceInstaller<Error = Box<dyn std::error::Error>>> =
        match options.language {
            Language::Python3 => Box::new(buildgen::python3::Installer::new(
                install_dir,
                options.serde_package_name,
                options.package_name,
            )),
            Language::TypeScript => Box::new(buildgen::typescript::Installer::new(install_dir)),
            Language::Swift => Box::new(buildgen::swift::Installer::new(install_dir)),
            Language::Rust => Box::new(buildgen::rust::Installer::new(
                install_dir,
                options.aptos_version_number,
            )),
            Language::Cpp => Box::new(buildgen::cpp::Installer::new(install_dir)),
            Language::Java => Box::new(buildgen::java::Installer::new(install_dir)),
            Language::Csharp => Box::new(buildgen::csharp::Installer::new(install_dir)),
            Language::Go => Box::new(buildgen::golang::Installer::new(
                install_dir,
                options.serde_package_name,
                options.package_name,
            )),
        };

    if let Some(name) = options.module_name {
        installer
            .install_transaction_builders(&name, abis.as_slice())
            .unwrap();
    }
}
