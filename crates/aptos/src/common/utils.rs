// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{CliResult, Error};
use serde::Serialize;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

/// Prompts for confirmation until a yes or no is given explicitly
/// TODO: Capture interrupts
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

/// Convert an empty response to Success
pub fn to_common_success_result(result: Result<(), Error>) -> CliResult {
    to_common_result(result.map(|()| "Success"))
}

/// For pretty printing outputs in JSON
pub fn to_common_result<T: Serialize>(result: Result<T, Error>) -> CliResult {
    let is_err = result.is_err();
    let result: ResultWrapper<T> = result.into();
    let string = serde_json::to_string_pretty(&result).unwrap();
    if is_err {
        Err(string)
    } else {
        Ok(string)
    }
}

/// A result wrapper for displaying either a correct execution result or an error.
///
/// The purpose of this is to have a pretty easy to recognize JSON output format e.g.
///
/// {
///   "Result":{
///     "encoded":{ ... }
///   }
/// }
///
/// {
///   "Error":"Failed to run command"
/// }
///
#[derive(Debug, Serialize)]
enum ResultWrapper<T> {
    Result(T),
    Error(String),
}

impl<T> From<Result<T, Error>> for ResultWrapper<T> {
    fn from(result: Result<T, Error>) -> Self {
        match result {
            Ok(inner) => ResultWrapper::Result(inner),
            Err(inner) => ResultWrapper::Error(inner.to_string()),
        }
    }
}

/// Checks if a file exists, being overridden by `--assume-yes`
pub fn check_if_file_exists(file: &Path, assume_yes: bool) -> Result<(), Error> {
    if file.exists()
        && !assume_yes
        && !prompt_yes(
            format!(
                "{:?} already exists, are you sure you want to overwrite it?",
                file.as_os_str()
            )
            .as_str(),
        )
    {
        Err(Error::AbortedError)
    } else {
        Ok(())
    }
}

/// Write a `&[u8]` to a file
pub fn write_to_file(key_file: &Path, name: &str, encoded: &[u8]) -> Result<(), Error> {
    let mut file = File::create(key_file).map_err(|e| Error::IO(name.to_string(), e))?;
    file.write_all(encoded)
        .map_err(|e| Error::IO(name.to_string(), e))
}

/// Appends a file extension to a `Path` without overwriting the original extension.
pub fn append_file_extension(
    file: &Path,
    appended_extension: &'static str,
) -> Result<PathBuf, Error> {
    let extension = file
        .extension()
        .map(|extension| extension.to_str().unwrap_or_default());
    if let Some(extension) = extension {
        Ok(file.with_extension(extension.to_owned() + "." + appended_extension))
    } else {
        Ok(file.with_extension(appended_extension))
    }
}
