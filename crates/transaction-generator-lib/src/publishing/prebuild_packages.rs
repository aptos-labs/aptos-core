// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_sdk::bcs;
use move_binary_format::CompiledModule;
use move_package::source_package::std_lib::StdVersion;
use std::{
    fmt::Write,
    path::{Path, PathBuf},
};

/// Get the local framework path based on this source file's location.
/// Note: If this source file is moved to a different location, this function
/// may need to be updated.
fn get_local_framework_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("aptos-move").join("framework"))
        .expect("framework path")
        .to_string_lossy()
        .to_string()
}

pub fn create_prebuilt_packages_rs_file(
    base_dir: impl AsRef<Path>,
    packages_to_build: Vec<(&str, &str, bool)>,
    output_file: impl AsRef<Path>,
    in_aptos_core: bool,
) -> anyhow::Result<()> {
    let mut string_buffer = "".to_string();
    //
    // File header
    //
    writeln!(
        string_buffer,
        r#"// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0"#
    )
    .expect("Writing header comment failed");

    //
    // Module comment
    //
    writeln!(
        string_buffer,
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
        string_buffer,
        "
use aptos_transaction_generator_lib::entry_point_trait::PreBuiltPackages;
use once_cell::sync::Lazy;
use std::collections::HashMap;",
    )
    .expect("Use directive failed");
    writeln!(string_buffer).expect("Empty line failed");

    let mut packages = Vec::new();
    for (package_name, additional_package, mut use_latest_language) in packages_to_build {
        use_latest_language = true;
        let mut build_options = if use_latest_language {
            BuildOptions::move_2().set_latest_language()
        } else {
            BuildOptions::move_2()
        };
        build_options.dev = true;
        if in_aptos_core {
            build_options.override_std = Some(StdVersion::Local(get_local_framework_path()));
        }

        packages.push(write_package(
            &mut string_buffer,
            base_dir.as_ref().join(additional_package),
            package_name,
            build_options,
        ));
    }

    write_accessors(&mut string_buffer, packages);

    writeln!(
        string_buffer,
        "
#[derive(Debug)]
pub struct PreBuiltPackagesImpl;

impl PreBuiltPackages for PreBuiltPackagesImpl {{
    fn package_metadata(&self, package_name: &str) -> &[u8] {{
        PACKAGE_TO_METADATA.get(package_name).expect(package_name)
    }}

    fn package_modules(&self, package_name: &str) -> &[Vec<u8>] {{
        PACKAGE_TO_MODULES.get(package_name).expect(package_name)
    }}

    fn package_script(&self, package_name: &str) -> Option<&Vec<u8>> {{
        PACKAGE_TO_SCRIPT.get(package_name)
    }}
}}",
    )
    .expect("PreBuiltPackages failed");

    let mut generic_mod = std::fs::File::create(output_file).unwrap();
    {
        use std::io::Write;
        write!(generic_mod, "{}", string_buffer)?;
    }
    Ok(())
}

// Write out given package
fn write_package(
    file: &mut String,
    package_path: PathBuf,
    package_name: &str,
    build_options: BuildOptions,
) -> (String, bool) {
    println!("Building package {}", package_name);
    // build package
    let package =
        BuiltPackage::build(package_path, build_options).expect("building package must succeed");
    let modules = package.extract_code();
    let mut scripts = package.extract_script_code();
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
    for module in &modules {
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

    assert!(
        scripts.len() <= 1,
        "Only single script can be added per package"
    );

    let has_script = if let Some(script) = scripts.pop() {
        let name: String = format!("SCRIPT_{}", package_name.to_uppercase());
        writeln!(file).expect("Empty line failed");
        write_lazy(file, name.as_str(), &script);
        true
    } else {
        false
    };

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
    (package_name.to_string(), has_script)
}

fn write_accessors(file: &mut String, packages: Vec<(String, bool)>) {
    writeln!(file).expect("Empty line failed");
    writeln!(file, "#[rustfmt::skip]").expect("rustfmt skip failed");
    writeln!(
        file,
        "pub static PACKAGE_TO_METADATA: Lazy<HashMap<String, Vec<u8>>> = Lazy::new(|| {{ HashMap::from([",
    )
    .expect("Lazy PACKAGE_TO_METADATA declaration failed");

    for (package, _) in &packages {
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

    for (package, _) in &packages {
        writeln!(
            file,
            "\t(\"{}\".to_string(), MODULES_{}.to_vec()),",
            package,
            package.to_uppercase()
        )
        .expect("PACKAGE_TO_MODULES declaration failed");
    }
    writeln!(file, "])}});").expect("Lazy declaration closing } failed");

    writeln!(file).expect("Empty line failed");
    writeln!(file, "#[rustfmt::skip]").expect("rustfmt skip failed");
    writeln!(
        file,
        "pub static PACKAGE_TO_SCRIPT: Lazy<HashMap<String, Vec<u8>>> = Lazy::new(|| {{ HashMap::from([",
    )
    .expect("Lazy PACKAGE_TO_SCRIPT declaration failed");

    for (package, has_script) in &packages {
        if *has_script {
            writeln!(
                file,
                "\t(\"{}\".to_string(), SCRIPT_{}.to_vec()),",
                package,
                package.to_uppercase()
            )
            .expect("PACKAGE_TO_SCRIPT declaration failed");
        }
    }
    writeln!(file, "])}});").expect("Lazy declaration closing } failed");
}

// Write out a `Lazy` declaration
fn write_lazy(file: &mut String, data_name: &str, data: &[u8]) {
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
fn write_vector(file: &mut String, data: &[u8]) {
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
