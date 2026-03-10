// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Context as _};
use std::{
    fs::File,
    io::Write as _,
    process::{Command, Stdio},
    sync::Once,
};

static INIT: Once = Once::new();

fn ts_init() {
    INIT.call_once(|| {
        // pnpm doesn't seem to handle being run concurrently well
        let lock_file = File::create("ts-batch-encrypt/.pnpm-install.lock").unwrap();
        lock_file.lock().unwrap();

        let child = Command::new("pnpm")
            .current_dir("ts-batch-encrypt")
            .arg("install")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn pnpm install")
            .unwrap();

        let output = child
            .wait_with_output()
            .context("failed to run pnpm install")
            .unwrap();
        if !output.status.success() {
            println!(
                "pnpm install failed. Status code: {:?}\n stdout: {}\n stderr: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
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
        stdin.write_all(input)?;
        // drop stdin to signal EOF
    }

    let output = child.wait_with_output().context("failed to run node")?;
    if !output.status.success() {
        return Err(anyhow!(
            "ts error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("typescript console:");
    println!("---");
    println!("{}", String::from_utf8_lossy(&output.stderr));
    println!("---");

    Ok(output.stdout)
}
