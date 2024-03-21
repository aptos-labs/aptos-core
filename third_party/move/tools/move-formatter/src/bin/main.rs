// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use commentfmt::{load_config, CliOptions, Config, EmitMode, Verbosity};
use getopts::{Matches, Options};
use io::Error as IoError;
use movefmt::{
    core::fmt::format_entry,
    tools::movefmt_diff::{make_diff, print_mismatches_default_message, DIFF_CONTEXT_SIZE},
    tools::utils::*,
};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("MOVEFMT_LOG"))
        .init();
    let opts = make_opts();

    let exit_code = match execute(&opts) {
        Ok(code) => code,
        Err(e) => {
            tracing::info!("{e:#}");
            1
        }
    };
    // Make sure standard output is flushed before we exit.
    std::io::stdout().flush().unwrap();

    // Exit with given exit code.
    //
    // NOTE: this immediately terminates the process without doing any cleanup,
    // so make sure to finish all necessary cleanup before this is called.
    std::process::exit(exit_code);
}

/// movefmt operations.
enum Operation {
    /// Format files and their child modules.
    Format { files: Vec<PathBuf> },
    /// Print the help message.
    Help(HelpOp),
    /// Print version information
    Version,
    /// Output default config to a file, or stdout if None
    ConfigOutputDefault { path: Option<String> },
    /// Output current config (as if formatting to a file) to stdout
    ConfigOutputCurrent { path: Option<String> },
}

/// movefmt operations errors.
#[derive(Error, Debug)]
pub enum OperationError {
    /// An unknown help topic was requested.
    #[error("Unknown help topic: `{0}`.")]
    UnknownHelpTopic(String),
    /// An unknown print-config option was requested.
    #[error("Unknown print-config option: `{0}`.")]
    UnknownPrintConfigTopic(String),
    /// An io error during reading or writing.
    #[error("{0}")]
    IoError(IoError),
}

impl From<IoError> for OperationError {
    fn from(e: IoError) -> OperationError {
        OperationError::IoError(e)
    }
}

/// Arguments to `--help`
enum HelpOp {
    None,
    Config,
}

fn make_opts() -> Options {
    let mut opts = Options::new();
    let emit_opts = "[files|new_files|stdout|check_diff]";

    opts.optopt("", "emit", "What data to emit and how", emit_opts);
    opts.optopt(
        "",
        "config-path",
        "Recursively searches the given path for the movefmt.toml config file",
        "[Path for the configuration file]",
    );
    opts.optopt(
        "",
        "print-config",
        "Dumps a default or current config to PATH(eg: movefmt.toml)",
        "[default|current] PATH",
    );
    opts.optmulti(
        "",
        "config",
        "Set options from command line. These settings take priority over .movefmt.toml",
        "[key1=val1,key2=val2...]",
    );

    opts.optflag("v", "verbose", "Print verbose output");
    opts.optflag("q", "quiet", "Print less output");
    opts.optflag("V", "version", "Show version information");
    let help_topic_msg = "Show help".to_owned();
    opts.optflagopt("h", "help", &help_topic_msg, "=TOPIC");

    opts
}

// Returned i32 is an exit code
fn execute(opts: &Options) -> Result<i32> {
    let matches = opts.parse(env::args().skip(1))?;
    let options = GetOptsOptions::from_matches(&matches)?;

    match determine_operation(&matches)? {
        Operation::Help(HelpOp::None) => {
            print_usage_to_stdout(opts, "");
            Ok(0)
        }
        Operation::Help(HelpOp::Config) => {
            print_usage_to_stdout(opts, "");
            Ok(0)
        }
        Operation::Version => {
            print_version();
            Ok(0)
        }
        Operation::ConfigOutputDefault { path } => {
            let toml = Config::default().all_options().to_toml()?;
            if let Some(path) = path {
                let mut file = File::create(path)?;
                file.write_all(toml.as_bytes())?;
            } else {
                io::stdout().write_all(toml.as_bytes())?;
            }
            Ok(0)
        }
        Operation::ConfigOutputCurrent { path } => {
            let path = match path {
                Some(path) => path,
                None => return Err(format_err!("PATH required for `--print-config current`")),
            };

            let file = PathBuf::from(path);
            let file = file.canonicalize().unwrap_or(file);

            let (config, _) = load_config(Some(file.parent().unwrap()), Some(options))?;
            let toml = config.all_options().to_toml()?;
            io::stdout().write_all(toml.as_bytes())?;

            Ok(0)
        }
        Operation::Format { files } => format(files, &options),
    }
}

