// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use diem_types::account_address::AccountAddress;
use std::{path::Path, process::Command};
use url::Url;

/// Launches a Deno REPL for the shuffle project, generating transaction
/// builders and loading them into the REPL namespace for easy on chain interaction.
pub fn handle(
    project_path: &Path,
    network: Url,
    key_path: &Path,
    sender_address: AccountAddress,
) -> Result<()> {
    shared::generate_typescript_libraries(project_path)?;
    let deno_bootstrap = format!(
        r#"import * as Shuffle from "{shuffle}";
        import * as main from "{main}";
        import * as codegen from "{codegen}";
        import * as DiemHelpers from "{helpers}";
        import * as help from "{repl_help}";"#,
        shuffle = project_path.join("repl.ts").to_string_lossy(),
        main = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("mod.ts")
            .to_string_lossy(),
        codegen = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("generated/diemTypes/mod.ts")
            .to_string_lossy(),
        helpers = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("helpers.ts")
            .to_string_lossy(),
        repl_help = project_path
            .join(shared::MAIN_PKG_PATH)
            .join("repl_help.ts")
            .to_string_lossy()
    );
    let filtered_envs =
        shared::get_filtered_envs_for_deno(project_path, &network, key_path, sender_address);
    Command::new("deno")
        .args(["repl", "--unstable", "--eval", deno_bootstrap.as_str()])
        .envs(&filtered_envs)
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}
