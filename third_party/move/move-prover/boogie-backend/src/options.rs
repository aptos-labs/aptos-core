// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use itertools::Itertools;
use move_command_line_common::env::{read_bool_env_var, read_env_var};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Default flags passed to boogie. Additional flags will be added to this via the -B option.
const DEFAULT_BOOGIE_FLAGS: &[&str] = &[
    "-doModSetAnalysis",
    "-printVerifiedProceduresCount:0",
    "-printModel:1",
    "-enhancedErrorMessages:1",
    "-proverOpt:O:model_validate=true",
];

/// Versions for boogie, z3, and cvc5. The upgrade of boogie and z3 is mostly backward compatible,
/// but not always. Setting the max version allows Prover to warn users for the higher version of
/// boogie and z3 because those may be incompatible.
pub const MIN_BOOGIE_VERSION: Option<&str> = Some("3.0.1.0");
pub const MAX_BOOGIE_VERSION: Option<&str> = Some("3.2.4.0");

pub const MIN_Z3_VERSION: Option<&str> = Some("4.11.2");
pub const MAX_Z3_VERSION: Option<&str> = Some("4.11.2");

pub const MIN_CVC5_VERSION: Option<&str> = Some("0.0.3");
pub const MAX_CVC5_VERSION: Option<&str> = None;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VectorTheory {
    BoogieArray,
    BoogieArrayIntern,
    SmtArray,
    SmtArrayExt,
    SmtSeq,
}

impl VectorTheory {
    pub fn is_extensional(&self) -> bool {
        matches!(
            self,
            VectorTheory::BoogieArrayIntern | VectorTheory::SmtArrayExt | VectorTheory::SmtSeq
        )
    }
}

/// Options to define custom native functions to include in generated Boogie file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomNativeOptions {
    /// Bytes of the custom template.
    pub template_bytes: Vec<u8>,
    /// List of (module name, module instance key, single_type_expected) tuples,
    /// used to generate instantiated versions of generic native functions.
    pub module_instance_names: Vec<(String, String, bool)>,
}

pub fn custom_native_options() -> Vec<(String, String, bool)> {
    vec![
        (
            "0x1::object".to_string(),
            "object_instances".to_string(),
            true,
        ),
        (
            "0x1::aggregator_v2".to_string(),
            "aggregator_v2_instances".to_string(),
            true,
        ),
    ]
}

/// Contains information about a native method implementing mutable borrow semantics for a given
/// type in an alternative storage model (returning &mut without taking appropriate &mut as a
/// parameter, much like vector::borrow_mut)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowAggregate {
    /// Method's name (qualified with module name, e.g., m::foo)
    pub name: String,
    /// Name of the read aggregate
    pub read_aggregate: String,
    /// Name of the write aggregate
    pub write_aggregate: String,
}

impl BorrowAggregate {
    pub fn new(name: String, read_aggregate: String, write_aggregate: String) -> Self {
        BorrowAggregate {
            name,
            read_aggregate,
            write_aggregate,
        }
    }
}

/// Boogie options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BoogieOptions {
    /// Path to the boogie executable.
    pub boogie_exe: String,
    /// Use experimental boogie exe found via env var EXP_BOOGIE_EXE.
    pub use_exp_boogie: bool,
    /// Path to the z3 executable.
    pub z3_exe: String,
    /// Whether to use cvc5.
    pub use_cvc5: bool,
    /// Path to the cvc5 executable.
    pub cvc5_exe: String,
    /// Whether to generate debug trace code.
    pub debug_trace: bool,
    /// List of flags to pass on to boogie.
    pub boogie_flags: Vec<String>,
    /// Whether to use native array theory.
    pub use_smt_array_theory: bool,
    /// Whether to produce an SMT file for each verification problem.
    pub generate_smt: bool,
    /// Whether native instead of stratified equality should be used.
    pub native_equality: bool,
    /// A string determining the type of requires used for parameter type checks. Can be
    /// `"requires"` or `"free requires`".
    pub type_requires: String,
    /// The depth until which stratified functions are expanded.
    pub stratification_depth: usize,
    /// A string to be used to inline a function of medium size. Can be empty or `{:inline}`.
    pub aggressive_func_inline: String,
    /// A string to be used to inline a function of small size. Can be empty or `{:inline}`.
    pub func_inline: String,
    /// A bound to apply to the length of serialization results.
    pub serialize_bound: usize,
    /// How many times to call the prover backend for the verification problem. This is used for
    /// benchmarking.
    pub bench_repeat: usize,
    /// Whether to use the sequence theory as the internal representation for $Vector type.
    pub vector_using_sequences: bool,
    /// A seed for the prover.
    pub random_seed: usize,
    /// The number of cores to use for parallel processing of verification conditions.
    pub proc_cores: usize,
    /// The number of shards to split the verification problem into.
    pub shards: usize,
    /// If there are shards, specifies to only run the given shard. Shards are numbered
    /// starting at 1.
    pub only_shard: Option<usize>,
    /// A (soft) timeout for the solver, per verification condition, in seconds.
    pub vc_timeout: usize,
    /// Whether allow local timeout overwrites the global one
    pub global_timeout_overwrite: bool,
    /// Whether Boogie output and log should be saved.
    pub keep_artifacts: bool,
    /// Eager threshold for quantifier instantiation.
    pub eager_threshold: usize,
    /// Lazy threshold for quantifier instantiation.
    pub lazy_threshold: usize,
    /// Whether to use the new Boogie `{:debug ..}` attribute for tracking debug values.
    pub stable_test_output: bool,
    /// Number of Boogie instances to be run concurrently.
    pub num_instances: usize,
    /// Whether to run Boogie instances sequentially.
    pub sequential_task: bool,
    /// A hard timeout for boogie execution; if the process does not terminate within
    /// this time frame, it will be killed. Zero for no timeout.
    pub hard_timeout_secs: u64,
    /// Whether to skip verification of type instantiations of functions. This may miss
    /// some verification conditions if different type instantiations can create
    /// different behavior via type reflection or storage access, but can speed up
    /// verification.
    pub skip_instance_check: bool,
    /// What vector theory to use.
    pub vector_theory: VectorTheory,
    /// Whether to generate a z3 trace file and where to put it.
    pub z3_trace_file: Option<String>,
    /// Options to define user-custom native funs.
    pub custom_natives: Option<CustomNativeOptions>,
    /// Number of iterations to unroll loops.
    pub loop_unroll: Option<u64>,
    /// Optional aggregate function names for native methods implementing mutable borrow semantics
    pub borrow_aggregates: Vec<BorrowAggregate>,
}

