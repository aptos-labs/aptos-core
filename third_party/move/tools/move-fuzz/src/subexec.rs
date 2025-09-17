// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Result};
use command_group::{CommandGroup, GroupChild, Signal, UnixChildExt};
use log::{error, trace};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, ExitStatus, Stdio},
    sync::{Arc, RwLock},
    thread,
    thread::JoinHandle,
};

/// Internal representation of a subcommand execution
pub struct SubExec {
    child: GroupChild,
    thread_stdout: JoinHandle<Result<()>>,
    thread_stderr: JoinHandle<Result<()>>,
}

impl SubExec {
    /// Launch the subcommand, collect stdout and stderr if requested
    pub fn run(
        mut command: Command,
        stdout: Option<Arc<RwLock<Vec<String>>>>,
        stderr: Option<Arc<RwLock<Vec<String>>>>,
    ) -> Result<Self> {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .group_spawn()?;

        let stream_stdout = child.inner().stdout.take().expect("stdout pipe");
        let stream_stderr = child.inner().stderr.take().expect("stderr pipe");

        let thread_stdout =
            thread::spawn(move || Self::log_and_maybe_capture_stream(stream_stdout, "O", stdout));
        let thread_stderr =
            thread::spawn(move || Self::log_and_maybe_capture_stream(stream_stderr, "X", stderr));

        Ok(Self {
            child,
            thread_stdout,
            thread_stderr,
        })
    }

    /// Check if the command has terminated (and if so, return the exit status)
    pub fn probe(&mut self) -> Result<Option<ExitStatus>> {
        let status = self
            .child
            .try_wait()
            .map_err(|e| anyhow!("failed to wait for process: {e}"))?;

        if status.is_some() {
            while !self.thread_stdout.is_finished() {
                // wait for stdout thread to finish
            }
            while !self.thread_stderr.is_finished() {
                // wait for stderr thread to finish
            }
        }
        Ok(status)
    }

    /// Terminate the execution, either via waiting or via signal
    fn end(self, signal: Option<Signal>) -> Result<ExitStatus> {
        let Self {
            mut child,
            thread_stdout,
            thread_stderr,
        } = self;

        let mut has_error = false;

        // terminate the child process group first
        match signal {
            None => {
                if let Err(e) = child.wait() {
                    error!("failed to wait for process: {e}");
                    has_error = true;
                }
            },
            Some(signal) => {
                if let Err(e) = child.signal(signal) {
                    error!("failed to send signal {signal} to process: {e}");
                    has_error = true;
                }
            },
        }

        // force kill the process group if runs into errors
        if has_error {
            match child.kill() {
                Ok(()) => (),
                Err(e) => panic!("unable to kill command: {e}"),
            }
        }

        // we must have obtained a status here
        let status = child.wait().expect("command terminated");

        // terminate both threads as well
        match thread_stdout.join() {
            Ok(Ok(())) => (),
            Ok(Err(e)) => {
                error!("stdout thread runs into error: {e}");
                has_error = true;
            },
            Err(e) => panic!("stdout thread panics: {e:?}"),
        }
        match thread_stderr.join() {
            Ok(Ok(())) => (),
            Ok(Err(e)) => {
                error!("stderr thread runs into error: {e}");
                has_error = true;
            },
            Err(e) => panic!("stderr thread panics: {e:?}"),
        }

        // finish with the result
        if has_error {
            bail!("execution runs into unexpected error");
        }
        Ok(status)
    }

    /// Interrupt the execution of the subcommand
    pub fn interrupt(self) -> Result<()> {
        self.end(Some(Signal::SIGINT)).map(|_| ())
    }

    /// Wait for the completion of the execution
    pub fn wait(self) -> Result<ExitStatus> {
        self.end(None)
    }

    /// Shortcut: execute to status
    pub fn invoke(command: Command) -> Result<bool> {
        let status = Self::run(command, None, None)?.wait()?;
        Ok(status.success())
    }

    /// Shortcut: execute to output stdout
    pub fn output_stdout(command: Command) -> Result<(bool, Vec<String>)> {
        let stdout = Arc::new(RwLock::new(vec![]));
        let status = Self::run(command, Some(Arc::clone(&stdout)), None)?.wait()?;

        let stream_stdout = Arc::into_inner(stdout)
            .expect("single reference of arc")
            .into_inner()
            .expect("lock not poisoned");
        Ok((status.success(), stream_stdout))
    }

    /// Shortcut: execute to output stderr
    #[allow(dead_code)]
    pub fn output_stderr(command: Command) -> Result<(bool, Vec<String>)> {
        let stderr = Arc::new(RwLock::new(vec![]));
        let status = Self::run(command, None, Some(Arc::clone(&stderr)))?.wait()?;

        let stream_stderr = Arc::into_inner(stderr)
            .expect("single reference of arc")
            .into_inner()
            .expect("lock not poisoned");
        Ok((status.success(), stream_stderr))
    }

    /// Utility function: stream reader
    fn log_and_maybe_capture_stream<T: Read>(
        stream: T,
        log_tag: &str,
        accumulator: Option<Arc<RwLock<Vec<String>>>>,
    ) -> Result<()> {
        let tid = thread::current().id();
        let mut buffer = String::new();
        let mut reader = BufReader::new(stream);
        loop {
            let len = reader
                .read_line(&mut buffer)
                .map_err(|e| anyhow!("failed to read stream: {e}"))?;
            if len == 0 {
                break;
            }

            // NOTE: intentionally remove the newline character
            if buffer.chars().last().expect("at least one char in line") == '\n' {
                buffer.pop();
            }
            if buffer.chars().last().is_some_and(|ch| ch == '\r') {
                buffer.pop();
            }

            trace!("<{tid:?}> |{log_tag}| {buffer}");
            if let Some(lines) = accumulator.as_ref() {
                lines
                    .write()
                    .expect("stream write lock")
                    .push(buffer.clone());
            }

            // always reset the buffer
            buffer.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SubExec;
    use anyhow::Result;
    use std::{
        io::Cursor,
        process::Command,
        sync::{Arc, RwLock},
    };

    #[test]
    fn test_log_and_maybe_capture_stream_trims_crlf() -> Result<()> {
        let accumulator = Arc::new(RwLock::new(vec![]));
        SubExec::log_and_maybe_capture_stream(
            Cursor::new(b"alpha\r\nbeta\nlast"),
            "O",
            Some(Arc::clone(&accumulator)),
        )?;
        assert_eq!(accumulator.read().unwrap().clone(), vec![
            "alpha".to_string(),
            "beta".to_string(),
            "last".to_string()
        ]);
        Ok(())
    }

    #[test]
    fn test_output_stdout_captures_lines() -> Result<()> {
        let mut command = Command::new("bash");
        command.args(["-lc", "printf 'one\\ntwo\\n'"]);
        let (success, stdout) = SubExec::output_stdout(command)?;
        assert!(success);
        assert_eq!(stdout, vec!["one".to_string(), "two".to_string()]);
        Ok(())
    }

    #[test]
    fn test_output_stderr_captures_lines() -> Result<()> {
        let mut command = Command::new("bash");
        command.args(["-lc", "printf 'oops\\n' >&2"]);
        let (success, stderr) = SubExec::output_stderr(command)?;
        assert!(success);
        assert_eq!(stderr, vec!["oops".to_string()]);
        Ok(())
    }
}
