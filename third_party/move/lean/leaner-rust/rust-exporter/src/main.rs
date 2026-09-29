// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Rustc Public M0 driver spike.
//!
//! The exporter detaches a versioned `LeanerIR.Import.RawUnit` document in the
//! callback below. The exchange schema belongs to `leaner-ir`; the local typed
//! mirror is extended one exhaustively rejected mapper slice at a time.

#![cfg_attr(feature = "rustc-public", feature(rustc_private))]

#[cfg(feature = "rustc-public")]
extern crate rustc_driver;
#[cfg(feature = "rustc-public")]
extern crate rustc_interface;
#[cfg(feature = "rustc-public")]
extern crate rustc_middle;
#[cfg(feature = "rustc-public")]
extern crate rustc_public;

#[cfg(feature = "rustc-public")]
mod driver {
    mod exchange;
    mod mapper;
    mod probe;

    use rustc_public::CompilerError;
    use std::{
        env, fs,
        io::{self, Write},
        ops::ControlFlow,
        path::PathBuf,
        process::{Command, ExitCode},
    };

    struct Options {
        output: Option<PathBuf>,
        rustc_args: Vec<String>,
    }

    enum Invocation {
        Direct(Options),
        CargoRoot(Options),
        CargoDependency(ExitCode),
    }

    fn exit_code(status: std::process::ExitStatus) -> ExitCode {
        match status.code() {
            Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
            _ => ExitCode::FAILURE,
        }
    }

