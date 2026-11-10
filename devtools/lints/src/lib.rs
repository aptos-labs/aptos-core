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
mod raw_memory_access;
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
        raw_memory_access::RAW_MEMORY_ACCESS,
        unsafe_cell_reborrow::UNSAFE_CELL_REBORROW,
    ]);
    let hash_map_scope = config.no_std_hash_map.clone();
    lint_store.register_late_lint_pass(Box::new(move |_| {
        Box::new(no_std_hash_map::NoStdHashMap::new(hash_map_scope.clone()))
    }));
    let raw_scope = config.raw_memory_access.clone();
    lint_store.register_late_lint_pass(Box::new(move |_| {
        Box::new(raw_memory_access::RawMemoryAccess::new(raw_scope.clone()))
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
/// file; the one piece of config they need is the trait lists of
/// `unsafe_cell_reborrow`, naming traits in `ui/unsafe_cell_reborrow.rs`.
#[test]
fn ui() {
    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .dylint_toml(
            r#"
            [aptos_lints.unsafe_cell_reborrow]
            inert_traits = ["unsafe_cell_reborrow::Inert"]
            context_traits = ["unsafe_cell_reborrow::CtxTrait"]
            "#,
        )
        .run();
}
