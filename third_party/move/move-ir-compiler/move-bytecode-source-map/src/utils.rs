// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::source_map::SourceMap;
use anyhow::{Context, Result};
use move_ir_types::location::Loc;
use std::{fs::File, io::Read, path::Path};

pub type Error = (Loc, String);
pub type Errors = Vec<Error>;

pub fn source_map_from_file(file_path: &Path) -> Result<SourceMap> {
    let mut bytes = Vec::new();
    File::open(file_path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .with_context(|| {
            format!(
                "Reading in source map information for file {}",
                file_path.to_string_lossy(),
            )
        })?;
    bcs::from_bytes::<SourceMap>(&bytes).with_context(|| {
        format!(
            "Deserializing source map information for file {}",
            file_path.to_string_lossy()
        )
    })
}
