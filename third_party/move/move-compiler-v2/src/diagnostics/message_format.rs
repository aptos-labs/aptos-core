use crate::diagnostics::{human::HumanEmitter, json::JsonEmitter, Emitter};
use clap::ValueEnum;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, PartialOrd, Eq, PartialEq, Clone, ValueEnum)]
pub enum MessageFormat {
    #[default]
    Human,
    Json,
}

impl MessageFormat {
    pub fn into_emitter(self) -> Box<dyn Emitter> {
        match self {
            MessageFormat::Human => {
                let stderr = StandardStream::stderr(ColorChoice::Auto);
                Box::new(HumanEmitter::new(stderr))
            },
            MessageFormat::Json => {
                let stderr = StandardStream::stderr(ColorChoice::Auto);
                Box::new(JsonEmitter::new(stderr))
            },
        }
    }
}
