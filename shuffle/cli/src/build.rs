// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use std::path::Path;

pub fn handle(project_path: &Path) -> Result<()> {
    shared::generate_typescript_libraries(project_path)?;
    println!(
        "Completed Move compilation and Typescript generation: {}",
        project_path.display()
    );
    Ok(())
}
