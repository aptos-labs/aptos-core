// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use std::{path::Path, process::Command};

pub fn handle(project_path: &Path) -> Result<()> {
    let _config = shared::read_config(project_path)?;

    // TODO: Fix hardcoding of mod.ts by copying it into project path and referencing
    // it via project_path.join(...). Remove Message pkg hardcode and iterate over pkgs.
    let deno_bootstrap = format!(
        r#"import * as Shuffle from "/Users/droc/workspace/diem/shuffle/cli/repl.ts";
        import * as TxnBuilder from "{}/Message/txn_builders/mod.ts";
    "#,
        project_path.display()
    );

    Command::new("deno")
        .args(["repl", "--unstable", "--eval", deno_bootstrap.as_str()])
        .env("PROJECT_PATH", project_path.to_string_lossy().to_string())
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}
