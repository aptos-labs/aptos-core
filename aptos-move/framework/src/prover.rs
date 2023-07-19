// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::build_model;
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{ColorChoice, StandardStream},
};
use log::LevelFilter;
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeMap, path::Path, time::Instant};
use tempfile::TempDir;

#[derive(Debug, Clone, clap::Parser, serde::Serialize, serde::Deserialize)]
pub struct ProverOptions {
    /// Verbosity level
    #[clap(long, short)]
    pub verbosity: Option<LevelFilter>,

    /// Filters targets out from the package. Any module with a matching file name will
    /// be a target, similar as with `cargo test`.
    #[clap(long, short)]
    pub filter: Option<String>,

    /// Whether to display additional information in error reports. This may help
    /// debugging but also can make verification slower.
    #[clap(long, short)]
    pub trace: bool,

    /// Whether to use cvc5 as the smt solver backend. The environment variable
    /// `CVC5_EXE` should point to the binary.
    #[clap(long)]
    pub cvc5: bool,

    /// The depth until which stratified functions are expanded.
    #[clap(long, default_value_t = 6)]
    pub stratification_depth: usize,

    /// A seed for the prover.
    #[clap(long, default_value_t = 0)]
    pub random_seed: usize,

    /// The number of cores to use for parallel processing of verification conditions.
    #[clap(long, default_value_t = 4)]
    pub proc_cores: usize,

    /// A (soft) timeout for the solver, per verification condition, in seconds.
    #[clap(long, default_value_t = 40)]
    pub vc_timeout: usize,

    /// Whether to check consistency of specs by injecting impossible assertions.
    #[clap(long)]
    pub check_inconsistency: bool,

    /// Whether to keep loops as they are and pass them on to the underlying solver.
    #[clap(long)]
    pub keep_loops: bool,

    /// Number of iterations to unroll loops.
    #[clap(long)]
    pub loop_unroll: Option<u64>,

    /// Whether output for e.g. diagnosis shall be stable/redacted so it can be used in test
    /// output.
    #[clap(long)]
    pub stable_test_output: bool,

    /// Whether to dump intermediate step results to files.
    #[clap(long)]
    pub dump: bool,

    #[clap(skip)]
    pub for_test: bool,
}

impl Default for ProverOptions {
    fn default() -> Self {
        Self {
            verbosity: None,
            filter: None,
            trace: false,
            cvc5: false,
            stratification_depth: 6,
            random_seed: 0,
            proc_cores: 4,
            vc_timeout: 40,
            check_inconsistency: false,
            keep_loops: false,
            loop_unroll: None,
            stable_test_output: false,
            dump: false,
            for_test: false,
        }
    }
}

impl ProverOptions {
    /// Runs the move prover on the package.
    pub fn prove(
        self,
        dev_mode: bool,
        package_path: &Path,
        named_addresses: BTreeMap<String, AccountAddress>,
        bytecode_version: Option<u32>,
    ) -> anyhow::Result<()> {
        let now = Instant::now();
        let for_test = self.for_test;
        let model = build_model(
            dev_mode,
            package_path,
            named_addresses,
            self.filter.clone(),
            bytecode_version,
        )?;
        let mut options = self.convert_options();
        // Need to ensure a distinct output.bpl file for concurrent execution. In non-test
        // mode, we actually want to use the static output.bpl for debugging purposes
        let _temp_holder = if for_test {
            let temp_dir = TempDir::new()?;
            std::fs::create_dir_all(temp_dir.path())?;
            options.output_path = temp_dir
                .path()
                .join("boogie.bpl")
                .to_string_lossy()
                .to_string();
            Some(temp_dir)
        } else {
            options.output_path = std::env::current_dir()?
                .join("boogie.bpl")
                .display()
                .to_string();
            None
        };
        options.backend.custom_natives =
            Some(move_prover_boogie_backend::options::CustomNativeOptions {
                template_bytes: include_bytes!("aptos-natives.bpl").to_vec(),
                module_instance_names: vec![(
                    "0x1::object".to_string(),
                    "object_instances".to_string(),
                    true,
                )],
            });
        let mut writer = StandardStream::stderr(ColorChoice::Auto);
        move_prover::run_move_prover_with_model(&model, &mut writer, options, Some(now))?;
        Ok(())
    }

    fn convert_options(self) -> move_prover::cli::Options {
        let verbosity_level = if let Some(level) = self.verbosity {
            level
        } else if self.for_test {
            LevelFilter::Warn
        } else {
            LevelFilter::Info
        };
        let opts = move_prover::cli::Options {
            output_path: "".to_string(),
            verbosity_level,
            prover: move_stackless_bytecode::options::ProverOptions {
                stable_test_output: self.stable_test_output,
                auto_trace_level: if self.trace {
                    move_stackless_bytecode::options::AutoTraceLevel::VerifiedFunction
                } else {
                    move_stackless_bytecode::options::AutoTraceLevel::Off
                },
                report_severity: Severity::Warning,
                dump_bytecode: self.dump,
                dump_cfg: false,
                check_inconsistency: self.check_inconsistency,
                skip_loop_analysis: self.keep_loops,
                ..Default::default()
            },
            backend: move_prover_boogie_backend::options::BoogieOptions {
                use_cvc5: self.cvc5,
                boogie_flags: vec![],
                generate_smt: self.dump,
                stratification_depth: self.stratification_depth,
                proc_cores: self.proc_cores,
                vc_timeout: self.vc_timeout,
                keep_artifacts: self.dump,
                stable_test_output: self.stable_test_output,
                z3_trace_file: if self.dump {
                    Some("z3.trace".to_string())
                } else {
                    None
                },
                custom_natives: None,
                loop_unroll: self.loop_unroll,
                ..Default::default()
            },
            ..Default::default()
        };
        if self.for_test {
            opts.setup_logging_for_test();
        } else {
            opts.setup_logging()
        }
        opts
    }

    pub fn default_for_test() -> Self {
        Self {
            for_test: true,
            ..Self::default()
        }
    }
}
