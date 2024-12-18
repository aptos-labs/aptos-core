use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, PartialOrd, Eq, PartialEq, Clone, ValueEnum)]
pub enum MessageFormat {
    #[default]
    Human,
    Json,
}
