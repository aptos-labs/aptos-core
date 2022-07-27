// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::{CliError, CliTypedResult};
use clap::Parser;
use std::path::Path;

/// Reads a line from input
pub fn read_line(input_name: &'static str) -> CliTypedResult<String> {
    let mut input_buf = String::new();
    let _ = std::io::stdin()
        .read_line(&mut input_buf)
        .map_err(|err| CliError::IO(input_name.to_string(), err))?;

    Ok(input_buf)
}

/// Prompts for confirmation until a yes or no is given explicitly
pub fn prompt_yes(prompt: &str) -> bool {
    let mut result: Result<bool, ()> = Err(());

    // Read input until a yes or a no is given
    while result.is_err() {
        println!("{} [yes/no] >", prompt);
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        result = match input.trim().to_lowercase().as_str() {
            "yes" | "y" => Ok(true),
            "no" | "n" => Ok(false),
            _ => Err(()),
        };
    }
    result.unwrap()
}

/// Prompt yes but override with a command line option
pub fn prompt_yes_with_override(prompt: &str, prompt_options: PromptOptions) -> CliTypedResult<()> {
    if prompt_options.assume_no || (!prompt_options.assume_yes && !prompt_yes(prompt)) {
        Err(CliError::AbortedError)
    } else {
        Ok(())
    }
}

/// Checks if a file exists, being overridden by `PromptOptions`
pub fn check_if_file_exists(file: &Path, prompt_options: PromptOptions) -> CliTypedResult<()> {
    if file.exists() {
        prompt_yes_with_override(
            &format!(
                "{:?} already exists, are you sure you want to overwrite it?",
                file.as_os_str(),
            ),
            prompt_options,
        )?
    }

    Ok(())
}

/// An insertable option for use with prompts.
#[derive(Clone, Copy, Debug, Parser)]
pub struct PromptOptions {
    /// Assume yes for all yes/no prompts
    #[clap(long, group = "prompt_options")]
    pub assume_yes: bool,
    /// Assume no for all yes/no prompts
    #[clap(long, group = "prompt_options")]
    pub assume_no: bool,
}

impl PromptOptions {
    pub fn yes() -> Self {
        Self {
            assume_yes: true,
            assume_no: false,
        }
    }
}
