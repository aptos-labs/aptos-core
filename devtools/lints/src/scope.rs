// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-lint crate scoping, read from the `[aptos_lints]` table of
//! `dylint.toml` at the workspace root:
//!
//! ```toml
//! [aptos_lints.no_std_hash_map]
//! crates = ["mono-move-*"]           # globs on crate names; trailing `*` only
//! exempt = ["mono-move-testsuite"]   # subtracted from `crates`
//! ```
//!
//! A lint with no `crates` entry applies everywhere (that is what the UI
//! tests rely on); with one, it applies only to the matching crates. Scoping
//! by crate name rather than by path is what lets a lint be promoted from
//! one subsystem to the repo by editing the config.

use rustc_lint::LateContext;
use rustc_span::def_id::LOCAL_CRATE;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub no_std_hash_map: Scope,
}

impl Config {
    pub fn load() -> Self {
        dylint_linting::config_or_default(env!("CARGO_PKG_NAME"))
    }
}

/// Which crates a lint applies to.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Scope {
    pub crates: Vec<String>,
    pub exempt: Vec<String>,
}

impl Scope {
    /// Whether the lint applies to the crate being compiled. Cheap enough to
    /// call once per pass; lints cache the answer.
    pub fn applies(&self, cx: &LateContext<'_>) -> bool {
        let name = cx.tcx.crate_name(LOCAL_CRATE);
        let name = name.as_str();
        // Cargo passes crate names with `-` turned into `_`; accept either.
        let normalized = name.replace('-', "_");
        let matches = |pattern: &String| {
            let pattern = pattern.replace('-', "_");
            match pattern.strip_suffix('*') {
                Some(prefix) => normalized.starts_with(prefix),
                None => normalized == pattern,
            }
        };
        if self.exempt.iter().any(matches) {
            return false;
        }
        self.crates.is_empty() || self.crates.iter().any(matches)
    }
}
