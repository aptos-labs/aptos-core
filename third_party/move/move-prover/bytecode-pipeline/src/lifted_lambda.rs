// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Name-based identification of compiler-lifted lambda functions. Mirrors
//! `LIFTED_FUN_MARKER` from `move-compiler-v2/src/env_pipeline/lambda_lifter.rs`
//! (this crate does not depend on `move-compiler-v2`, so the marker cannot be
//! imported); keep the two in sync.

use move_model::model::FunctionEnv;

/// Marker embedded in the names of compiler-lifted lambda functions
/// (`__lambda__<n>__<parent>`).
pub const LIFTED_FUN_MARKER: &str = "__lambda__";

/// Whether `fun_env` is a compiler-lifted lambda, identified by name.
pub fn is_lifted_lambda(fun_env: &FunctionEnv) -> bool {
    fun_env
        .symbol_pool()
        .string(fun_env.get_name())
        .contains(LIFTED_FUN_MARKER)
}

/// The enclosing function's simple name for a lifted lambda named
/// `__lambda__<n>__<parent>`, or [None] if `fun_env` is not one.
pub fn lifted_lambda_parent_name(fun_env: &FunctionEnv) -> Option<String> {
    let pool = fun_env.symbol_pool();
    let name = pool.string(fun_env.get_name());
    let rest = name.strip_prefix(LIFTED_FUN_MARKER)?;
    let pos = rest.find("__")?;
    Some(rest[pos + 2..].to_string())
}
