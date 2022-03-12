// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{cargo::Cargo, context::XContext, Result};
use anyhow::bail;
use log::info;
use structopt::{clap::arg_enum, StructOpt};

#[derive(Debug, StructOpt)]
pub struct Args {
    /// Mode to run in
    #[structopt(long, case_insensitive = true, possible_values = &WorkspaceHackMode::variants(), default_value = "write")]
    mode: WorkspaceHackMode,
}

arg_enum! {
    #[derive(Clone, Copy, Debug)]
    /// Valid modes for generate-workspace-hack
    pub enum WorkspaceHackMode {
        Write,
        Diff,
        Check,
        Verify,
        Disable,
    }
}

pub fn run(args: Args, xctx: XContext) -> Result<()> {
    let hakari_builder = xctx.core().hakari_builder()?;
    let &hakari_package = hakari_builder
        .hakari_package()
        .expect("hakari package specified by builder");

    let update_cargo_lock = match args.mode {
        WorkspaceHackMode::Verify => {
            match hakari_builder.verify() {
                Ok(()) => {
                    info!("workspace-hack is valid");
                }
                Err(errors) => {
                    println!("{}", errors.display());
                    bail!("workspace-hack doesn't unify everything successfully");
                }
            }

            false
        }
        WorkspaceHackMode::Disable => {
            let existing_toml = hakari_builder
                .read_toml()
                .expect("hakari package specified by builder")?;
            let disabled_msg = "\n\
            # Disabled through `cargo x generate-workspace-hack --mode disable`.\n\
            # To re-enable, `run cargo x generate-workspace-hack`.\n\
            \n";
            existing_toml.write_to_file(disabled_msg)?
        }
        _other => {
            let hakari = hakari_builder.compute();
            let existing_toml = hakari
                .read_toml()
                .expect("hakari package specified by builder")?;
            let new_toml = hakari.to_toml_string(&xctx.core().hakari_output_options())?;

            match args.mode {
                WorkspaceHackMode::Write => {
                    // Write out the contents to the TOML file.
                    existing_toml.write_to_file(&new_toml)?
                }
                WorkspaceHackMode::Diff => {
                    let patch = existing_toml.diff_toml(&new_toml);
                    // TODO: add global coloring options to x
                    let formatter = hakari::diffy::PatchFormatter::new().with_color();
                    let diff = formatter.fmt_patch(&patch);
                    println!("{}", diff);
                    false
                }
                WorkspaceHackMode::Check => {
                    if existing_toml.is_changed(&new_toml) {
                        bail!("existing TOML is different from generated version (run with --mode diff for diff)");
                    }
                    false
                }
                WorkspaceHackMode::Disable | WorkspaceHackMode::Verify => {
                    unreachable!("already processed in outer match")
                }
            }
        }
    };

    // Update Cargo.lock if the file on disk changed.
    if update_cargo_lock {
        info!("Workspace hack contents changed, updating Cargo.lock");
        let mut cmd = Cargo::new(xctx.config().cargo_config(), "update", true);
        cmd.args(&["--package", hakari_package.name()]);
        cmd.run()?;
    }

    Ok(())
}