fn format(files: Vec<PathBuf>, options: &GetOptsOptions) -> Result<i32> {
    eprintln!("options = {:?}", options);
    let (config, config_path) = load_config(None, Some(options.clone()))?;
    let mut use_config = config.clone();
    tracing::info!(
        "config.[verbose, indent] = [{:?}, {:?}], {:?}",
        config.verbose(),
        config.indent_size(),
        options
    );

    if config.verbose() == Verbosity::Verbose {
        if let Some(path) = config_path.as_ref() {
            println!("Using movefmt config file {}", path.display());
        }
    }

    for file in files {
        if !file.exists() {
            eprintln!("Error: file `{}` does not exist", file.to_str().unwrap());
            continue;
        } else if file.is_dir() {
            eprintln!("Error: `{}` is a directory", file.to_str().unwrap());
            continue;
        } else {
            // Check the file directory if the config-path could not be read or not provided
            if config_path.is_none() {
                let (local_config, config_path) =
                    load_config(Some(file.parent().unwrap()), Some(options.clone()))?;
                tracing::debug!("local config_path = {:?}", config_path);
                if local_config.verbose() == Verbosity::Verbose {
                    if let Some(path) = config_path {
                        println!(
                            "Using movefmt local config file {} for {}",
                            path.display(),
                            file.display()
                        );
                        use_config = local_config.clone();
                    }
                }
            } else if use_config.verbose() == Verbosity::Verbose {
                println!(
                    "Using movefmt config file {} for {}",
                    config_path.clone().unwrap_or_default().display(),
                    file.display()
                );
            }
        }

        let content_origin = std::fs::read_to_string(file.as_path()).unwrap();
        match format_entry(content_origin.clone(), use_config.clone()) {
            Ok(formatted_text) => {
                let emit_mode = if let Some(op_emit) = options.emit_mode {
                    op_emit
                } else {
                    use_config.emit_mode()
                };
                match emit_mode {
                    EmitMode::NewFiles => {
                        std::fs::write(mk_result_filepath(&file.to_path_buf()), formatted_text)?
                    }
                    EmitMode::Files => {
                        std::fs::write(&file, formatted_text)?;
                    }
                    EmitMode::Stdout => {
                        println!("{}", formatted_text);
                    }
                    EmitMode::Diff => {
                        let compare =
                            make_diff(&formatted_text, &content_origin, DIFF_CONTEXT_SIZE);
                        if !compare.is_empty() {
                            let mut failures = HashMap::new();
                            failures.insert(file.to_owned(), compare);
                            print_mismatches_default_message(failures);
                        }
                    }
                }
            }
            Err(_) => {
                tracing::info!("file '{:?}' skipped because of parse not ok", file);
            }
        }
    }
    Ok(0)
}

fn print_usage_to_stdout(opts: &Options, reason: &str) {
    let sep = if reason.is_empty() {
        String::new()
    } else {
        format!("{reason}\n\n")
    };
    let msg = format!("{sep}Format Move code\n\nusage: movefmt [options] <file>...");
    println!("{}", opts.usage(&msg));
}

fn print_version() {
    println!("movefmt v0.0.1");
}

