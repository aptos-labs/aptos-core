// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Functionality related to the command line interface of the Move prover.

use anyhow::anyhow;
use clap::{builder::PossibleValuesParser, Arg, ArgAction, ArgAction::SetTrue, Command};
use codespan_reporting::diagnostic::Severity;
use legacy_move_compiler::{command_line::SKIP_ATTRIBUTE_CHECKS, shared::NumericalAddress};
use log::LevelFilter;
use move_abigen::AbigenOptions;
use move_docgen::DocgenOptions;
use move_errmapgen::ErrmapOptions;
use move_model::{
    metadata::LanguageVersion, model::VerificationScope, options::ModelBuilderOptions,
};
use move_prover_boogie_backend::{
    options,
    options::{BoogieOptions, CustomNativeOptions, VectorTheory},
};
use move_prover_bytecode_pipeline::options::{AutoTraceLevel, ProverOptions};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Represents options provided to the tool. Most of those options are configured via a toml
/// source; some over the command line flags.
///
/// NOTE: any fields carrying structured data must appear at the end for making
/// toml printing work. When changing this config, use `mvp --print-config` to
/// verify this works.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// The path to the boogie output which represents the verification problem.
    pub output_path: String,
    /// Verbosity level for logging.
    pub verbosity_level: LevelFilter,
    /// Whether to run the documentation generator instead of the prover.
    pub run_docgen: bool,
    /// Whether to run the ABI generator instead of the prover.
    pub run_abigen: bool,
    /// Whether to run the error map generator instead of the prover.
    pub run_errmapgen: bool,
    /// Whether to run the read write set analysis instead of the prover
    pub run_read_write_set: bool,
    /// The paths to the Move sources.
    /// Each source path should refer to either (1) a Move file or (2) a directory containing Move
    /// files, all to be compiled (e.g., not the root directory of a package---which contains
    /// Move.toml---but a specific subdirectory such as `sources`, `scripts`, and/or `tests`,
    /// depending on compilation mode).
    pub move_sources: Vec<String>,
    /// The paths to any dependencies for the Move sources. Those will not be verified but
    /// can be used by `move_sources`.
    /// Each move_dep path should refer to either (1) a Move file or (2) a directory containing
    /// Move files, all to be compiled (e.g., not the root directory of a package---which contains
    /// Move.toml---but a specific subdirectory such as `sources`).
    pub move_deps: Vec<String>,
    /// The values assigned to named addresses in the Move code being verified.
    pub move_named_address_values: Vec<String>,
    /// Whether to run experimental pipeline
    pub experimental_pipeline: bool,
    /// Whether to skip checking for unknown attributes
    pub skip_attribute_checks: bool,
    /// The language version to use
    pub language_version: Option<LanguageVersion>,
    /// BEGIN OF STRUCTURED OPTIONS. DO NOT ADD VALUE FIELDS AFTER THIS
    /// Options for the model builder.
    pub model_builder: ModelBuilderOptions,
    /// Options for the documentation generator.
    pub docgen: DocgenOptions,
    /// Options for the prover.
    pub prover: ProverOptions,
    /// Options for the prover backend.
    pub backend: BoogieOptions,
    /// Options for the ABI generator.
    pub abigen: AbigenOptions,
    /// Options for the error map generator.
    /// TODO: this currently create errors during deserialization, so skip them for this.
    #[serde(skip_serializing)]
    pub errmapgen: ErrmapOptions,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            output_path: "output.bpl".to_string(),
            run_docgen: false,
            run_abigen: false,
            run_errmapgen: false,
            run_read_write_set: false,
            verbosity_level: LevelFilter::Info,
            move_sources: vec![],
            move_deps: vec![],
            move_named_address_values: vec![],
            model_builder: ModelBuilderOptions::default(),
            prover: ProverOptions::default(),
            backend: BoogieOptions::default(),
            docgen: DocgenOptions::default(),
            abigen: AbigenOptions::default(),
            errmapgen: ErrmapOptions::default(),
            experimental_pipeline: false,
            skip_attribute_checks: false,
            language_version: Some(LanguageVersion::default()),
        }
    }
}

pub static DEFAULT_OPTIONS: Lazy<Options> = Lazy::new(Options::default);

impl Options {
    /// Creates options from toml configuration source.
    pub fn create_from_toml(toml_source: &str) -> anyhow::Result<Options> {
        Ok(toml::from_str(toml_source)?)
    }

