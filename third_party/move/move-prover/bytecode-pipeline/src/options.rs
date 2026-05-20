// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use codespan_reporting::diagnostic::Severity;
use move_model::model::{GlobalEnv, VerificationScope};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum AutoTraceLevel {
    Off,
    VerifiedFunction,
    AllFunctions,
}

impl AutoTraceLevel {
    pub fn verified_functions(self) -> bool {
        use AutoTraceLevel::*;
        matches!(self, VerifiedFunction | AllFunctions)
    }

    pub fn functions(self) -> bool {
        use AutoTraceLevel::*;
        matches!(self, AllFunctions)
    }

    pub fn invariants(self) -> bool {
        use AutoTraceLevel::*;
        matches!(self, VerifiedFunction | AllFunctions)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
#[serde(default, deny_unknown_fields)]
pub struct ProverOptions {
    /// Whether to only generate backend code.
    #[arg(short = 'g', long)]
    pub generate_only: bool,
    /// Whether output for e.g. diagnosis shall be stable/redacted so it can be used in test
    /// output.
    #[arg(skip)]
    pub stable_test_output: bool,
    /// Scope of what functions to verify.
    #[arg(skip = VerificationScope::All)]
    pub verify_scope: VerificationScope,
    /// Auto trace level.
    #[arg(skip = AutoTraceLevel::Off)]
    pub auto_trace_level: AutoTraceLevel,
    /// Minimal severity level for diagnostics to be reported.
    #[arg(skip = Severity::Warning)]
    pub report_severity: Severity,
    /// Whether to dump the transformed stackless bytecode to a file
    #[arg(long)]
    pub dump_bytecode: bool,
    /// Whether to dump the control-flow graphs (in dot format) to files, one per each function
    #[arg(long, requires = "dump_bytecode")]
    pub dump_cfg: bool,
    /// Number of Boogie instances to be run concurrently.
    #[arg(skip = 1usize)]
    pub num_instances: usize,
    /// Whether to run Boogie instances sequentially.
    #[arg(long = "sequential")]
    pub sequential_task: bool,
    /// Whether to check the inconsistency
    #[arg(long)]
    pub check_inconsistency: bool,
    /// Whether to consider a function that abort unconditionally as an inconsistency violation
    #[arg(long)]
    pub unconditional_abort_as_inconsistency: bool,
    /// Whether to run the transformation passes for concrete interpretation (instead of proving)
    #[arg(skip)]
    pub for_interpretation: bool,
    /// Whether to skip loop analysis.
    #[arg(skip)]
    pub skip_loop_analysis: bool,
    /// Whether to run spec inference instead of verification.
    #[arg(skip)]
    pub inference: bool,
    /// Do not add `pragma opaque` to inferred specs.
    #[arg(long)]
    pub no_inference_opaque: bool,
    /// Optional names of native methods (qualified with module name, e.g., m::foo) implementing
    /// mutable borrow semantics
    #[arg(skip)]
    pub borrow_natives: Vec<String>,
    /// Targets to exclude from verification. Each entry must be
    /// `VerificationScope::Only(name)` or `VerificationScope::OnlyModule(name)`.
    #[arg(skip)]
    pub verify_exclude: Vec<VerificationScope>,
    /// Inline spec let bindings by substituting the expression directly into conditions,
    /// instead of creating Identical temps.
    #[arg(long, default_value_t = false)]
    pub inline_spec_lets: bool,
}

// add custom struct for mutation options

impl Default for ProverOptions {
    fn default() -> Self {
        Self {
            generate_only: false,
            stable_test_output: false,
            verify_scope: VerificationScope::All,
            auto_trace_level: AutoTraceLevel::Off,
            report_severity: Severity::Warning,
            dump_bytecode: false,
            dump_cfg: false,
            num_instances: 1,
            sequential_task: false,
            check_inconsistency: false,
            unconditional_abort_as_inconsistency: false,
            for_interpretation: false,
            skip_loop_analysis: false,
            inference: false,
            no_inference_opaque: false,
            borrow_natives: vec![],
            verify_exclude: vec![],
            inline_spec_lets: false,
        }
    }
}

impl ProverOptions {
    pub fn get(env: &GlobalEnv) -> Rc<ProverOptions> {
        if !env.has_extension::<ProverOptions>() {
            env.set_extension(ProverOptions::default())
        }
        env.get_extension::<ProverOptions>().unwrap()
    }

    pub fn set(env: &GlobalEnv, options: ProverOptions) {
        env.set_extension::<ProverOptions>(options);
    }
}
