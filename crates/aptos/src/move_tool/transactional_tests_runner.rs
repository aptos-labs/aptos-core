// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// Most of the code below comes from the crate `datatest-stable`. Because the limitation of `datatest-stable`,
/// we are not able to run transactional tests as a subcommand of the Aptos CLI. Therefore, we need to duplicate code
/// here and make minor modifications.
///
use clap::Parser;

use crate::common::types::{CliError, CliTypedResult};

use std::{
    io::{self, Write},
    num::NonZeroUsize,
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    process,
    sync::mpsc::{channel, Sender},
    thread,
};

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Run Move transactional tests
#[derive(Parser, Clone)]
pub struct TransactionalTestOpts {
    /// The filter string is tested against the name of all tests, and only those tests whose names
    /// contain the filter are run.
    #[clap(long)]
    pub filter: Option<String>,

    /// Exactly match filters rather than match by substring
    #[clap(long = "exact")]
    pub filter_exact: bool,

    /// Number of threads used for running tests in parallel
    #[clap(long, default_value = "32")]
    pub test_threads: NonZeroUsize,

    /// Output minimal information
    #[clap(long)]
    pub quiet: bool,

    /// List all tests
    #[clap(long)]
    pub list: bool,

    /// Path to contain the tests
    #[clap(long, parse(from_os_str))]
    pub root_path: PathBuf,

    /// Pattern to match the test files
    #[clap(long, default_value = r".*\.(mvir|move)$")]
    pub pattern: String,
}

/// Helper function to iterate through all the files in the given directory, skipping hidden files,
/// and return an iterator of their paths.
pub fn iterate_directory(path: &Path) -> impl Iterator<Item = PathBuf> {
    walkdir::WalkDir::new(path)
        .into_iter()
        .map(::std::result::Result::unwrap)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_str()
                    .map_or(false, |s| !s.starts_with('.')) // Skip hidden files
        })
        .map(|entry| entry.path().to_path_buf())
}

pub fn derive_test_name(root: &Path, path: &Path, test_name: &str) -> String {
    let relative = path.strip_prefix(root).unwrap_or_else(|_| {
        panic!(
            "failed to strip prefix '{}' from path '{}'",
            root.display(),
            path.display()
        )
    });
    let mut test_name = test_name.to_string();
    test_name = format!("{}::{}", test_name, relative.display());
    test_name
}

struct Test {
    testfn: Box<dyn Fn() -> Result<()> + Send>,
    name: String,
}

enum TestResult {
    Ok,
    Failed,
    FailedWithMsg(String),
}

pub(crate) fn runner(options: &TransactionalTestOpts, reqs: &[Requirements]) -> CliTypedResult<()> {
    let mut tests: Vec<Test> = reqs.iter().flat_map(|req| req.expand()).collect();
    tests.sort_by(|a, b| a.name.cmp(&b.name));

    if options.list {
        for test in &tests {
            println!("{}: test", test.name);
        }

        return Ok(());
    }

    match run_tests(options, tests) {
        Ok(true) => Ok(()),
        Ok(false) => process::exit(101),
        Err(e) => Err(CliError::UnexpectedError(format!(
            "error: io error when running tests: {:?}",
            e
        ))),
    }
}

fn run_tests(options: &TransactionalTestOpts, tests: Vec<Test>) -> io::Result<bool> {
    let total = tests.len();

    // Filter out tests
    let mut remaining = match &options.filter {
        None => tests,
        Some(filter) => tests
            .into_iter()
            .filter(|test| {
                if options.filter_exact {
                    test.name == filter[..]
                } else {
                    test.name.contains(&filter[..])
                }
            })
            .rev()
            .collect(),
    };

    let filtered_out = total - remaining.len();
    let mut summary = TestSummary::new(total, filtered_out);

    if !options.quiet {
        summary.write_starting_msg()?;
    }

    let (tx, rx) = channel();

    let mut pending = 0;
    while pending > 0 || !remaining.is_empty() {
        while pending < options.test_threads.get() && !remaining.is_empty() {
            let test = remaining.pop().unwrap();
            run_test(test, tx.clone());
            pending += 1;
        }

        let (name, result) = rx.recv().unwrap();
        summary.handle_result(name, result)?;

        pending -= 1;
    }

    // Write Test Summary
    if !options.quiet {
        summary.write_summary()?;
    }

    Ok(summary.success())
}