    fn argument_value<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
        arguments
            .windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
            .or_else(|| {
                let prefix = format!("{name}=");
                arguments
                    .iter()
                    .find_map(|argument| argument.strip_prefix(&prefix))
            })
    }

    /// Cargo invokes `RUSTC_WRAPPER` as `wrapper rustc <rustc arguments>`.
    /// Delegate every dependency to Cargo's compiler and intercept only the
    /// explicitly selected primary library target.
    fn cargo_invocation(output: String) -> Result<Invocation, String> {
        let mut arguments = env::args().skip(1);
        let rustc = arguments
            .next()
            .ok_or_else(|| "Cargo wrapper invocation is missing its rustc path".to_owned())?;
        let rustc_args = arguments.collect::<Vec<_>>();
        let package = env::var("LEANER_RUST_CARGO_PACKAGE")
            .map_err(|_| "LEANER_RUST_CARGO_PACKAGE is not set".to_owned())?;
        let crate_name = env::var("LEANER_RUST_CARGO_CRATE")
            .map_err(|_| "LEANER_RUST_CARGO_CRATE is not set".to_owned())?;
        let selected = env::var_os("CARGO_PRIMARY_PACKAGE").is_some()
            && env::var("CARGO_PKG_NAME").is_ok_and(|name| name == package)
            && argument_value(&rustc_args, "--crate-name") == Some(crate_name.as_str());
        if !selected {
            let status = Command::new(rustc)
                .args(&rustc_args)
                .status()
                .map_err(|error| format!("run Cargo's rustc for a dependency: {error}"))?;
            return Ok(Invocation::CargoDependency(exit_code(status)));
        }
        let mut rustc_args = rustc_args;
        rustc_args.push("-Cpanic=abort".to_owned());
        rustc_args.insert(0, "rustc".to_owned());
        Ok(Invocation::CargoRoot(Options {
            output: Some(PathBuf::from(output)),
            rustc_args,
        }))
    }

    fn options() -> Result<Invocation, String> {
        if let Ok(output) = env::var("LEANER_RUST_CARGO_OUTPUT") {
            return cargo_invocation(output);
        }
        let args = env::args().skip(1).collect::<Vec<_>>();
        let (exporter_args, mut rustc_args) = match args.iter().position(|arg| arg == "--") {
            Some(delimiter) => (args[..delimiter].to_vec(), args[delimiter + 1..].to_vec()),
            None => (vec![], args),
        };
        let mut output = None;
        let mut index = 0;
        while index < exporter_args.len() {
            match exporter_args[index].as_str() {
                "--output" => {
                    let path = exporter_args
                        .get(index + 1)
                        .ok_or_else(|| "--output requires a path or `-`".to_owned())?;
                    if output.replace(PathBuf::from(path)).is_some() {
                        return Err("--output may be specified only once".to_owned());
                    }
                    index += 2;
                },
                argument => return Err(format!("unknown exporter argument `{argument}`")),
            }
        }
        if rustc_args.is_empty() {
            return Err(
                "usage: leaner-rust-export [--output <path|->] -- <rustc arguments>".to_owned(),
            );
        }
        if output.is_some() {
            // The initial Rust profile has only terminal panic semantics. Put
            // this last so an emitted artifact cannot claim `panic=abort`
            // while observing an earlier user-supplied unwind setting.
            rustc_args.push("-Cpanic=abort".to_owned());
        }
        rustc_args.insert(0, "rustc".to_owned());
        Ok(Invocation::Direct(Options { output, rustc_args }))
    }

    /// Runs inside Rustc Public's post-analysis callback.  `Break` is
    /// intentionally translated by `run!` to `Compilation::Stop`, so no LLVM
    /// code generation, object production, or linking follows this probe.
    fn extract_m0(emit_artifact: bool) -> ControlFlow<Result<Option<String>, String>, ()> {
        let report = probe::collect();
        eprintln!("leaner-rust-export: {report}");
        eprintln!("leaner-rust-export: stopping before codegen");
        let artifact = if emit_artifact {
            mapper::raw_unit_json().map(Some)
        } else {
            Ok(None)
        };
        ControlFlow::Break(artifact)
    }

    fn write_artifact(path: &PathBuf, artifact: &str) -> Result<(), String> {
        if path.as_os_str() == "-" {
            let mut stdout = io::stdout().lock();
            stdout
                .write_all(artifact.as_bytes())
                .and_then(|()| stdout.flush())
                .map_err(|error| format!("write RawUnit JSON to stdout: {error}"))
        } else {
            fs::write(path, artifact)
                .map_err(|error| format!("write RawUnit JSON to `{}`: {error}", path.display()))
        }
    }

    pub fn main() -> ExitCode {
        let invocation = match options() {
            Ok(invocation) => invocation,
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(2);
            },
        };
        let options = match invocation {
            Invocation::Direct(options) | Invocation::CargoRoot(options) => options,
            Invocation::CargoDependency(exit_code) => return exit_code,
        };
        let emit_artifact = options.output.is_some();

        match rustc_public::run!(&options.rustc_args, || extract_m0(emit_artifact)) {
            Err(CompilerError::Interrupted(Ok(Some(artifact)))) => {
                match write_artifact(
                    options.output.as_ref().expect("artifact output exists"),
                    &artifact,
                ) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(message) => {
                        eprintln!("leaner-rust-export: {message}");
                        ExitCode::FAILURE
                    },
                }
            },
            Err(CompilerError::Interrupted(Ok(None))) => ExitCode::SUCCESS,
            Err(CompilerError::Interrupted(Err(message))) => {
                eprintln!("leaner-rust-export: cannot emit RawUnit: {message}");
                ExitCode::FAILURE
            },
            Ok(()) => {
                eprintln!("leaner-rust-export: compiler unexpectedly continued after analysis");
                ExitCode::FAILURE
            },
            Err(error) => {
                eprintln!("leaner-rust-export: Rustc Public failed: {error:?}");
                ExitCode::FAILURE
            },
        }
    }
}

#[cfg(feature = "rustc-public")]
fn main() -> std::process::ExitCode {
    driver::main()
}

#[cfg(not(feature = "rustc-public"))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "leaner-rust-export was built without Rustc Public support; \\
         rebuild with `cargo build --features rustc-public`"
    );
    std::process::ExitCode::from(2)
}
