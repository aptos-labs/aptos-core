// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::term::termcolor::Buffer;
#[allow(unused_imports)]
use log::debug;
use move_docgen::{Docgen, DocgenOptions};
use move_model::metadata::LanguageVersion;
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

const STDLIB_DEPENDENCY: &str = "../../move-stdlib/sources";
const NAMED_ADDRESSES: &str = "std=0x1";

const V2_TEST_DIR: &str = "test-compiler-v2";

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let is_root_template = path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
        .ends_with("_template.md");

    let mut sources = vec![];
    if !is_root_template {
        sources.push(path.to_string_lossy().to_string());
    } else {
        // add as sources all .notest_move files starting with the name of the template.
        let base_name = path.file_stem().unwrap().to_string_lossy().to_string();
        let dir_name = path.parent().unwrap();
        dir_name.read_dir().unwrap().for_each(|res| {
            let f = res.unwrap();
            let name = f.file_name().to_string_lossy().to_string();
            if name.starts_with(&base_name) && name.ends_with(".notest_move") {
                sources.push(f.path().to_string_lossy().to_string());
            }
        });
    }

    let language_version = if path.to_str().is_some_and(|s| s.contains(V2_TEST_DIR)) {
        Some(LanguageVersion::latest_stable())
    } else {
        Some(LanguageVersion::default())
    };

    let mut docgen_options = DocgenOptions::default();
    if is_root_template {
        docgen_options.root_doc_templates = vec![path.to_string_lossy().to_string()];
    }
    docgen_options.include_specs = true;
    docgen_options.include_impl = true;
    docgen_options.include_private_fun = true;

    docgen_options.specs_inlined = true;
    test_docgen(
        path,
        &sources,
        language_version,
        docgen_options.clone(),
        "spec_inline.md",
    )?;

    docgen_options.specs_inlined = false;
    test_docgen(
        path,
        &sources,
        language_version,
        docgen_options.clone(),
        "spec_separate.md",
    )?;

    docgen_options.specs_inlined = true;
    docgen_options.collapsed_sections = false;
    test_docgen(
        path,
        &sources,
        language_version,
        docgen_options,
        "spec_inline_no_fold.md",
    )?;

    Ok(())
}

fn test_docgen(
    path: &Path,
    sources: &[String],
    language_version: Option<LanguageVersion>,
    mut docgen_options: DocgenOptions,
    suffix: &str,
) -> anyhow::Result<()> {
    let mut temp_path = PathBuf::from(TempDir::new()?.path());
    docgen_options.output_directory = temp_path.to_string_lossy().to_string();
    let base_name = format!(
        "{}.md",
        path.file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("_template", "")
    );
    temp_path.push(&base_name);

    let compiler_options = move_compiler_v2::Options {
        sources: sources.to_vec(),
        dependencies: vec![STDLIB_DEPENDENCY.to_string()],
        named_address_mapping: vec![NAMED_ADDRESSES.to_string()],
        language_version,
        skip_attribute_checks: true,
        compile_verify_code: true,
        ..Default::default()
    };

    let mut error_writer = Buffer::no_color();
    let mut output =
        match move_compiler_v2::run_move_compiler_for_analysis(&mut error_writer, compiler_options)
        {
            Ok(model) => {
                let generator = Docgen::new(&model, &docgen_options);
                for (file, content) in generator.r#gen() {
                    let out_path = PathBuf::from(&file);
                    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
                    std::fs::write(out_path.as_path(), content).unwrap();
                }
                let mut contents = String::new();
                debug!("writing to {}", temp_path.display());
                let mut file = File::open(temp_path.as_path()).unwrap();
                file.read_to_string(&mut contents).unwrap();
                contents
            },
            Err(err) => format!("Move docgen returns: {}\n", err),
        };
    output += &String::from_utf8_lossy(&error_writer.into_inner());
    let baseline_path = path.with_extension(suffix);
    verify_or_update_baseline(baseline_path.as_path(), &output)?;
    Ok(())
}

datatest_stable::harness!(test_runner, "tests/sources", r".*\.move$|.*_template\.md$",);
