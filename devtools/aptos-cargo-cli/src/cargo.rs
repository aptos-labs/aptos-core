// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use log::debug;
use std::{
    ffi::{OsStr, OsString},
    process::{Command, Stdio},
};

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

    pub fn run(&mut self) {
        self.inner.stdout(Stdio::inherit()).stderr(Stdio::inherit());

        if !self.pass_through_args.is_empty() {
            self.inner.arg("--").args(&self.pass_through_args);
        }

        debug!("Executing command: {:?}", self.inner);

        let _ = self.inner.output();
    }
}