    /// Creates options from toml configuration file.
    pub fn create_from_toml_file(toml_file: &str) -> anyhow::Result<Options> {
        Self::create_from_toml(&std::fs::read_to_string(toml_file)?)
    }

    // Creates options from command line arguments. This parses the arguments and terminates
    // the program on errors, printing usage information. The first argument is expected to be
    // the program name.
    pub fn create_from_args(args: &[String]) -> anyhow::Result<Options> {
        // Clap definition of the command line interface.
        let is_number = |s: &str| {
            s.parse::<usize>()
                .map_err(|_| "expected number".to_string())
        };
        let cli = Command::new("mvp")
            .version("0.1.0")
            .about("The Move Prover")
            .author("The Diem Core Contributors")
            .arg(
                Arg::new("config")
                    .short('c')
                    .long("config")
                    .value_name("TOML_FILE")
                    .help("path to a configuration file. \
                     Values in this file will be overridden by command line flags"),
            )
            .arg(
                Arg::new("config-str")
                    .conflicts_with("config")
                    .short('C')
                    .long("config-str")
                    .num_args(0..)
                    .value_name("TOML_STRING")
                    .help("inlines configuration string in toml syntax. Can be repeated. \
                     Use as in `-C=prover.opt=value -C=backend.opt=value`"),
            )
            .arg(
                Arg::new("print-config")
                    .long("print-config")
                    .action(SetTrue)
                    .help("prints the effective toml configuration, then exits")
            )
            .arg(
                Arg::new("velor")
                    .long("velor")
                    .action(SetTrue)
                    .help("configures the prover to use Velor natives")
            )
            .arg(
                Arg::new("compiler-v2")
                    .long("compiler-v2")
                    .action(SetTrue)
                    .help("whether to use Move compiler v2 to compile to bytecode")
            )
            .arg(
                Arg::new("language-version")
                    .long("language-version")
                    .value_parser(clap::value_parser!(LanguageVersion))
                    .help("the language version to use")
            )
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("BOOGIE_FILE")
                    .help("path to the boogie output which represents the verification problem"),
            )
            .arg(
                Arg::new("verbosity")
                    .short('v')
                    .long("verbose")
                    .value_parser(PossibleValuesParser::new(["error", "warn", "info", "debug"]))
                    .help("verbosity level"),
            )
            .arg(
                Arg::new("vector-theory")
                    .long("vector-theory")
                    .value_parser(PossibleValuesParser::new(["BoogieArray", "BoogieArrayIntern",
                                              "SmtArray", "SmtArrayExt", "SmtSeq"]))
                    .help("vector theory to use"),
            )
            .arg(
                Arg::new("generate-only")
                    .long("generate-only")
                    .short('g')
                    .action(SetTrue)
                    .help("only generates boogie file but does not call boogie"),
            )
            .arg(
                Arg::new("severity")
                    .long("severity")
                    .short('s')
                    .value_parser(PossibleValuesParser::new(["bug", "error", "warn", "note"]))
                    .help("The minimall level on which diagnostics are reported")
            )
            .arg(
                Arg::new("trace")
                    .long("trace")
                    .action(SetTrue)
                    .short('t')
                    .help("enables automatic tracing of expressions in prover errors")
            )
            .arg(
                Arg::new("keep")
                    .long("keep")
                    .short('k')
                    .action(SetTrue)
                    .help("keeps intermediate artifacts of the backend around")
            )
            .arg(
                Arg::new("boogie-poly")
                    .long("boogie-poly")
                    .action(SetTrue)
                    .help("whether to use the old polymorphic Boogie backend")
            )
            .arg(
                Arg::new("inv-v1")
                    .long("inv-v1")
                    .help("whether to use the old v1 invariant processing (without disabled invariants)")
            )
            .arg(
                Arg::new("negative")
                    .long("negative")
                    .help("runs negative verification checks")
            ).arg(
                Arg::new("seed")
                    .long("seed")
                    .short('S')
                    .value_name("NUMBER")
                    .value_parser(is_number)
                    .help("sets a random seed for the prover (default 0)")
            )
            .arg(
                Arg::new("cores")
                    .long("cores")
                    .value_name("NUMBER")
                    .value_parser(is_number)
                    .help("sets the number of cores to use. \
                     NOTE: multiple cores may currently lead to scrambled model \
                     output from boogie (default 4)")
            )
            .arg(
                Arg::new("timeout")
                    .long("timeout")
                    .short('T')
                    .value_name("NUMBER")
                    .value_parser(is_number)
                    .help("sets a timeout (in seconds) for each \
                             individual verification condition (default 40)")
            )
            .arg(
                Arg::new("ignore-pragma-opaque-when-possible")
                    .long("ignore-pragma-opaque-when-possible")
                    .action(SetTrue)
                    .help("Ignore the \"opaque\" pragma on specs of \
                    all functions when possible"),
            )
            .arg(
                Arg::new("ignore-pragma-opaque-internal-only")
                    .long("ignore-pragma-opaque-internal-only")
                    .action(SetTrue)
                    .help("Ignore the \"opaque\" pragma on specs of \
                    internal functions when possible"),
            )
            .arg(
                Arg::new("simplification-pipeline")
                    .long("simplify")
                    .num_args(0..)
                    .help("Specify one simplification pass to run on the specifications. \
                    This option May be specified multiple times to compose a pipeline")
            )
            .arg(
                Arg::new("docgen")
                    .long("docgen")
                    .action(SetTrue)
                    .help("runs the documentation generator instead of the prover. \
                    Generated docs will be written into the directory `./doc` unless configured otherwise via toml"),
            )
            .arg(
                Arg::new("docgen-template")
                    .long("docgen-template")
                    .value_name("FILE")
                    .help("a template for documentation generation."),
            )
            .arg(
                Arg::new("abigen")
                    .long("abigen")
                    .action(SetTrue)
                    .help("runs the ABI generator instead of the prover. \
                    Generated ABIs will be written into the directory `./abi` unless configured otherwise via toml"),
            )
            .arg(
                Arg::new("errmapgen")
                    .long("errmapgen")
                    .action(SetTrue)
                    .help("runs the error map generator instead of the prover. \
                    The generated error map will be written to `errmap` unless configured otherwise"),
            )
            .arg(
                Arg::new("packedtypesgen")
                    .long("packedtypesgen")
                    .help("runs the packed types generator instead of the prover.")
            )
            .arg(
                Arg::new("escape")
                    .long("escape")
                    .help("runs the escape analysis instead of the prover.")
            )
            .arg(
                Arg::new("read-write-set")
                    .long("read-write-set")
                    .action(SetTrue)
                    .help("runs the read/write set analysis instead of the prover.")
            )
            .arg(
                Arg::new("verify")
                    .long("verify")
                    .value_parser(PossibleValuesParser::new(["public", "all", "none"]))
                    .value_name("SCOPE")
                    .help("default scope of verification \
                    (can be overridden by `pragma verify=true|false`)"),
            )
            .arg(
                Arg::new("bench-repeat")
                    .long("bench-repeat")
                    .value_name("COUNT")
                    .value_parser(is_number)
                    .help(
                        "for benchmarking: how many times to call the backend on the verification problem",
                    ),
            )
            .arg(
                Arg::new("dependencies")
                    .long("dependency")
                    .short('d')
                    .action(ArgAction::Append)
                    .value_name("PATH_TO_DEPENDENCY")
                    .help("path to a Move file, or a directory which will be searched for \
                    Move files, containing dependencies which will not be verified")
            )
            .arg(
                Arg::new("named-addresses")
                .long("named-addresses")
                .short('a')
                .num_args(0..)
                .help("specifies the value(s) of named addresses used in Move files")
            )
            .arg(
                Arg::new("sources")
                    .num_args(1..)
                    .value_name("PATH_TO_SOURCE_FILE")
                    .help("the source files to verify"),
            )
            .arg(
                Arg::new("eager-threshold")
                    .long("eager-threshold")
                    .value_name("NUMBER")
                    .value_parser(is_number)
                    .help("sets the eager threshold for quantifier instantiation (default 100)")
            )
            .arg(
                Arg::new("lazy-threshold")
                    .long("lazy-threshold")
                    .value_name("NUMBER")
                    .value_parser(is_number)
                    .help("sets the lazy threshold for quantifier instantiation (default 100)")
            )
            .arg(
                Arg::new("dump-bytecode")
                    .long("dump-bytecode")
                    .action(SetTrue)
                    .help("whether to dump the transformed bytecode to a file")
            )
            .arg(
                Arg::new("dump-cfg")
                    .long("dump-cfg")
                    .action(SetTrue)
                    .requires("dump-bytecode")
                    .help("whether to dump the per-function control-flow graphs (in dot format) to files")
            )
            .arg(
                Arg::new("num-instances")
                    .long("num-instances")
                    .value_name("NUMBER")
                    .value_parser(is_number)
                    .help("sets the number of Boogie instances to run concurrently (default 1)")
            )
            .arg(
                Arg::new("sequential")
                    .long("sequential")
                    .action(SetTrue)
                    .help("whether to run the Boogie instances sequentially")
            )
            .arg(
                Arg::new("stable-test-output")
                    .long("stable-test-output")
                    .action(SetTrue)
                    .help("instruct the prover to produce output in diagnosis which is stable \
                     and suitable for baseline tests. This redacts values in diagnosis which might\
                     be non-deterministic, and may do other things to keep output stable.")
            )
            .arg(
                Arg::new("use-cvc5")
                    .long("use-cvc5")
                    .action(SetTrue)
                    .help("uses cvc5 solver instead of z3")
            )
            .arg(
                Arg::new("use-exp-boogie")
                    .long("use-exp-boogie")
                    .action(SetTrue)
                    .help("uses experimental boogie expected in EXP_BOOGIE_EXE")
            )
            .arg(
                Arg::new("generate-smt")
                    .long("generate-smt")
                    .action(SetTrue)
                    .help("instructs boogie to log smtlib files for verified functions")
            )
            .arg(
                Arg::new("experimental-pipeline")
                    .long("experimental-pipeline")
                    .short('e')
                    .action(SetTrue)
                    .help("whether to run experimental pipeline")
            )
            .arg(
                Arg::new(SKIP_ATTRIBUTE_CHECKS)
                    .long(SKIP_ATTRIBUTE_CHECKS)
                    .action(SetTrue)
                    .help("whether to not complain about unknown attributes in Move code")
            )
            .arg(
                Arg::new("weak-edges")
                    .long("weak-edges")
                    .help("whether to use exclusively weak edges in borrow analysis")
            )
            .arg(
                Arg::new("exp_mut_param")
                    .long("exp-mut-param")
                    .help("exp_mut_param experiment")
            )
            .arg(
                Arg::new("check-inconsistency")
                    .long("check-inconsistency")
                    .action(SetTrue)
                    .help("checks whether there is any inconsistency")
            )
            .arg(
                Arg::new("unconditional-abort-as-inconsistency")
                    .long("unconditional-abort-as-inconsistency")
                    .action(SetTrue)
                    .help("treat functions that do not return (i.e., abort unconditionally) \
                    as inconsistency violations")
            )
            .arg(
                Arg::new("verify-only")
                    .long("verify-only")
                    .value_name("FUNCTION_NAME")
                    .help("only generate verification condition for one function. \
                    This overrides verification scope and can be overridden by the pragma verify=false")
            )
            .arg(
                Arg::new("z3-trace")
                    .long("z3-trace")
                    .value_name("FUNCTION_NAME")
                    .help("only generate verification condition for given function, \
                    and generate a z3 trace file for analysis. The file will be stored \
                    at FUNCTION_NAME.z3log.")
            )
            .arg(
                Arg::new("script-reach")
                    .long("script-reach")
                    .help("For each script function which is verification target, \
                    print out the names of all called functions, directly or indirectly.")
            )
            .arg(
                Arg::new("ban-int-2-bv")
                    .long("ban-int-2-bv")
                    .action(SetTrue)
                    .long("whether allow converting int to bit vector when generating the boogie file")
            )
            .after_help("More options available via `--config file` or `--config-str str`. \
            Use `--print-config` to see format and current values. \
            See `move-prover/src/cli.rs::Option` for documentation.");