impl Default for BoogieOptions {
    fn default() -> Self {
        Self {
            bench_repeat: 1,
            boogie_exe: read_env_var("BOOGIE_EXE"),
            use_exp_boogie: false,
            z3_exe: read_env_var("Z3_EXE"),
            use_cvc5: false,
            cvc5_exe: read_env_var("CVC5_EXE"),
            boogie_flags: vec![],
            debug_trace: false,
            use_smt_array_theory: false,
            generate_smt: false,
            native_equality: false,
            type_requires: "free requires".to_owned(),
            stratification_depth: 6,
            aggressive_func_inline: "".to_owned(),
            func_inline: "{:inline}".to_owned(),
            serialize_bound: 0,
            vector_using_sequences: false,
            random_seed: 1,
            proc_cores: 4,
            shards: 1,
            only_shard: None,
            vc_timeout: 40,
            global_timeout_overwrite: true,
            keep_artifacts: false,
            eager_threshold: 100,
            lazy_threshold: 100,
            stable_test_output: false,
            num_instances: 1,
            sequential_task: false,
            hard_timeout_secs: 0,
            vector_theory: VectorTheory::BoogieArray,
            z3_trace_file: None,
            custom_natives: None,
            loop_unroll: None,
            borrow_aggregates: vec![],
            skip_instance_check: false,
        }
    }
}

impl BoogieOptions {
    /// Derive options based on other set options.
    pub fn derive_options(&mut self) {
        use VectorTheory::*;
        self.native_equality = self.vector_theory.is_extensional();
        if matches!(self.vector_theory, SmtArray | SmtArrayExt) {
            self.use_smt_array_theory = true;
        }
    }

    /// Returns command line to call boogie.
    pub fn get_boogie_command(&self, boogie_file: &str) -> anyhow::Result<Vec<String>> {
        let mut result = if self.use_exp_boogie {
            // This should have a better ux...
            vec![read_env_var("EXP_BOOGIE_EXE")]
        } else {
            vec![self.boogie_exe.clone()]
        };

        // If we don't have a boogie executable, nothing will work
        if result.iter().all(|path| path.is_empty()) {
            anyhow::bail!("No boogie executable set.  Please set BOOGIE_EXE");
        }

        let mut add = |sl: &[&str]| result.extend(sl.iter().map(|s| (*s).to_string()));
        add(DEFAULT_BOOGIE_FLAGS);
        if self.use_cvc5 {
            add(&[
                "-proverOpt:SOLVER=cvc5",
                &format!("-proverOpt:PROVER_PATH={}", &self.cvc5_exe),
            ]);
        } else {
            add(&[&format!("-proverOpt:PROVER_PATH={}", &self.z3_exe)]);
        }
        if self.use_smt_array_theory {
            if matches!(self.vector_theory, VectorTheory::SmtArray) {
                add(&["/proverOpt:O:smt.array.extensional=false"])
            }
        } else {
            add(&["-useArrayAxioms"]);
            add(&[&format!(
                "-proverOpt:O:smt.QI.EAGER_THRESHOLD={}",
                self.eager_threshold
            )]);
            add(&[&format!(
                "-proverOpt:O:smt.QI.LAZY_THRESHOLD={}",
                self.lazy_threshold
            )]);
        }
        if let Some(iters) = self.loop_unroll {
            add(&[&format!("-loopUnroll:{}", iters)]);
        }
        add(&[&format!(
            "-vcsCores:{}",
            if self.stable_test_output {
                // Do not use multiple cores if stable test output is requested.
                // Error messages may appear in non-deterministic order otherwise.
                1
            } else {
                self.proc_cores
            }
        )]);

        // TODO: see what we can make out of these flags.
        //add(&["-proverOpt:O:smt.QI.PROFILE=true"]);
        //add(&["-proverOpt:O:trace=true"]);
        //add(&["-proverOpt:VERBOSITY=3"]);
        //add(&["-proverOpt:C:-st"]);

        if let Some(file) = &self.z3_trace_file {
            add(&[
                "-proverOpt:O:trace=true",
                &format!("-proverOpt:O:trace_file_name={}", file),
            ]);
        }
        if self.generate_smt {
            add(&["-proverLog:@PROC@.smt"]);
        }
        for f in &self.boogie_flags {
            add(&[f.as_str()]);
        }
        add(&[boogie_file]);
        Ok(result)
    }

