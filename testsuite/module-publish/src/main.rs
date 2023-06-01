// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_framework::{BuildOptions, BuiltPackage};
use move_binary_format::CompiledModule;
use std::{fs, fs::File, io::Write, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "module-publish",
    about = "Write Move packages binaries in a Rust file (raw_module_data.rs). Defaults to \n\
         aptos-core/crates/transaction-emitter-lib/src/transaction_generator/publishing/raw_module_data.rs"
)]
struct Args {
    #[structopt(long, help = "Optional output directory for raw_module_data.rs")]
    out_dir: Option<String>,
}

// List of additional packages (beyond those in testsuite/module-publish/src/packages) to include
fn additional_packages() -> Vec<(&'static str, &'static str)> {
    // Pairs of (package_name, package_path)
    vec![(
        "ambassador_token",
        "../../aptos-move/move-examples/token_objects/ambassador",
    )]
}

// Run "cargo run -p module-publish" to generate the file `raw_module_data.rs`.

// This file updates `raw_module_data.rs` in
// `crates/transaction-emitter-lib/src/transaction_generator/publishing/` by default,
// or in a provided directory.
// That file contains `Lazy` static variables for the binary of all the packages in
// `testsuite/simple/src/packages` as `Lazy`.
// In `crates/transaction-emitter-lib/src/transaction_generator/publishing` you should
// also find the files that can load, manipulate and use the modules.
// Typically those modules will be altered (publishing at different addresses requires a module
// address rewriting, versioning may benefit from real changes), published and used in transaction.
// Code to conveniently do that should be in that crate.
//
// All of that considered, please be careful when changing this file or the modules in
// `testsuite/simple/src/packages` given that it will likely require
// changes in `crates/transaction-emitter-lib/src/transaction_generator/publishing`.
fn main() -> Result<()> {
    let args = Args::from_args();

    // build GenericModule
    let provided_dir = match &args.out_dir {
        None => env!("CARGO_MANIFEST_DIR"),
        Some(str) => str,
    };
    println!("Building GenericModule in {}", provided_dir);
    let base_dir = std::path::Path::new(provided_dir);
    // this is gotta be the most brittle solution ever!
    // If directory structure changes this breaks.
    // However it is a test that is ignored and runs only with the intent of creating files
    // for the modules compiled, so people can change it as they wish and need to.
    let base_path = base_dir.join("../../crates/transaction-generator-lib/src/publishing/");
    let mut generic_mod = std::fs::File::create(base_path.join("raw_module_data.rs")).unwrap();

    //
    // File header
    //
    writeln!(
        generic_mod,
        r#"// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0"#
    )
    .expect("Writing header comment failed");

    //
    // Module comment
    //
    writeln!(
        generic_mod,
        r#"
// This file was generated. Do not modify!
//
// To update this code, run `cargo run -p module-publish` in aptos core.
// That test compiles the set of modules defined in
// `testsuite/simple/src/simple/sources/`
// and it writes the binaries here.
// The module name (prefixed with `MODULE_`) is a `Lazy` instance that returns the
// byte array of the module binary.
// This crate should also provide a Rust file that allows proper manipulation of each
// module defined below."#
    )
    .expect("Writing header comment failed");

    //
    // use ... directives
    //
    writeln!(
        generic_mod,
        r#"
use once_cell::sync::Lazy;
use std::collections::HashMap;"#,
    )
    .expect("Use directive failed");

    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = base_dir.join("src/packages");
    let paths: fs::ReadDir = fs::read_dir(path).unwrap();

    let mut packages = Vec::new();
    for path in paths {
        let dir = path.unwrap();

        // Skip if Move.toml doesn't exist, as it is not a move source folder.
        if !dir.path().join("Move.toml").exists() {
            continue;
        }

        let file_name = dir.file_name();

        // write out package metadata
        writeln!(generic_mod).expect("Empty line failed");
        packages.push(write_package(
            &mut generic_mod,
            dir.path(),
            file_name.to_str().unwrap(),
        ));
    }

    for (package_name, additional_package) in additional_packages() {
        packages.push(write_package(
            &mut generic_mod,
            base_dir.join(additional_package),
            package_name,
        ));
    }

    write_accessors(&mut generic_mod, packages);

    Ok(())
}

