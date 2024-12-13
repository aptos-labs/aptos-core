// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::TestDetails;
use anyhow::{bail, Result};
use std::{
    fmt::{Display, Formatter},
    io::{self, Write as _},
};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(Debug, Clone)]
pub enum TestResult {
    Ok,
    FailedWithMsg(String),
}

impl Display for TestResult {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TestResult::Ok => write!(f, "Test Ok"),
            TestResult::FailedWithMsg(msg) => write!(f, "Test Failed: {}", msg),
        }
    }
}

pub trait TestObserver {
    fn name(&self) -> String;
    fn handle_result(&self, details: &TestDetails, result: &TestResult) -> Result<()>;
    fn finish(&self) -> Result<()>;
}

pub struct TestSummary {
    stdout: StandardStream,
    total: usize,
    filtered_out: usize,
    passed: usize,
    failed: Vec<String>,
    observers: Vec<Box<dyn TestObserver>>,
}

impl TestSummary {
    pub fn new(total: usize, filtered_out: usize) -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
            total,
            filtered_out,
            passed: 0,
            failed: Vec::new(),
            observers: Vec::new(),
        }
    }

    pub fn add_observer(&mut self, observer: Box<dyn TestObserver>) {
        self.observers.push(observer);
    }

    pub fn handle_result(&mut self, details: TestDetails, result: TestResult) -> Result<()> {
        write!(self.stdout, "test {} ... ", details.name())?;
        match result.clone() {
            TestResult::Ok => {
                self.passed += 1;
                self.write_ok()?;
            },
            TestResult::FailedWithMsg(msg) => {
                self.failed.push(details.name());
                self.write_failed()?;
                writeln!(self.stdout)?;

                write!(self.stdout, "Error: {}", msg)?;
            },
        }
        writeln!(self.stdout)?;
        let mut errors = vec![];
        for observer in &self.observers {
            let result = observer.handle_result(&details, &result);
            if let Err(e) = result {
                errors.push(format!("{}: {}", observer.name(), e));
            }
        }
        if !errors.is_empty() {
            bail!("Failed to handle_result in observers: {:?}", errors);
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<()> {
        let mut errors = vec![];
        for observer in &self.observers {
            let result = observer.finish();
            if let Err(e) = result {
                errors.push(format!("{}: {}", observer.name(), e));
            }
        }
        if !errors.is_empty() {
            bail!("Failed to finish observers: {:?}", errors);
        }
        Ok(())
    }

    fn write_ok(&mut self) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(self.stdout, "ok")?;
        self.stdout.reset()?;
        Ok(())
    }

    fn write_failed(&mut self) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        write!(self.stdout, "FAILED")?;
        self.stdout.reset()?;
        Ok(())
    }

    pub fn write_starting_msg(&mut self) -> io::Result<()> {
        writeln!(self.stdout)?;
        writeln!(
            self.stdout,
            "running {} tests",
            self.total - self.filtered_out
        )?;
        Ok(())
    }

    pub fn write_summary(&mut self) -> io::Result<()> {
        // Print out the failing tests
        if !self.failed.is_empty() {
            writeln!(self.stdout)?;
            writeln!(self.stdout, "failures:")?;
            for name in &self.failed {
                writeln!(self.stdout, "    {}", name)?;
            }
        }

        writeln!(self.stdout)?;
        write!(self.stdout, "test result: ")?;
        if self.failed.is_empty() {
            self.write_ok()?;
        } else {
            self.write_failed()?;
        }
        writeln!(
            self.stdout,
            ". {} passed; {} failed; {} filtered out",
            self.passed,
            self.failed.len(),
            self.filtered_out
        )?;
        writeln!(self.stdout)?;
        Ok(())
    }

    pub fn success(&self) -> bool {
        self.failed.is_empty()
    }
}