fn run_test(test: Test, channel: Sender<(String, TestResult)>) {
    let Test { name, testfn } = test;

    let cfg = thread::Builder::new().name(name.clone());
    cfg.spawn(move || {
        let result = match catch_unwind(AssertUnwindSafe(testfn)) {
            Ok(Ok(())) => TestResult::Ok,
            Ok(Err(e)) => TestResult::FailedWithMsg(format!("{:?}", e)),
            Err(_) => TestResult::Failed,
        };

        channel.send((name, result)).unwrap();
    })
    .unwrap();
}

struct TestSummary {
    stdout: StandardStream,
    total: usize,
    filtered_out: usize,
    passed: usize,
    failed: Vec<String>,
}

impl TestSummary {
    fn new(total: usize, filtered_out: usize) -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
            total,
            filtered_out,
            passed: 0,
            failed: Vec::new(),
        }
    }

    fn handle_result(&mut self, name: String, result: TestResult) -> io::Result<()> {
        write!(self.stdout, "test {} ... ", name)?;
        match result {
            TestResult::Ok => {
                self.passed += 1;
                self.write_ok()?;
            }
            TestResult::Failed => {
                self.failed.push(name);
                self.write_failed()?;
            }
            TestResult::FailedWithMsg(msg) => {
                self.failed.push(name);
                self.write_failed()?;
                writeln!(self.stdout)?;

                write!(self.stdout, "Error: {}", msg)?;
            }
        }
        writeln!(self.stdout)?;
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

    fn write_starting_msg(&mut self) -> io::Result<()> {
        writeln!(self.stdout)?;
        writeln!(
            self.stdout,
            "running {} tests",
            self.total - self.filtered_out
        )?;
        Ok(())
    }

    fn write_summary(&mut self) -> io::Result<()> {
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

    fn success(&self) -> bool {
        self.failed.is_empty()
    }
}

#[doc(hidden)]
pub struct Requirements {
    test: fn(&Path) -> Result<()>,
    test_name: String,
    root: String,
    pattern: String,
}

impl Requirements {
    #[doc(hidden)]
    pub fn new(
        test: fn(&Path) -> Result<()>,
        test_name: String,
        root: String,
        pattern: String,
    ) -> Self {
        Self {
            test,
            test_name,
            root,
            pattern,
        }
    }

    /// Generate standard test descriptors ([`test::TestDescAndFn`]) from the descriptor of
    /// `#[datatest::files(..)]`.
    ///
    /// Scans all files in a given directory, finds matching ones and generates a test descriptor
    /// for each of them.
    fn expand(&self) -> Vec<Test> {
        let root = Path::new(&self.root).to_path_buf();

        let re = regex::Regex::new(&self.pattern)
            .unwrap_or_else(|_| panic!("invalid regular expression: '{}'", self.pattern));

        let tests: Vec<_> = iterate_directory(&root)
            .filter_map(|path| {
                let input_path = path.to_string_lossy();
                if re.is_match(&input_path) {
                    let testfn = self.test;
                    let name = derive_test_name(&root, &path, &self.test_name);
                    let testfn = Box::new(move || (testfn)(&path));

                    Some(Test { testfn, name })
                } else {
                    None
                }
            })
            .collect();

        // We want to avoid silent fails due to typos in regexp!
        if tests.is_empty() {
            panic!(
                "no test cases found for test '{}'. Scanned directory: '{}' with pattern '{}'",
                self.test_name, self.root, self.pattern,
            );
        }

        tests
    }
}