        // Parse the arguments. This will abort the program on parsing errors and print help.
        // It will also accept options like --help.
        let matches = cli.get_matches_from(args);

        // Initialize options.
        let get_vec = |s: &str| -> Vec<String> {
            match matches.get_many::<String>(s) {
                Some(vs) => vs.map(|v| v.to_string()).collect(),
                _ => vec![],
            }
        };

        let mut options = if matches.contains_id("config") {
            if matches.contains_id("config-str") {
                return Err(anyhow!(
                    "currently, if `--config` (including via $MOVE_PROVER_CONFIG) is given \
                       `--config-str` cannot be used. Consider editing your \
                       configuration file instead."
                ));
            }
            let value = matches.get_one::<String>("config").unwrap();
            if value.is_empty() {
                Self::default()
            } else {
                Self::create_from_toml_file(value)?
            }
        } else if matches.contains_id("config-str") {
            Self::create_from_toml(matches.get_one::<&str>("config-str").unwrap())?
        } else {
            Options::default()
        };

        // Analyze arguments.
        if matches.contains_id("output") {
            options.output_path = matches.get_one::<String>("output").unwrap().to_string();
        }
        if matches.contains_id("verbosity") {
            options.verbosity_level = match matches.get_one::<String>("verbosity").unwrap().as_str()
            {
                "error" => LevelFilter::Error,
                "warn" => LevelFilter::Warn,
                "info" => LevelFilter::Info,
                "debug" => LevelFilter::Debug,
                _ => unreachable!("should not happen"),
            }
        }
        if matches.contains_id("vector-theory") {
            options.backend.vector_theory =
                match matches.get_one::<String>("vector-theory").unwrap().as_str() {
                    "BoogieArray" => VectorTheory::BoogieArray,
                    "BoogieArrayIntern" => VectorTheory::BoogieArrayIntern,
                    "SmtArray" => VectorTheory::SmtArray,
                    "SmtArrayExt" => VectorTheory::SmtArrayExt,
                    "SmtSeq" => VectorTheory::SmtSeq,
                    _ => unreachable!("should not happen"),
                }
        }

