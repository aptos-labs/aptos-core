// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Custom lints for aptos-core, built on [dylint](https://github.com/trailofbits/dylint).
//!
//! Each lint lives in its own module and is a late lint pass (full type
//! information, HIR and MIR available). Which crates a lint applies to is
//! decided by `dylint.toml` at the workspace root, not by the lint: a lint
//! whose scope does not name the crate being compiled does nothing. See
//! [`scope`].
//!
//! Running: `cargo dylint --all -- -p <crate>...` (needs `cargo-dylint` and
//! `dylint-link`; `scripts/dev_setup.sh` installs them). The crate is pinned
//! to its own nightly in `rust-toolchain.toml`; `cargo dylint upgrade
//! devtools/lints` bumps the toolchain and `clippy_utils` together.
//!
//! Suppressing a lint at a site:
//! `#[cfg_attr(dylint_lib = "aptos_lints", allow(<lint_name>))]`.

#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod no_std_hash_map;
mod scope;
mod unsafe_cell_reborrow;

use rustc_lint::LintStore;
use rustc_session::Session;

dylint_linting::dylint_library!();

#[allow(clippy::no_mangle_with_rust_abi)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &Session, lint_store: &mut LintStore) {
    dylint_linting::init_config(sess);
    let config = scope::Config::load();

    lint_store.register_lints(&[
        no_std_hash_map::NO_STD_HASH_MAP,
        unsafe_cell_reborrow::UNSAFE_CELL_REBORROW,
    ]);
    let hash_map_scope = config.no_std_hash_map.clone();
    lint_store.register_late_lint_pass(Box::new(move |_| {
        Box::new(no_std_hash_map::NoStdHashMap::new(hash_map_scope.clone()))
    }));
    let cell_config = config.unsafe_cell_reborrow.clone();
    lint_store.register_late_lint_pass(Box::new(move |_| {
        Box::new(unsafe_cell_reborrow::UnsafeCellReborrow::new(
            cell_config.clone(),
        ))
    }));
}

/// `ui/` holds UI tests in rustc's and clippy's sense (and under their
/// conventional name): each `ui/<lint>.rs` is compiled through the dylint
/// driver and the exact diagnostics it produces are diffed against
/// `ui/<lint>.stderr`. They pin what a lint flags, what it leaves alone, and
/// what it says.
///
/// The tests run with no crate scopes, so every lint applies to every test
/// file; the one piece of config they need is the struct and
/// anchor-function lists of `unsafe_cell_reborrow`, naming items in
/// `ui/unsafe_cell_reborrow.rs`.
#[test]
fn ui() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .dylint_toml(
            r#"
            [aptos_lints.unsafe_cell_reborrow]
            structs = [
                "unsafe_cell_reborrow::Ctx",
                "unsafe_cell_reborrow::Leaky",
                "unsafe_cell_reborrow::Adapter",
                "unsafe_cell_reborrow::Store",
                "unsafe_cell_reborrow::TupleCell",
                "unsafe_cell_reborrow::Nested",
                "unsafe_cell_reborrow::NoCells",
                "unsafe_cell_reborrow::Renamed",
            ]
            anchor_fns = ["unsafe_cell_reborrow::anchor_borrow"]
            "#,
        )
        .run();
}

/// A lint pass sees one crate, so it cannot tell that an entry of the
/// `structs` list names a crate that does not exist or that the lint is not
/// run on. Such an entry would switch the check off without a word; this
/// test reads the repository's `dylint.toml` and catches it.
#[test]
fn listed_structs_are_in_linted_crates() {
    #[derive(serde::Deserialize)]
    struct DylintToml {
        aptos_lints: scope::Config,
    }

    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let toml = std::fs::read_to_string(format!("{root}/dylint.toml")).expect("read dylint.toml");
    let config = toml::from_str::<DylintToml>(&toml)
        .expect("parse dylint.toml")
        .aptos_lints
        .unsafe_cell_reborrow;

    let metadata = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .expect("run cargo metadata");
    assert!(metadata.status.success(), "cargo metadata failed");
    let metadata: serde_json::Value =
        serde_json::from_slice(&metadata.stdout).expect("parse cargo metadata");
    let workspace_crates: Vec<String> = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .map(|package| package["name"].as_str().expect("name").replace('-', "_"))
        .collect();

    for listed in &config.structs {
        let krate = listed.split("::").next().expect("a path");
        assert!(
            workspace_crates.iter().any(|name| name == krate),
            "`{listed}` is listed for `unsafe_cell_reborrow`, but `{krate}` is not a crate of \
             this workspace"
        );
        assert!(
            config.scope.applies_to(krate),
            "`{listed}` is listed for `unsafe_cell_reborrow`, but the lint is not run on `{krate}`"
        );
    }
}
