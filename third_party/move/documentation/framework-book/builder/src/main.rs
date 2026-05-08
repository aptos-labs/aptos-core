// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generates the Aptos Framework Book by driving move-docgen across the
//! six framework packages.
//!
//! Two phases:
//!   1. Module pass — for each package, in dep order, run docgen with output
//!      directly into `framework-book/src/<pkg>/`. Cross-package references
//!      resolve via `doc_path` set to previously-generated subdirs.
//!   2. Summary pass — build a synthetic umbrella package depending on all
//!      six framework packages, run docgen against `SUMMARY.template.md`
//!      with the path-filtered `{{move-index PKG}}` placeholder.

use anyhow::{Context, Result};
use aptos_framework::build_model;
use move_docgen::{Docgen, DocgenOptions, IndexLinkStyle};
use move_model::metadata::LanguageVersion;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// Framework packages to document, in dep order.
/// `(subdir under aptos-move/framework, Move package name, use_latest_language)`.
/// The subdir name doubles as the path component used by `{{move-index PKG}}`.
const PACKAGES: &[(&str, &str, bool)] = &[
    ("move-stdlib", "MoveStdlib", false),
    ("aptos-stdlib", "AptosStdlib", false),
    ("aptos-framework", "AptosFramework", false),
    ("aptos-token-objects", "AptosTokenObjects", false),
    ("aptos-trading", "AptosTrading", false),
    ("aptos-experimental", "AptosExperimental", true),
];

fn main() -> Result<()> {
    let workspace_root = workspace_root();
    let book_root = workspace_root.join("third_party/move/documentation/framework-book");
    let book_src = book_root.join("src");
    let framework_dir = workspace_root.join("aptos-move/framework");

    // docgen computes link targets via simple relative-path arithmetic, so
    // both `output_directory` and entries in `doc_path` must be relative to
    // a stable CWD. CWD = `<book>/src` keeps cross-package refs short:
    // a module in `<pkg>/foo.md` references siblings as `../<other>/bar.md`.
    let original_cwd = std::env::current_dir()?;
    std::fs::create_dir_all(&book_src)?;
    std::env::set_current_dir(&book_src)?;
    let result = (|| -> Result<()> {
        for (i, (subdir, _, use_latest)) in PACKAGES.iter().enumerate() {
            let package_path = framework_dir.join(subdir);
            let output_dir = book_src.join(subdir);
            std::fs::create_dir_all(&output_dir)?;
            clear_md_files(&output_dir)?;

            let doc_path: Vec<String> = PACKAGES[..i]
                .iter()
                .map(|(s, _, _)| (*s).to_string())
                .collect();

            let references = package_path.join("doc_template/references.md");
            let references_file = references
                .exists()
                .then(|| references.to_string_lossy().into_owned());

            // Each package contributes a hand-authored landing page via
            // `doc_template/overview.md`; its `{{move-index}}` placeholder
            // expands to that package's modules during this per-package pass.
            let overview = package_path.join("doc_template/overview.md");
            let root_doc_templates = if overview.exists() {
                vec![overview.to_string_lossy().into_owned()]
            } else {
                vec![]
            };

            eprintln!("[book] generating module docs for {}", subdir);
            run_docgen(
                &package_path,
                *use_latest,
                false, // primary modules only — flagged by move-package
                DocgenOptions {
                    section_level_start: 1,
                    include_private_fun: true,
                    include_specs: true,
                    specs_inlined: false,
                    include_impl: true,
                    toc_depth: 3,
                    collapsed_sections: true,
                    output_directory: (*subdir).to_string(),
                    doc_path,
                    root_doc_templates,
                    references_file,
                    include_dep_diagrams: false,
                    include_call_diagrams: false,
                    compile_relative_to_output_dir: false,
                    output_format: None,
                    index_link_style: IndexLinkStyle::Anchored,
                    ensure_unix_paths: true,
                },
            )?;
        }

        eprintln!("[book] generating SUMMARY.md");
        build_summary(&framework_dir)?;
        Ok(())
    })();
    std::env::set_current_dir(&original_cwd).ok();
    result?;

    eprintln!("[book] done");
    Ok(())
}

