// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_command_line_common::env::read_bool_env_var;

pub mod compiler;

pub const DEPENDENCY: &str = "dependency";
pub const DEPENDENCY_SHORT: char = 'd';

pub const SENDER: &str = "sender";
pub const SENDER_SHORT: char = 's';

pub const OUT_DIR: &str = "out-dir";
pub const OUT_DIR_SHORT: char = 'o';
pub const DEFAULT_OUTPUT_DIR: &str = "build";

pub const SHADOW: &str = "shadow";
pub const SHADOW_SHORT: char = 'S';

pub const SKIP_ATTRIBUTE_CHECKS: &str = "skip-attribute-checks";

pub const SOURCE_MAP: &str = "source-map";
pub const SOURCE_MAP_SHORT: char = 'm';

pub const TEST: &str = "test";
pub const TEST_SHORT: char = 't';

pub const VERIFY: &str = "verify";
pub const VERIFY_SHORT: char = 'v';

pub const LANGUAGE_VERSION: &str = "language-version";

/// Color flag interpreted locally in diagnostics/mod.rs.
/// (Is translated to codespan_reporting::term::termcolor::ColorChoice).
/// Choices here are `NONE`, `ANSI`, `ALWAYS`, with default to Auto.
pub const COLOR_MODE_ENV_VAR: &str = "COLOR_MODE";

pub const MOVE_COMPILED_INTERFACES_DIR: &str = "mv_interfaces";

pub const COMPILED_NAMED_ADDRESS_MAPPING: &str = "compiled-module-address-name";

// default value for compiler --debug flag (1 or true to set)
// (usually for debugging situations where compiler flags are hard to reach)
pub const MOVE_COMPILER_DEBUG_ENV_VAR: &str = "MOVE_COMPILER_DEBUG";
pub const MVC_DEBUG_ENV_VAR: &str = "MVC_DEBUG";

// Name of compiler CLI debug clap flag (in CLI, looks like --debug):
pub const DEBUG_FLAG: &str = "debug";

// default value for boolean --dump-bytecode flag (1 or true to set)
// (usually for debugging situations where compiler flags are hard to reach)
pub const MOVE_COMPILER_DUMP_ENV_VAR: &str = "MOVE_COMPILER_DUMP";
pub const MVC_DUMP_ENV_VAR: &str = "MVC_DUMP";

pub const MOVE_COMPILER_WARN_OF_DEPRECATION_USE: &str = "MOVE_COMPILER_WARN_OF_DEPRECATION_USE";
pub const MOVE_COMPILER_WARN_OF_DEPRECATION_USE_FLAG: &str = "Wdeprecation";

pub const WARN_OF_DEPRECATION_USE_IN_VELOR_LIBS: &str = "WARN_OF_DEPRECATION_USE_IN_VELOR_LIBS";
pub const WARN_OF_DEPRECATION_USE_IN_VELOR_LIBS_FLAG: &str = "Wdeprecation-velor";

pub const WARN_UNUSED_FLAG: &str = "Wunused";

pub const LANG_V2_FLAG: &str = "lang_v2";

// Flag to dump a stacktrace on a compiler error, for users who like
// to keep RUST_BACKTRACE always enabled.
pub const MOVE_COMPILER_BACKTRACE_ENV_VAR: &str = "MOVE_COMPILER_BACKTRACE";
pub const MVC_BACKTRACE_ENV_VAR: &str = "MVC_BACKTRACE";

pub fn get_move_compiler_backtrace_from_env() -> bool {
    read_bool_env_var(MOVE_COMPILER_BACKTRACE_ENV_VAR) || read_bool_env_var(MVC_BACKTRACE_ENV_VAR)
}