    /// Returns name of file where to log boogie output.
    pub fn get_boogie_log_file(&self, boogie_file: &str) -> String {
        format!("{}.log", boogie_file)
    }

    /// Adjust a timeout value, given in seconds, for the runtime environment.
    pub fn adjust_timeout(&self, time: usize) -> usize {
        // If env var MVP_TEST_ON_CI is set, add 100% to the timeout for added
        // robustness against flakiness.
        if read_bool_env_var("MVP_TEST_ON_CI") {
            usize::saturating_add(time, time)
        } else {
            time
        }
    }

    /// Checks whether the expected tool versions are installed in the environment.
    pub fn check_tool_versions(&self) -> anyhow::Result<()> {
        if !self.boogie_exe.is_empty() {
            // On Mac, version arg is `/version`, not `-version`
            let version_arg = if cfg!(target_os = "macos") {
                &["/version"]
            } else {
                &["-version"]
            };

            let version = Self::get_version(
                "boogie",
                &self.boogie_exe,
                version_arg,
                r"version ([0-9.]*)",
            )?;
            Self::check_version_is_compatible(
                "boogie",
                &version,
                MIN_BOOGIE_VERSION,
                MAX_BOOGIE_VERSION,
            )?;
        }
        if !self.z3_exe.is_empty() && !self.use_cvc5 {
            let version =
                Self::get_version("z3", &self.z3_exe, &["--version"], r"version ([0-9.]*)")?;
            Self::check_version_is_compatible("z3", &version, MIN_Z3_VERSION, MAX_Z3_VERSION)?;
        }
        if !self.cvc5_exe.is_empty() && self.use_cvc5 {
            let version =
                Self::get_version("cvc5", &self.cvc5_exe, &["--version"], r"version ([0-9.]*)")?;
            Self::check_version_is_compatible(
                "cvc5",
                &version,
                MIN_CVC5_VERSION,
                MAX_CVC5_VERSION,
            )?;
        }
        Ok(())
    }

    fn get_version(tool: &str, prog: &str, args: &[&str], regex: &str) -> anyhow::Result<String> {
        let out = match Command::new(prog).args(args).output() {
            Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
            Err(msg) => {
                return Err(anyhow!(
                    "cannot execute `{}` to obtain version of `{}`: {}",
                    prog,
                    tool,
                    msg.to_string()
                ))
            },
        };
        if let Some(cap) = Regex::new(regex).unwrap().captures(&out) {
            Ok(cap[1].to_string())
        } else {
            Err(anyhow!("cannot extract version from `{}`", prog))
        }
    }

    pub fn check_version_is_compatible(
        tool: &str,
        given: &str,
        expected_min: Option<&str>,
        expected_max: Option<&str>,
    ) -> anyhow::Result<()> {
        if let Some(expected) = expected_min {
            Self::check_version_le(expected, given, "least", expected, given, tool)?;
        }
        if let Some(expected) = expected_max {
            Self::check_version_le(given, expected, "most", expected, given, tool)?;
        }
        Ok(())
    }

    // This function checks if expected_lesser is actually less than or equal to expected_greater
    fn check_version_le(
        expected_lesser: &str,
        expected_greater: &str,
        relative_term: &str,
        expected_version: &str,
        given_version: &str,
        tool: &str,
    ) -> anyhow::Result<()> {
        let lesser_parts = expected_lesser.split('.').collect_vec();
        let greater_parts = expected_greater.split('.').collect_vec();

        if lesser_parts.len() < greater_parts.len() {
            return Err(anyhow!(
                "version strings {} and {} for `{}` cannot be compared",
                given_version,
                expected_version,
                tool
            ));
        }

        for (l, g) in lesser_parts.into_iter().zip(greater_parts.into_iter()) {
            let ln = l.parse::<usize>()?;
            let gn = g.parse::<usize>()?;
            if gn < ln {
                return Err(anyhow!(
                    "expected at {} version {} but found {} for `{}`",
                    relative_term,
                    expected_version,
                    given_version,
                    tool
                ));
            }
            if gn > ln {
                break;
            }
        }
        Ok(())
    }
}
