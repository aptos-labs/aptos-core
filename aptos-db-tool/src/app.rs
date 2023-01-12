use crate::commands::{
    backup::{
        self,
        utils::{GlobalRestoreOpt, GlobalRestoreOptions},
    },
    backup_verified, bootstrapper, restore,
};
use anyhow::Result;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(
    name = "aptos-db-tool",
    author="hgamiui9",
    version="0.1.0",
    about="A separate dev-oriented cli tool combine all the aptos db commands.",
    long_about = None,
    propagate_version = true
)]
pub struct Tool {
    #[clap(value_parser)]
    name: Option<String>,

    #[clap(flatten)]
    pub global: GlobalRestoreOpt,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(about = "Ledger backup tool.")]
    Backup {
        #[clap(subcommand)]
        command: backup::commands::Backup,
    },

    #[clap(about = "Verify aptos backup succeed or failed.")]
    BackupVerified {
        #[clap(flatten)]
        command: backup_verified::commands::BackupVerified,
    },

    #[clap(about = "Bootstrap AptosDB from a backup.")]
    Bootstrapper {
        #[clap(flatten)]
        command: bootstrapper::commands::Bootstrapper,
    },

    #[clap(about = "Restore the db from the backup files tool.")]
    Restore {
        #[clap(subcommand)]
        command: restore::commands::Restore,
    },
}

impl Tool {
    pub async fn process(self, global_opt: GlobalRestoreOptions) -> Result<()> {
        match &self.command {
            Commands::Backup { command } => command.process().await,
            Commands::Restore { command } => command.process(global_opt).await,
            Commands::Bootstrapper { command } => command.process().await,
            Commands::BackupVerified { command } => command.process().await,
        }
    }
}
