// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::UserContext,
    shared,
    shared::{Network, LATEST_USERNAME, TEST_USERNAME},
};
use anyhow::Result;
use diem_types::account_address::AccountAddress;
use std::{path::Path, process::Command};

/// Launches a Deno REPL for the shuffle project, generating transaction
/// builders and loading them into the REPL namespace for easy on chain interaction.
pub fn handle(
    home: &shared::Home,
    project_path: &Path,
    network: Network,
    key_path: &Path,
    sender_address: AccountAddress,
) -> Result<()> {
    shared::codegen_typescript_libraries(project_path, &sender_address)?;
    let deno_bootstrap = format!(
        r#"import * as context from "{context}";
        import * as devapi from "{devapi}";
        import * as helpers from "{helpers}";
        import * as main from "{main}";
        import * as codegen from "{codegen}";
        import * as help from "{repl_help}";
        import * as move from "{mv}";"#,
        context = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("context.ts")
            .to_string_lossy(),
        devapi = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("devapi.ts")
            .to_string_lossy(),
        helpers = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("helpers.ts")
            .to_string_lossy(),
        main = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("mod.ts")
            .to_string_lossy(),
        codegen = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("generated/diemTypes/mod.ts")
            .to_string_lossy(),
        repl_help = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("repl_help.ts")
            .to_string_lossy(),
        mv = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("move.ts")
            .to_string_lossy()
    );

    let network_home = home.new_network_home(&network.get_name());

    let latest_user = UserContext::new(LATEST_USERNAME, sender_address, key_path);
    let test_user = network_home.user_context_for(TEST_USERNAME)?;
    let filtered_envs = shared::get_filtered_envs_for_deno(
        home,
        project_path,
        &network,
        &[&latest_user, &test_user],
    )?;
    Command::new("deno")
        .args(["repl", "--unstable", "--eval", deno_bootstrap.as_str()])
        .envs(&filtered_envs)
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}