        if matches.contains_id("severity") {
            options.prover.report_severity =
                match matches.get_one::<String>("severity").unwrap().as_str() {
                    "bug" => Severity::Bug,
                    "error" => Severity::Error,
                    "warn" => Severity::Warning,
                    "note" => Severity::Note,
                    _ => unreachable!("should not happen"),
                }
        }

        if matches.get_flag("generate-only") {
            options.prover.generate_only = true;
        }
        if matches
            .get_many::<String>("sources")
            .unwrap_or_default()
            .len()
            > 0
        {
            options.move_sources = get_vec("sources");
        }
        if matches
            .get_many::<String>("dependencies")
            .unwrap_or_default()
            .len()
            > 0
        {
            options.move_deps = get_vec("dependencies");
        }
        if matches
            .get_many::<String>("named-addresses")
            .unwrap_or_default()
            .len()
            > 0
        {
            options.move_named_address_values = get_vec("named-addresses");
        }
        if matches.contains_id("verify") {
            options.prover.verify_scope =
                match matches.get_one::<String>("verify").unwrap().as_str() {
                    "public" => VerificationScope::Public,
                    "all" => VerificationScope::All,
                    "none" => VerificationScope::None,
                    _ => unreachable!("should not happen"),
                }
        }
        if matches.contains_id("bench-repeat") {
            options.backend.bench_repeat = *matches.try_get_one("bench-repeat")?.unwrap();
        }
        if matches.get_flag("ignore-pragma-opaque-when-possible") {
            options.model_builder.ignore_pragma_opaque_when_possible = true;
        }
        if matches.get_flag("ignore-pragma-opaque-internal-only") {
            options.model_builder.ignore_pragma_opaque_internal_only = true;
        }
        if matches.get_flag("docgen") {
            options.run_docgen = true;
        }
        if matches.contains_id("docgen-template") {
            options.run_docgen = true;
            options.docgen.root_doc_templates = vec![matches
                .get_one("docgen-template")
                .map(|s: &&str| s.to_string())
                .unwrap()]
        }
        if matches.get_flag("abigen") {
            options.run_abigen = true;
        }
        if matches.get_flag("errmapgen") {
            options.run_errmapgen = true;
        }
        if matches.get_flag("read-write-set") {
            options.run_read_write_set = true;
        }
        if matches.get_flag("trace") {
            options.prover.auto_trace_level = AutoTraceLevel::VerifiedFunction;
        }
        if matches.get_flag("dump-bytecode") {
            options.prover.dump_bytecode = true;
        }
        if matches.get_flag("dump-cfg") {
            options.prover.dump_cfg = true;
        }
        if matches.contains_id("num-instances") {
            let num_instances = *matches.try_get_one("num-instances")?.unwrap();
            options.backend.num_instances = std::cmp::max(num_instances, 1); // at least one instance
        }
        if matches.get_flag("sequential") {
            options.prover.sequential_task = true;
            options.prover.sequential_task = true;
        }
        if matches.get_flag("stable-test-output") {
            //options.prover.stable_test_output = true;
            options.backend.stable_test_output = true;
        }
        if matches.get_flag("keep") {
            options.backend.keep_artifacts = true;
        }
        if matches.get_flag("boogie-poly") {
            options.prover.boogie_poly = true;
        }
        if matches.contains_id("seed") {
            options.backend.random_seed = *matches.try_get_one("seed")?.unwrap();
        }
        if matches.get_flag("experimental-pipeline") {
            options.experimental_pipeline = true;
        }
        if matches.contains_id(SKIP_ATTRIBUTE_CHECKS) {
            options.skip_attribute_checks = true;
        }
        if matches.contains_id("timeout") {
            options.backend.vc_timeout = *matches.try_get_one("timeout")?.unwrap();
        }
        if matches.contains_id("cores") {
            options.backend.proc_cores = *matches.try_get_one("cores")?.unwrap();
        }
        if matches.contains_id("eager-threshold") {
            options.backend.eager_threshold = *matches.try_get_one("eager-threshold")?.unwrap();
        }
        if matches.contains_id("lazy-threshold") {
            options.backend.lazy_threshold = *matches.try_get_one("lazy-threshold")?.unwrap();
        }
        if matches.get_flag("use-cvc5") {
            options.backend.use_cvc5 = true;
        }
        if matches.get_flag("use-exp-boogie") {
            options.backend.use_exp_boogie = true;
        }
        if matches.get_flag("generate-smt") {
            options.backend.generate_smt = true;
        }

