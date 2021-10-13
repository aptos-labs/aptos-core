// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;

use anyhow::Result;
use shared::get_shuffle_dir;
use std::{collections::HashMap, path::Path, process::Command};

/// Launches a Deno REPL for the shuffle project, generating transaction
/// builders and loading them into the REPL namespace for easy on chain interaction.
pub fn handle(project_path: &Path, network: Option<String>) -> Result<()> {
    let config = shared::read_config(project_path)?;
    shared::generate_typescript_libraries(project_path)?;
    let deno_bootstrap = format!(
        r#"import * as Shuffle from "{shuffle}";
        import * as main from "{main}";
        import * as codegen from "{codegen}";
        import * as DiemHelpers from "{helpers}";"#,
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
    );
    let mut filtered_envs: HashMap<String, String> = HashMap::new();
    filtered_envs.insert(
        String::from("PROJECT_PATH"),
        project_path.to_string_lossy().to_string(),
    );
    filtered_envs.insert(
        String::from("SHUFFLE_HOME"),
        get_shuffle_dir().to_string_lossy().to_string(),
    );
    let shuffle_network = match network {
        Some(network) => network,
        None => config.get_network().to_string(),
    };

    filtered_envs.insert(String::from("SHUFFLE_NETWORK"), shuffle_network);

    Command::new("deno")
        .args(["repl", "--unstable", "--eval", deno_bootstrap.as_str()])
        .envs(&filtered_envs)
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}
