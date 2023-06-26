// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Helpers for fuzz testing.

use aptos_fuzzer::{commands, FuzzTarget};
use clap::Parser;
use std::{env, ffi::OsString, fs, path::PathBuf};

#[derive(Debug, Parser)]
#[clap(name = "fuzzer", about = "Aptos fuzzer")]
struct Opt {
    /// Print extended debug output
    #[clap(long = "debug")]
    debug: bool,
    #[clap(subcommand)]
    cmd: Command,
}

/// The default number of items to generate in a corpus.
const GENERATE_DEFAULT_ITEMS: usize = 128;

#[derive(Debug, Parser)]
enum Command {
    /// Generate corpus for a particular fuzz target
    #[clap(name = "generate")]
    Generate {
        /// Number of items to generate in the corpus
        #[clap(
            short = 'n',
            long = "num-items",
            default_value_t = GENERATE_DEFAULT_ITEMS
        )]
        num_items: usize,
        /// Custom directory for corpus output to be stored in (required if not running under
        /// `cargo run`)
        #[clap(long = "corpus-dir", value_parser)]
        corpus_dir: Option<PathBuf>,
        #[clap(name = "TARGET")]
        /// Name of target to generate (use `list` to list)
        target: FuzzTarget,
    },
    /// Run fuzzer on specified target (must be run under `cargo run`)
    #[clap(name = "fuzz")]
    Fuzz {
        /// Target to fuzz (use `list` to list targets)
        #[clap(name = "TARGET", required = true)]
        target: FuzzTarget,
        /// Custom directory for corpus
        #[clap(long = "corpus-dir", value_parser)]
        corpus_dir: Option<PathBuf>,
        /// Custom directory for artifacts
        #[clap(long = "artifact-dir", value_parser)]
        artifact_dir: Option<PathBuf>,
        /// Arguments for `cargo fuzz run`
        #[clap(name = "ARGS", value_parser, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// List fuzz targets
    #[clap(name = "list")]
    List {
        /// Only print out names, no descriptions.
        #[clap(long = "no-desc", short = 'n')]
        no_desc: bool,
    },
}

/// The default directory for corpuses. Also return whether the directory was freshly created.
fn default_corpus_dir(target: FuzzTarget) -> (PathBuf, bool) {
    default_dir(target, "corpus")
}

/// The default directory for artifacts.
fn default_artifact_dir(target: FuzzTarget) -> PathBuf {
    default_dir(target, "artifacts").0
}

fn default_dir(target: FuzzTarget, intermediate_dir: &str) -> (PathBuf, bool) {
    let mut dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect(
        "--corpus-dir not set and this binary is not running under cargo run. \
         Either use cargo run or pass in the --corpus-dir flag.",
    ));
    // If a "fuzz" subdirectory doesn't exist, the user might be doing it wrong.
    dir.push("fuzz");
    if !dir.is_dir() {
        panic!(
            "Subdirectory {:?} of cargo manifest directory does not exist \
             (did you run `cargo fuzz init`?)",
            dir
        );
    }

    // The name of the corpus is derived from the name of the target.
    dir.push(intermediate_dir);
    dir.push(target.name());

    println!("Using default {} directory: {:?}", intermediate_dir, dir);
    let created = !dir.exists();
    fs::create_dir_all(&dir).expect("Failed to create directory");
    (dir, created)
}

fn main() {
    let opt: Opt = Opt::parse();

    match opt.cmd {
        Command::Generate {
            num_items,
            corpus_dir,
            target,
        } => {
            let corpus_dir = corpus_dir.unwrap_or_else(|| default_corpus_dir(target).0);
            let item_count = commands::make_corpus(target, num_items, &corpus_dir, opt.debug)
                .expect("Failed to create corpus");
            println!("Wrote {} items to corpus", item_count);
        },
        Command::Fuzz {
            corpus_dir,
            artifact_dir,
            target,
            args,
        } => {
            let corpus_dir = match corpus_dir {
                Some(dir) => {
                    // Don't generate the corpus here -- custom directory means the user knows
                    // what they're doing.
                    dir
                },
                None => {
                    let (dir, created) = default_corpus_dir(target);
                    if created {
                        println!("New corpus, generating...");
                        commands::make_corpus(target, GENERATE_DEFAULT_ITEMS, &dir, opt.debug)
                            .expect("Failed to create corpus");
                    }
                    dir
                },
            };
            let artifact_dir = artifact_dir.unwrap_or_else(|| default_artifact_dir(target));
            commands::fuzz_target(target, corpus_dir, artifact_dir, args).unwrap();
        },
        Command::List { no_desc } => {
            commands::list_targets(no_desc);
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Opt::command().debug_assert()
}
