use serde::{Deserialize, Serialize};

/// Move mutator options
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// The paths to the Move sources.
    pub move_sources: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            move_sources: vec![],
        }
    }
}


/// Runs the Move mutator tool.
/// Entry point for the Move mutator tool both for the CLI and the Rust API.
pub fn run_move_mutator(options: Options) -> anyhow::Result<()> {
    println!(
        "Executed move-mutator with the following options: {:?}",
        options
    );

    Ok(())
}
