// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use clap::{Command, Subcommand};
use clap_complete::{generate_to, shells::Bash, shells::Fish, shells::Zsh, shells::PowerShell};
use std::env;

include!("src/lib.rs");

fn main() -> shadow_rs::SdResult<()> {

    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };
    let cmd = Command::new("aptos");
    let mut cmd = Tool::augment_subcommands(cmd);
    let path = generate_to(
        Bash,
        &mut cmd, // We need to specify what generator to use
        "aptos",  // We need to specify the bin name manually
        outdir.clone(),   // We need to specify where to write to
    )?;

    println!("cargo:warning=completion file is generated: {:?}", path);

    let path = generate_to(
        Fish,
        &mut cmd, // We need to specify what generator to use
        "aptos",  // We need to specify the bin name manually
        outdir.clone(),   // We need to specify where to write to
    )?;

    println!("cargo:warning=completion file is generated: {:?}", path);

    let path = generate_to(
        PowerShell,
        &mut cmd, // We need to specify what generator to use
        "aptos",  // We need to specify the bin name manually
        outdir.clone(),   // We need to specify where to write to
    )?;

    println!("cargo:warning=completion file is generated: {:?}", path);

    let path = generate_to(
        Zsh,
        &mut cmd, // We need to specify what generator to use
        "aptos",  // We need to specify the bin name manually
        outdir,   // We need to specify where to write to
    )?;

    println!("cargo:warning=completion file is generated: {:?}", path);
    shadow_rs::new()
}