fn run_docgen(
    package_path: &Path,
    use_latest_language: bool,
    all_files_as_targets: bool,
    opts: DocgenOptions,
) -> Result<()> {
    let language_version = use_latest_language.then(LanguageVersion::latest);
    let env = build_model(
        false, // dev_mode
        false, // test_mode
        false, // verify_mode
        package_path,
        BTreeMap::new(),
        None, // target_filter
        None, // bytecode_version
        None, // compiler_version
        language_version,
        true, // skip_attribute_checks
        BTreeSet::new(),
        vec![],
        true, // with_bytecode
        all_files_as_targets,
    )
    .with_context(|| format!("failed to build model for {}", package_path.display()))?;

    if env.has_errors() {
        anyhow::bail!(
            "compilation errors building model for {}",
            package_path.display()
        );
    }

    let docgen = Docgen::new(&env, &opts);
    for (file_name, content) in docgen.r#gen() {
        let dest = PathBuf::from(file_name);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, content).with_context(|| format!("writing `{}`", dest.display()))?;
    }
    Ok(())
}

fn build_summary(framework_dir: &Path) -> Result<()> {
    let tmp = tempfile::TempDir::new()?;
    let pkg_path = tmp.path().join("umbrella");
    std::fs::create_dir_all(&pkg_path)?;
    std::fs::create_dir_all(pkg_path.join("sources"))?;
    let pkg_path = pkg_path.as_path();
    let scratch_out = tmp.path().join("out");
    std::fs::create_dir_all(&scratch_out)?;

    let deps_block: String = PACKAGES
        .iter()
        .map(|(subdir, name, _)| {
            format!(
                "{name} = {{ local = {path:?} }}\n",
                name = name,
                path = framework_dir.join(subdir).to_string_lossy()
            )
        })
        .collect();

    let manifest = format!(
        "[package]\n\
         name = \"AptosFrameworkBook\"\n\
         version = \"0.0.0\"\n\
         \n\
         [addresses]\n\
         std = \"0x1\"\n\
         aptos_std = \"0x1\"\n\
         aptos_framework = \"0x1\"\n\
         aptos_fungible_asset = \"0xA\"\n\
         aptos_token = \"0x3\"\n\
         aptos_token_objects = \"0x4\"\n\
         aptos_trading = \"0x5\"\n\
         aptos_experimental = \"0x7\"\n\
         core_resources = \"0xA550C18\"\n\
         vm = \"0x0\"\n\
         vm_reserved = \"0x0\"\n\
         Extensions = \"0x1\"\n\
         \n\
         [dependencies]\n\
         {deps_block}",
    );
    std::fs::write(pkg_path.join("Move.toml"), manifest)?;

    // CWD = `<book>/src` here; the template lives one level up from where
    // the SUMMARY pass writes its scratch output, so use a path that's valid
    // wherever docgen reads it from.
    let template = "SUMMARY_template.md";
    if !Path::new(template).exists() {
        anyhow::bail!("expected template at <book>/src/{}", template);
    }

    // The SUMMARY pass loads every framework module as a primary target so all
    // of them are visible to `{{move-index PKG}}`. As a side effect docgen also
    // emits a `.md` for each module — we don't want those flat files inside
    // `src/`. Send them to an absolute scratch directory and copy back only
    // `SUMMARY.md`. doc_path doesn't matter here: `{{move-index PKG}}` builds
    // its links from the placeholder argument, not from `info.target_file`.
    let scratch_out_str = scratch_out.to_string_lossy().into_owned();
    run_docgen(
        pkg_path,
        true, // umbrella depends on aptos-experimental → use latest language
        true, // load every dep source file as a primary target so all modules
        // appear in the model and can be enumerated by `{{move-index PKG}}`
        DocgenOptions {
            section_level_start: 1,
            include_private_fun: false,
            include_specs: false,
            specs_inlined: false,
            include_impl: false,
            toc_depth: 3,
            collapsed_sections: false,
            output_directory: scratch_out_str,
            doc_path: vec![],
            root_doc_templates: vec![template.to_string()],
            references_file: None,
            include_dep_diagrams: false,
            include_call_diagrams: false,
            compile_relative_to_output_dir: false,
            output_format: None,
            // Multi-document book layout: emit `[mod](pkg/mod.md)` links.
            index_link_style: IndexLinkStyle::Plain,
            ensure_unix_paths: true,
        },
    )?;

    let scratch_summary = scratch_out.join("SUMMARY.md");
    let dest_summary = Path::new("SUMMARY.md");
    std::fs::copy(&scratch_summary, dest_summary).with_context(|| {
        format!(
            "copying {} → {}",
            scratch_summary.display(),
            dest_summary.display()
        )
    })?;
    Ok(())
}

/// Walk up from this crate's manifest to the workspace root.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/third_party/move/documentation/framework-book/builder
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(5)
        .expect("cannot locate workspace root from manifest dir")
        .to_path_buf()
}

/// Remove any `.md` files in `dir` so stale module docs from previous builds
/// don't linger when a module is removed upstream.
fn clear_md_files(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}
