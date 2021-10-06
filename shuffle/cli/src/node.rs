// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_types::on_chain_config::VMPublishingOption;
use std::path::Path;

pub fn handle(project_path: &Path) -> Result<()> {
    let publishing_option = VMPublishingOption::open();
    diem_node::load_test_environment(
        Some(project_path.join("nodeconfig")),
        false,
        Some(publishing_option),
        diem_framework_releases::current_module_blobs().to_vec(),
        rand::rngs::OsRng,
    );
    Ok(())
}