fn determine_operation(matches: &Matches) -> Result<Operation, OperationError> {
    if matches.opt_present("h") {
        let topic = matches.opt_str("h");
        if topic.is_none() {
            return Ok(Operation::Help(HelpOp::None));
        } else if topic == Some("config".to_owned()) {
            return Ok(Operation::Help(HelpOp::Config));
        }
    }
    let mut free_matches = matches.free.iter();
    if let Some(kind) = matches.opt_str("print-config") {
        let path = free_matches.next().cloned();
        match kind.as_str() {
            "default" => return Ok(Operation::ConfigOutputDefault { path }),
            "current" => return Ok(Operation::ConfigOutputCurrent { path }),
            _ => {
                return Err(OperationError::UnknownPrintConfigTopic(kind));
            }
        }
    }

    if matches.opt_present("version") {
        return Ok(Operation::Version);
    }

    let files: Vec<_> = free_matches
        .map(|s| {
            let p = PathBuf::from(s);
            // we will do comparison later, so here tries to canonicalize first
            // to get the expected behavior.
            p.canonicalize().unwrap_or(p)
        })
        .collect();

    if files.is_empty() {
        eprintln!("no file argument is supplied \n-------------------------------------\n");
        return Ok(Operation::Help(HelpOp::None));
    }

    Ok(Operation::Format { files })
}

/// Parsed command line options.
#[derive(Clone, Debug, Default)]
struct GetOptsOptions {
    quiet: bool,
    verbose: bool,
    config_path: Option<PathBuf>,
    emit_mode: Option<EmitMode>,
    inline_config: HashMap<String, String>,
}

impl GetOptsOptions {
    pub fn from_matches(matches: &Matches) -> Result<GetOptsOptions> {
        let mut options = GetOptsOptions {
            verbose: matches.opt_present("verbose"),
            quiet: matches.opt_present("quiet"),
            ..Default::default()
        };
        if options.verbose && options.quiet {
            return Err(format_err!("Can't use both `--verbose` and `--quiet`"));
        }

        options.config_path = matches.opt_str("config-path").map(PathBuf::from);
        if let Some(ref emit_str) = matches.opt_str("emit") {
            options.emit_mode = Some(emit_mode_from_emit_str(emit_str)?);
        }
        options.inline_config = matches
            .opt_strs("config")
            .iter()
            .flat_map(|config| config.split(','))
            .map(
                |key_val| match key_val.char_indices().find(|(_, ch)| *ch == '=') {
                    Some((middle, _)) => {
                        let (key, val) = (&key_val[..middle], &key_val[middle + 1..]);
                        if !Config::is_valid_key_val(key, val) {
                            Err(format_err!("invalid key=val pair: `{}`", key_val))
                        } else {
                            Ok((key.to_string(), val.to_string()))
                        }
                    }

                    None => Err(format_err!(
                        "--config expects comma-separated list of key=val pairs, found `{}`",
                        key_val
                    )),
                },
            )
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(options)
    }
}

impl CliOptions for GetOptsOptions {
    fn apply_to(self, config: &mut Config) {
        if self.verbose {
            config.set().verbose(Verbosity::Verbose);
        } else if self.quiet {
            config.set().verbose(Verbosity::Quiet);
        } else {
            config.set().verbose(Verbosity::Normal);
        }
        if let Some(emit_mode) = self.emit_mode {
            config.set().emit_mode(emit_mode);
        }
        for (key, val) in self.inline_config {
            config.override_value(&key, &val);
        }
    }

    fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }
}

fn emit_mode_from_emit_str(emit_str: &str) -> Result<EmitMode> {
    match emit_str {
        "files" => Ok(EmitMode::Files),
        "new_files" => Ok(EmitMode::NewFiles),
        "stdout" => Ok(EmitMode::Stdout),
        "check_diff" => Ok(EmitMode::Diff),
        _ => Err(format_err!("Invalid value for `--emit`")),
    }
}
