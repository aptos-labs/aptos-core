// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Command-line front end for the generator:
//!   - `gen`: write one `.masm` test from a seed;
//!   - `gen-many`: write a batch of tests from fixed seeds;
//!   - `check`: assemble and verify an existing `.masm` file.
//!
//! When a test needs edits to its compiled bytecode, `gen` and `gen-many` write them to a
//! `.recipe.json` file next to it, which `check --recipe` applies.

use anyhow::{anyhow, bail, Context, Result};
use arbitrary::Unstructured;
use clap::{Parser, Subcommand};
use either::Either;
use masm_smith::{
    assembler_options, generate, split_assembler_sections, stdlib_modules, Config, MasmTest,
    PostProcessRecipe,
};
use move_asm::assembler;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::verifier;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(name = "masm-smith", about = "Move assembly test-case generator")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Generate one `.masm` test.
    Gen {
        /// Seed bytes for the fuzzer input. Either a hex string (e.g., `0xdeadbeef`) or
        /// `@path/to/file` to read raw bytes from disk. Defaults to reading from stdin.
        #[arg(long)]
        seed: Option<String>,
        /// Output path. Defaults to stdout.
        #[arg(long, short)]
        out: Option<PathBuf>,
        /// Generator config as JSON; fields it omits keep their defaults.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Generate many `.masm` tests with deterministic seeds.
    GenMany {
        /// Number of tests to produce.
        #[arg(long)]
        count: u32,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
        /// Generator config as JSON; fields it omits keep their defaults.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Assemble + verify an existing `.masm` file.
    Check {
        /// Path to the `.masm` file.
        path: PathBuf,
        /// Optional recipe sidecar to apply after assembly.
        #[arg(long)]
        recipe: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Gen { seed, out, config } => run_gen(seed, out, load_config(config.as_deref())?),
        Cmd::GenMany {
            count,
            out_dir,
            config,
        } => run_gen_many(count, out_dir, load_config(config.as_deref())?),
        Cmd::Check { path, recipe } => run_check(path, recipe),
    }
}

/// The config at `path`, or the default config when there is none.
fn load_config(path: Option<&Path>) -> Result<Config> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing config {}", path.display()))
}

fn run_gen(seed: Option<String>, out: Option<PathBuf>, cfg: Config) -> Result<()> {
    let bytes = load_seed(seed.as_deref())?;
    let mut u = Unstructured::new(&bytes);
    let test = generate(&mut u, &cfg).map_err(|e| anyhow!("generation failed: {e}"))?;
    let text = test.render();
    match out {
        None => {
            io::stdout().write_all(text.as_bytes())?;
        },
        Some(path) => {
            fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
            write_recipe_sidecar(&path, &test)?;
            eprintln!("wrote {}", path.display());
        },
    }
    Ok(())
}

fn run_gen_many(count: u32, out_dir: PathBuf, cfg: Config) -> Result<()> {
    fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    for i in 0..count {
        // Uniform bytes per index: zero padding would make nearly every generator decision
        // take its minimum and yield empty programs.
        let seed = masm_smith::seeded_bytes(u64::from(i), GEN_MANY_SEED_BYTES);
        let mut u = Unstructured::new(&seed);
        let test = match generate(&mut u, &cfg) {
            Ok(t) => t,
            Err(masm_smith::Error::OutOfInput) => continue,
            Err(e) => bail!("seed {} generation failed: {e}", i),
        };
        let out_path = out_dir.join(format!("test_{:05}.masm", i));
        fs::write(&out_path, test.render())
            .with_context(|| format!("writing {}", out_path.display()))?;
        write_recipe_sidecar(&out_path, &test)?;
    }
    Ok(())
}

/// Input bytes per `gen-many` program, enough that generation rarely runs out.
const GEN_MANY_SEED_BYTES: usize = 16 * 1024;

fn run_check(path: PathBuf, recipe: Option<PathBuf>) -> Result<()> {
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let recipe: Option<PostProcessRecipe> = match recipe {
        None => None,
        Some(p) => {
            let s = fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
            Some(serde_json::from_str(&s).context("parsing recipe JSON")?)
        },
    };
    let sections = split_assembler_sections(&text);
    if sections.is_empty() {
        bail!(
            "no `//# publish` or `//# run` sections found in {}",
            path.display()
        );
    }
    let mut overall_ok = true;
    let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
    for (idx, section) in sections.iter().enumerate() {
        let opts = assembler_options();
        match assembler::assemble(&opts, section, context.iter()) {
            Ok(Either::Left(module)) => {
                // Verify a recipe-corrupted clone but keep the pristine module as assembly
                // context — bounds-corrupting recipes leave the module unsafe to read, and
                // the assembler eagerly walks every context module when building later
                // sections.
                let mut for_verify = module.clone();
                if let Some(r) = recipe.as_ref() {
                    r.apply_to_module(idx, &mut for_verify);
                }
                match verifier::verify_module(&for_verify) {
                    Ok(()) => println!("section #{idx}: assemble + verify OK (module)"),
                    Err(err) => {
                        overall_ok = false;
                        println!("section #{idx}: verify FAILED: {:?}", err);
                    },
                }
                context.push(module);
            },
            Ok(Either::Right(script)) => match verifier::verify_script(&script) {
                Ok(()) => println!("section #{idx}: assemble + verify OK (script)"),
                Err(err) => {
                    overall_ok = false;
                    println!("section #{idx}: verify FAILED: {:?}", err);
                },
            },
            Err(diags) => {
                overall_ok = false;
                let rendered =
                    assembler::diag_to_string(path.to_string_lossy().as_ref(), section, diags);
                println!("section #{idx}: ASSEMBLY FAILED:\n{}", rendered);
            },
        }
    }
    if overall_ok {
        Ok(())
    } else {
        bail!("check failed")
    }
}

fn write_recipe_sidecar(masm_path: &Path, test: &MasmTest) -> Result<()> {
    let Some(recipe) = test.post_process.as_ref() else {
        return Ok(());
    };
    if recipe.is_empty() {
        return Ok(());
    }
    let recipe_path = masm_path.with_extension("recipe.json");
    let json = serde_json::to_string_pretty(recipe).context("serializing recipe")?;
    fs::write(&recipe_path, json).with_context(|| format!("writing {}", recipe_path.display()))?;
    Ok(())
}

fn load_seed(seed: Option<&str>) -> Result<Vec<u8>> {
    match seed {
        None => {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf)?;
            Ok(buf)
        },
        Some(s) if s.starts_with('@') => {
            let path = &s[1..];
            Ok(fs::read(path).with_context(|| format!("reading seed file {}", path))?)
        },
        Some(s) => {
            let hex = s.strip_prefix("0x").unwrap_or(s);
            hex::decode(hex).map_err(|e| anyhow!("invalid hex seed: {e}"))
        },
    }
}
