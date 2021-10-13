// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use diem_config::config::DEFAULT_PORT;
use std::{path::Path, process::Command};

pub fn handle(project_path: &Path) -> Result<()> {
    let _config = shared::read_config(project_path)?;
    shared::generate_typescript_libraries(project_path)?;
    let tests_path_string = project_path
        .join("e2e")
        .as_path()
        .to_string_lossy()
        .to_string();

    Command::new("deno")
        .args([
            "test",
            "--unstable",
            tests_path_string.as_str(),
            "--allow-env=PROJECT_PATH",
            "--allow-read",
            format!("--allow-net=:{}", DEFAULT_PORT).as_str(),
        ])
        .env("PROJECT_PATH", project_path.to_string_lossy().to_string())
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}