        if matches.get_flag("check-inconsistency") {
            options.prover.check_inconsistency = true;
        }
        if matches.get_flag("unconditional-abort-as-inconsistency") {
            options.prover.unconditional_abort_as_inconsistency = true;
        }

        if matches.contains_id("verify-only") {
            options.prover.verify_scope = VerificationScope::Only(
                matches.get_one::<&str>("verify-only").unwrap().to_string(),
            );
        }

        if matches.contains_id("z3-trace") {
            let mut fun_name = matches.get_one::<&str>("z3-trace").copied().unwrap();
            options.prover.verify_scope = VerificationScope::Only(fun_name.to_string());
            if let Some(i) = fun_name.find("::") {
                fun_name = &fun_name[i + 2..];
            }
            options.backend.z3_trace_file = Some(format!("{}.z3log", fun_name));
        }

        if matches.get_flag("velor") {
            options.backend.custom_natives = Some(CustomNativeOptions {
                template_bytes: include_bytes!(
                    "../../../../velor-move/framework/src/velor-natives.bpl"
                )
                .to_vec(),
                module_instance_names: options::custom_native_options(),
            });
            options
                .move_named_address_values
                .push("Extensions=0x1".to_string())
        }
        if matches.contains_id("language-version") {
            options.language_version = matches
                .get_one::<LanguageVersion>("language-version")
                .cloned();
        }

        options.backend.derive_options();

        if matches.get_flag("print-config") {
            println!("{}", toml::to_string(&options).unwrap());
            Err(anyhow!("exiting"))
        } else {
            Ok(options)
        }
    }

    /// Sets up logging based on provided options. This should be called as early as possible
    /// and before any use of info!, warn! etc.
    pub fn setup_logging(&self) {
        move_compiler_v2::logging::setup_logging(Some(self.verbosity_level))
    }

    pub fn setup_logging_for_test(&self) {
        move_compiler_v2::logging::setup_logging_for_testing(Some(self.verbosity_level))
    }

    /// Convenience function to enable debugging (like high verbosity) on this instance.
    pub fn enable_debug(&mut self) {
        self.verbosity_level = LevelFilter::Debug;
    }

    /// Convenience function to set verbosity level to only show errors and warnings.
    pub fn set_quiet(&mut self) {
        self.verbosity_level = LevelFilter::Warn
    }
}

pub fn named_addresses_for_options(
    named_address_values: &BTreeMap<String, NumericalAddress>,
) -> Vec<String> {
    named_address_values
        .iter()
        .map(|(name, addr)| format!("{}={}", name, addr))
        .collect()
}
