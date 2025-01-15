// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::env::read_bool_env_var;

/// Extension for raw output files
pub const OUT_EXT: &str = "out";
/// Extension for expected output files
pub const EXP_EXT: &str = "exp";

pub fn get_compiler_exp_extension() -> &'static str {
    EXP_EXT
}

/// If any of these env vars is set, the test harness should overwrite
/// the existing .exp files with the output instead of checking
/// them against the output.
pub const UPDATE_BASELINE: &str = "UPDATE_BASELINE";
pub const UPBL: &str = "UPBL";
pub const UB: &str = "UB";

pub const PRETTY: &str = "PRETTY";
pub const FILTER: &str = "FILTER";

pub fn read_env_update_baseline() -> bool {
    read_bool_env_var(UPDATE_BASELINE) || read_bool_env_var(UPBL) || read_bool_env_var(UB)
}

pub fn add_update_baseline_fix(s: impl AsRef<str>) -> String {
    format!(
        "{}\n\
        Run with `env {}=1` (or `env {}=1`) to save the current output as \
        the new expected output",
        s.as_ref(),
        UB,
        UPDATE_BASELINE
    )
}

pub fn format_diff(expected: impl AsRef<str>, actual: impl AsRef<str>) -> String {
    use difference::*;

    let changeset = Changeset::new(expected.as_ref(), actual.as_ref(), "\n");

    let mut ret = String::new();

    for seq in changeset.diffs {
        match &seq {
            Difference::Same(x) => {
                ret.push_str(x);
                ret.push('\n');
            },
            Difference::Add(x) => {
                ret.push_str("\x1B[92m");
                ret.push_str(x);
                ret.push_str("\x1B[0m");
                ret.push('\n');
            },
            Difference::Rem(x) => {
                ret.push_str("\x1B[91m");
                ret.push_str(x);
                ret.push_str("\x1B[0m");
                ret.push('\n');
            },
        }
    }
    ret
}

pub fn format_diff_no_color(expected: impl AsRef<str>, actual: impl AsRef<str>) -> String {
    use difference::*;

    let changeset = Changeset::new(expected.as_ref(), actual.as_ref(), "\n");

    let mut ret = String::new();

    for seq in changeset.diffs {
        match &seq {
            Difference::Same(x) => {
                ret.push_str(&format!("= {}", x.replace('\n', "\n= ")));
                ret.push('\n');
            },
            Difference::Add(x) => {
                ret.push_str(&format!("+ {}", x.replace('\n', "\n+ ")));
                ret.push('\n');
            },
            Difference::Rem(x) => {
                ret.push_str(&format!("- {}", x.replace('\n', "\n- ")));
                ret.push('\n');
            },
        }
    }
    ret
}
