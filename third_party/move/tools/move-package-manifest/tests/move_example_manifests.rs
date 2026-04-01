// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::anyhow;
use std::{env, fs, path::Path};

datatest_stable::harness!(
    parse_move_example_manifests,
    {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../aptos-move/move-examples")
            .display()
            .to_string()
    },
    r"\bMove\.toml$"
);

fn parse_move_example_manifests(path: &Path) -> datatest_stable::Result<()> {
    let path = fs::canonicalize(path)?;
    let content = std::fs::read_to_string(&path)?;

    let parse_result = move_package_manifest::parse_package_manifest(&content);

    match parse_result {
        Ok(_parsed_manifest) => (),
        Err(err) => {
            let mut output = String::new();
            move_package_manifest::render_error(&mut output, &content, &err)?;

            return Err(anyhow!("Failed to parse {}\n\n{}", path.display(), output).into());
        },
    }

    Ok(())
}
