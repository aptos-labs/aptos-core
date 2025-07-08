// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use tracing::info;

#[tokio::test]
async fn validate_node_aptos_db() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let archive_path = Path::new("./assets/maptos.tar.xz");
    assert!(
        archive_path.exists(),
        "Archive file {} not found",
        archive_path.display()
    );

    // Create temporary directory
    let temp_dir_old = tempfile::Builder::new().prefix("movement_db-").tempdir()?;
    let temp_dir_new = tempfile::Builder::new()
        .prefix("movement_aptos_db-")
        .tempdir()?;
    let movement_db = temp_dir_old.path();
    let movement_aptos_db = temp_dir_new.path();

    extract_tar_archive(&archive_path, movement_db)?;
    extract_tar_archive(&archive_path, movement_aptos_db)?;

    let mut movement_db = movement_db.to_path_buf();
    movement_db.push(".maptos");
    let mut movement_aptos_db = movement_aptos_db.to_path_buf();
    movement_aptos_db.push(".maptos");

    let cmd = validation_tool::checks::node::Command {
        movement_db,
        movement_aptos_db,
    };
    let node = validation_tool::ValidationTool::Node(cmd);

    node.run().await?;

    Ok(())
}

fn extract_tar_archive(archive_path: &Path, temp_dir: &Path) -> std::io::Result<()> {
    info!(
        "Extracting tarball {} to {}",
        archive_path.display(),
        temp_dir.display()
    );
    let file = std::fs::File::open(archive_path)?;
    let buf_reader = std::io::BufReader::new(file);
    let decoder = xz2::read::XzDecoder::new(buf_reader);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(temp_dir)?;

    Ok(())
}
