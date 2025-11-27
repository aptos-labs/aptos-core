// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use anyhow::{anyhow, Context as _};
use std::{
    io::Write as _,
    process::{Command, Stdio},
};
use std::sync::Once;


static INIT: Once = Once::new();

fn ts_init() {
    INIT.call_once(|| {
        let child = Command::new("pnpm")
            .current_dir("ts-batch-encrypt")
            .arg("install")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn pnpm install").unwrap();


        let output = child.wait_with_output().context("failed to run pnpm install").unwrap();
        if !output.status.success() {
           println!("pnpm install failed with error {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

    });
}

pub(super) fn run_ts(fn_name: &str, input: &[u8]) -> anyhow::Result<Vec<u8>> {
    ts_init();
    let mut child = Command::new("pnpm")
        .current_dir("ts-batch-encrypt")
        .args(["exec", "tsx", "src/shim.ts", "--"])
        .arg(fn_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn node")?;

    {
        let mut stdin = child.stdin.take().context("no stdin")?;
        stdin.write_all(&input)?;
        // drop stdin to signal EOF
    }

    let output = child.wait_with_output().context("failed to run node")?;
    if !output.status.success() {
        return Err(anyhow!(
            "ts error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("{:?}", String::from_utf8_lossy(&output.stderr));

    Ok(output.stdout)
}
