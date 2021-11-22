// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use diem_types::account_address::AccountAddress;
use std::path::Path;

pub fn handle(project_path: &Path, sender_address: AccountAddress) -> Result<()> {
    shared::codegen_typescript_libraries(project_path, &sender_address)?;
    println!(
        "Completed Move compilation and Typescript generation: {}",
        project_path.display()
    );
    Ok(())
}
