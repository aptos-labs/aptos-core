// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint check for `friend` or `package` entry functions.
//!
//! When a function is marked `entry`, it becomes callable by anyone via a transaction,
//! regardless of its visibility modifier. A `friend entry` or
//! `package entry` function therefore does NOT restrict callers to friends
//! or the same package - the `entry` modifier overrides that restriction.

use move_binary_format::file_format::Visibility;
use move_compiler_v2::external_checks::FunctionChecker;
use move_model::model::FunctionEnv;

const CHECKER_NAME: &str = "unsafe_friend_package_entry";

#[derive(Default)]
pub struct UnsafeFriendPackageEntry;

impl FunctionChecker for UnsafeFriendPackageEntry {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_function(&self, func: &FunctionEnv) {
        if !func.is_entry() || func.visibility() != Visibility::Friend {
            return;
        }

        let visibility = if func.has_package_visibility() {
            "package"
        } else {
            "friend"
        };

        let name = func.get_name_str();
        let msg = format!(
            "`{name}` is callable by anyone. \
             The `entry` modifier allows direct invocation via transactions, \
             bypassing the `{visibility}` visibility restriction.",
        );

        self.report(func.module_env.env, &func.get_id_loc(), &msg);
    }
}
