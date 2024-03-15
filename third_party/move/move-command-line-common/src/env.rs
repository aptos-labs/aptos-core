// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;

/// An environment variable which can be set to cause the move compiler to generate
/// file formats at a given version. Only version v5 and greater are supported.
const BYTECODE_VERSION_ENV_VAR: &str = "MOVE_BYTECODE_VERSION";

/// An environment variable interpreted by codespan-reporting package, which is not
/// used here, but is used in many dependent packages, to decide whether to use
/// color for diagnostics when AutoColor has been selected.  This can be set
/// to disable color diagnostics, such as in testing (for simpler diffs).
pub const NO_COLOR_MODE_ENV_VAR: &str = "NO_COLOR";

/// Get the bytecode version from the environment variable.
// TODO: This should be configurable via toml and command line flags. See also #129.
pub fn get_bytecode_version_from_env(from_input: Option<u32>) -> Option<u32> {
    // This allows for bytecode version to come from command line flags and
    // other input locations, falls back to bytecode version if not provided
    if from_input.is_some() {
        from_input
    } else {
        std::env::var(BYTECODE_VERSION_ENV_VAR)
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
    }
}

/// An environment variable which can be set to force use of the move-compiler-v2
/// in all contexts where the V1 compiler is currently used.
const MOVE_COMPILER_V2_ENV_VAR: &str = "MOVE_COMPILER_V2";
const MVC_V2_ENV_VAR: &str = "MVC_V2";

pub fn get_move_compiler_v2_from_env() -> bool {
    read_bool_env_var(MOVE_COMPILER_V2_ENV_VAR) || read_bool_env_var(MVC_V2_ENV_VAR)
}

/// An environment variable which can be set to cause a panic if the V1 Move compiler is run (past
/// parsing and expansion phases, which are currently used by V2) as part of another toolchain or
/// testing process.  This is useful for debugging whether V2 is being invoked properly.
const MOVE_COMPILER_BLOCK_V1_ENV_VAR: &str = "MOVE_COMPILER_BLOCK_V1";
const MVC_BLOCK_V1_ENV_VAR: &str = "MVC_BLOCK_V1";

// Make this debugging option available as a CLI flag
pub const MOVE_COMPILER_BLOCK_V1_FLAG: &str = "block-compiler-v1";

pub fn get_move_compiler_block_v1_from_env() -> bool {
    read_bool_env_var(MOVE_COMPILER_BLOCK_V1_ENV_VAR) || read_bool_env_var(MVC_BLOCK_V1_ENV_VAR)
}

pub const OVERRIDE_EXP_CACHE: &str = "OVERRIDE_EXP_CACHE";

pub fn read_env_var(v: &str) -> String {
    std::env::var(v).unwrap_or_else(|_| String::new())
}

pub fn read_bool_env_var(v: &str) -> bool {
    let val = read_env_var(v).to_lowercase();
    val.parse::<bool>() == Ok(true) || val.parse::<usize>() == Ok(1)
}

pub fn bool_to_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

pub static MOVE_HOME: Lazy<String> = Lazy::new(|| {
    std::env::var("MOVE_HOME").unwrap_or_else(|_| {
        format!(
            "{}/.move",
            dirs_next::home_dir()
                .expect("user's home directory not found")
                .to_str()
                .unwrap()
        )
    })
});
