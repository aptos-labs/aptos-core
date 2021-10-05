// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use std::{path::Path, process::Command};

/// Launches a Deno REPL for the shuffle project, generating transaction
/// builders and loading them into the REPL namespace for easy on chain interaction.
pub fn handle(project_path: &Path) -> Result<()> {
    let _config = shared::read_config(project_path)?;

    let deno_bootstrap = format!(
        r#"import * as Shuffle from "{project}/repl.ts";
        import * as TxnBuilder from "{project}/{pkg}/txn_builders/mod.ts";
        import * as Helper from "{project}/{pkg}/txn_builders/helper.ts";"#,
        project = project_path.display(),
        pkg = shared::MAIN_PKG_PATH,
    );

    Command::new("deno")
        .args(["repl", "--unstable", "--eval", deno_bootstrap.as_str()])
        .env("PROJECT_PATH", project_path.to_string_lossy().to_string())
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}
