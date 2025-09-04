// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use log::info;
use std::{
    ffi::{OsStr, OsString},
    process::{Command, Stdio},
};

#[derive(Debug)]
pub struct Cargo {
    inner: Command,
    pass_through_args: Vec<OsString>,
}

impl Cargo {
    pub fn command<S>(command: S) -> Self
    where
        S: AsRef<OsStr>,
    {
        let mut inner = Command::new("cargo");
        inner.arg(command);
        Self {
            inner,
            pass_through_args: Vec::new(),
        }
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    pub fn pass_through<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.pass_through_args.push(arg.as_ref().to_owned());
        }
        self
    }

    pub fn run(&mut self, ignore_failed_exit_status: bool) {
        // Set up the output and arguments
        self.inner.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        if !self.pass_through_args.is_empty() {
            self.inner.arg("--").args(&self.pass_through_args);
        }

        // Log the command
        let command_to_execute = format!("{:?}", self.inner);
        info!("Executing command: {:?}", command_to_execute);

        // Execute the command
        let result = self.inner.output();

        // If the command failed, panic immediately with the error.
        // This will ensure that failures are not dropped silently.
        match result {
            Ok(output) => {
                if !ignore_failed_exit_status && !output.status.success() {
                    panic!(
                        "Command failed: {:?}. Output: {:?}",
                        command_to_execute, output
                    );
                }
            },
            Err(error) => {
                panic!(
                    "Unexpected error executing command: {:?}. Error: {:?}",
                    command_to_execute, error
                );
            },
        }
    }
}