// Write out given package
fn write_package(file: &mut File, package_path: PathBuf, package_name: &str) -> String {
    println!("Building package {}", package_name);
    // build package
    let package = BuiltPackage::build(package_path, BuildOptions::default())
        .expect("building package must succeed");
    let code = package.extract_code();
    let package_metadata = package.extract_metadata().expect("Metadata must exist");
    let metadata = bcs::to_bytes(&package_metadata).expect("Metadata must serialize");

    // write out package metadata
    write_lazy(
        file,
        format!("PACKAGE_{}_METADATA", package_name.to_uppercase()).as_str(),
        &metadata,
    );

    let mut module_names = Vec::new();

    // write out all modules
    for module in &code {
        // this is an unfortunate way to find the module name but it is not
        // clear how to do it otherwise
        let compiled_module = CompiledModule::deserialize(module).expect("Module must deserialize");
        let module_name = compiled_module.self_id().name().to_owned().into_string();
        // start Lazy declaration
        let name: String = format!(
            "MODULE_{}_{}",
            package_name.to_uppercase(),
            module_name.to_uppercase()
        );
        writeln!(file).expect("Empty line failed");
        write_lazy(file, name.as_str(), module);
        module_names.push(name);
    }

    writeln!(file).expect("Empty line failed");
    writeln!(file, "#[rustfmt::skip]").expect("rustfmt skip failed");
    writeln!(
        file,
        "pub static MODULES_{}: Lazy<Vec<Vec<u8>>> = Lazy::new(|| {{ vec![",
        package_name.to_uppercase(),
    )
    .expect("Lazy MODULES declaration failed");

    for module_name in module_names {
        writeln!(file, "\t{}.to_vec(),", module_name).expect("Module name declaration failed");
    }

    writeln!(file, "]}});").expect("Lazy declaration closing } failed");
    package_name.to_string()
}

fn write_accessors(file: &mut File, packages: Vec<String>) {
    writeln!(file).expect("Empty line failed");
    writeln!(file, "#[rustfmt::skip]").expect("rustfmt skip failed");
    writeln!(
        file,
        "pub static PACKAGE_TO_METADATA: Lazy<HashMap<String, Vec<u8>>> = Lazy::new(|| {{ HashMap::from([",
    )
    .expect("Lazy PACKAGE_TO_METADATA declaration failed");

    for package in &packages {
        writeln!(
            file,
            "\t(\"{}\".to_string(), PACKAGE_{}_METADATA.to_vec()),",
            package,
            package.to_uppercase()
        )
        .expect("PACKAGE_TO_METADATA declaration failed");
    }
    writeln!(file, "])}});").expect("Lazy declaration closing } failed");

    writeln!(file).expect("Empty line failed");
    writeln!(file, "#[rustfmt::skip]").expect("rustfmt skip failed");
    writeln!(
        file,
        "pub static PACKAGE_TO_MODULES: Lazy<HashMap<String, Vec<Vec<u8>>>> = Lazy::new(|| {{ HashMap::from([",
    )
    .expect("Lazy PACKAGE_TO_MODULES declaration failed");

    for package in &packages {
        writeln!(
            file,
            "\t(\"{}\".to_string(), MODULES_{}.to_vec()),",
            package,
            package.to_uppercase()
        )
        .expect("PACKAGE_TO_MODULES declaration failed");
    }
    writeln!(file, "])}});").expect("Lazy declaration closing } failed");
}

// Write out a `Lazy` declaration
fn write_lazy(file: &mut File, data_name: &str, data: &[u8]) {
    writeln!(file, "#[rustfmt::skip]").expect("rustfmt skip failed");
    writeln!(
        file,
        "pub static {}: Lazy<Vec<u8>> = Lazy::new(|| {{",
        data_name,
    )
    .expect("Lazy declaration failed");
    write_vector(file, data);
    writeln!(file, "}});").expect("Lazy declaration closing } failed");
}

// count of elements on a single line
const DATA_BREAK_UP: usize = 18;

// Write out a vector of bytes
fn write_vector(file: &mut File, data: &[u8]) {
    writeln!(file, "\tvec![").expect("Vector header failed");
    write!(file, "\t\t").expect("Tab write failed");
    let mut newline = false;
    for (idx, datum) in data.iter().enumerate() {
        if (idx + 1) % DATA_BREAK_UP == 0 {
            writeln!(file, "{},", datum).expect("Vector write failed");
            write!(file, "\t\t").expect("Tab write failed");
            newline = true;
        } else {
            if idx == data.len() - 1 {
                write!(file, "{},", datum).expect("Vector write failed");
            } else {
                write!(file, "{}, ", datum).expect("Vector write failed");
            }
            newline = false;
        }
    }
    if !newline {
        writeln!(file).expect("Empty writeln failed");
    }
    writeln!(file, "\t]").expect("Vector footer failed");
}
