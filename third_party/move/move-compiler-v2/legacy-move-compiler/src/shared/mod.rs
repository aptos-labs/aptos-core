// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    command_line as cli,
    diagnostics::{codes::Severity, Diagnostic, Diagnostics},
};
use clap::*;
use move_command_line_common::env::{bool_to_str, read_bool_env_var};
use move_ir_types::location::*;
use move_symbol_pool::Symbol;
use once_cell::sync::Lazy;
use petgraph::{algo::astar as petgraph_astar, graphmap::DiGraphMap};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug},
    hash::Hash,
    sync::atomic::{AtomicUsize, Ordering as AtomicOrdering},
};

pub mod ast_debug;
pub mod builtins;
pub mod remembering_unique_map;
pub mod unique_map;
pub mod unique_set;

pub use move_command_line_common::{
    address::NumericalAddress,
    parser::{
        parse_address_number as parse_address, parse_u128, parse_u16, parse_u256, parse_u32,
        parse_u64, parse_u8, NumberFormat,
    },
};

//**************************************************************************************************
// Address
//**************************************************************************************************

pub fn parse_named_address(s: &str) -> anyhow::Result<(String, NumericalAddress)> {
    let before_after = s.split('=').collect::<Vec<_>>();

    if before_after.len() != 2 {
        anyhow::bail!(
            "Invalid named address assignment. Must be of the form <address_name>=<address>, but \
             found '{}'",
            s
        );
    }
    let name = before_after[0].parse()?;
    let addr = NumericalAddress::parse_str(before_after[1])
        .map_err(|err| anyhow::format_err!("{}", err))?;
    Ok((name, addr))
}

//**************************************************************************************************
// Name
//**************************************************************************************************

pub trait TName: Eq + Ord + Clone {
    type Key: Ord + Clone;
    type Loc: Copy;
    fn drop_loc(self) -> (Self::Loc, Self::Key);
    fn add_loc(loc: Self::Loc, key: Self::Key) -> Self;
    fn borrow(&self) -> (&Self::Loc, &Self::Key);
}

pub trait Identifier {
    fn value(&self) -> Symbol;
    fn loc(&self) -> Loc;
}

// TODO maybe we should intern these strings somehow
pub type Name = Spanned<Symbol>;

impl TName for Name {
    type Key = Symbol;
    type Loc = Loc;

    fn drop_loc(self) -> (Loc, Symbol) {
        (self.loc, self.value)
    }

    fn add_loc(loc: Loc, key: Symbol) -> Self {
        sp(loc, key)
    }

    fn borrow(&self) -> (&Loc, &Symbol) {
        (&self.loc, &self.value)
    }
}

//**************************************************************************************************
// Graphs
//**************************************************************************************************

pub fn shortest_cycle<'a, T: Ord + Hash>(
    dependency_graph: &DiGraphMap<&'a T, ()>,
    start: &'a T,
) -> Vec<&'a T> {
    let shortest_path = dependency_graph
        .neighbors(start)
        .fold(None, |shortest_path, neighbor| {
            let path_opt = petgraph_astar(
                dependency_graph,
                neighbor,
                |finish| finish == start,
                |_e| 1,
                |_| 0,
            );
            match (shortest_path, path_opt) {
                (p, None) | (None, p) => p,
                (Some((acc_len, acc_path)), Some((cur_len, cur_path))) => Some(
                    if cur_len < acc_len {
                        (cur_len, cur_path)
                    } else {
                        (acc_len, acc_path)
                    },
                ),
            }
        });
    let (_, mut path) = shortest_path.unwrap();
    path.insert(0, start);
    path
}

//**************************************************************************************************
// Compilation Env
//**************************************************************************************************

pub type NamedAddressMap = BTreeMap<Symbol, NumericalAddress>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedAddressMapIndex(usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedAddressMaps(Vec<NamedAddressMap>);

impl NamedAddressMaps {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn insert(&mut self, m: NamedAddressMap) -> NamedAddressMapIndex {
        let index = self.0.len();
        self.0.push(m);
        NamedAddressMapIndex(index)
    }

    pub fn get(&self, idx: NamedAddressMapIndex) -> &NamedAddressMap {
        &self.0[idx.0]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagePaths<
    Path: Into<Symbol> + Debug = Symbol,
    NamedAddress: Into<Symbol> + Debug = Symbol,
> {
    pub name: Option<Symbol>,
    pub paths: Vec<Path>,
    pub named_address_map: BTreeMap<NamedAddress, NumericalAddress>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedPackagePath {
    pub package: Option<Symbol>,
    pub path: Symbol,
    pub named_address_map: NamedAddressMapIndex,
}

// Convenient helper functions for dealing with PackagePaths
pub fn string_vec_to_symbol_vec(string_vec: &[String]) -> Vec<Symbol> {
    string_vec
        .iter()
        .map(|s| Symbol::from(s.as_str()))
        .collect()
}

pub fn string_map_to_symbol_map<T: Clone>(string_map: &BTreeMap<String, T>) -> BTreeMap<Symbol, T> {
    string_map
        .iter()
        .map(|(s, v)| (Symbol::from(s.as_str()), v.clone()))
        .collect()
}

pub fn string_packagepath_to_symbol_packagepath<T: Clone>(
    input: &PackagePaths<String, String>,
) -> PackagePaths {
    PackagePaths {
        name: input.name,
        paths: string_vec_to_symbol_vec(&input.paths),
        named_address_map: string_map_to_symbol_map(&input.named_address_map),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationEnv {
    flags: Flags,
    diags: Diagnostics,
    /// Internal table used to pass known attributes to the parser for purposes of
    /// checking for unknown attributes.
    known_attributes: BTreeSet<String>,
    // TODO(tzakian): Remove the global counter and use this counter instead
    // pub counter: u64,
}

impl CompilationEnv {
    pub fn new(flags: Flags, known_attributes: BTreeSet<String>) -> Self {
        Self {
            flags,
            diags: Diagnostics::new(),
            known_attributes,
        }
    }

    pub fn add_diag(&mut self, diag: Diagnostic) {
        self.diags.add(diag)
    }

    pub fn add_diags(&mut self, diags: Diagnostics) {
        self.diags.extend(diags)
    }

    pub fn has_warnings_or_errors(&self) -> bool {
        !self.diags.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        // Non-blocking Error is the min level considered an error
        self.has_diags_at_or_above_severity(Severity::NonblockingError)
    }

    pub fn count_diags(&self) -> usize {
        self.diags.len()
    }

    pub fn has_diags_at_or_above_severity(&self, threshold: Severity) -> bool {
        match self.diags.max_severity() {
            Some(max) if max >= threshold => true,
            Some(_) | None => false,
        }
    }

    pub fn check_diags_at_or_above_severity(
        &mut self,
        threshold: Severity,
    ) -> Result<(), Diagnostics> {
        if self.has_diags_at_or_above_severity(threshold) {
            Err(std::mem::take(&mut self.diags))
        } else {
            Ok(())
        }
    }

    /// Should only be called after compilation is finished
    pub fn take_final_warning_diags(&mut self) -> Diagnostics {
        let final_diags = std::mem::take(&mut self.diags);
        debug_assert!(final_diags
            .max_severity()
            .map(|s| s == Severity::Warning)
            .unwrap_or(true));
        final_diags
    }

    pub fn flags(&self) -> &Flags {
        &self.flags
    }

    pub fn get_known_attributes(&self) -> &BTreeSet<String> {
        &self.known_attributes
    }
}

//**************************************************************************************************
// Counter
//**************************************************************************************************

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Counter(usize);

impl Counter {
    pub fn next() -> u64 {
        static COUNTER_NEXT: AtomicUsize = AtomicUsize::new(0);

        COUNTER_NEXT.fetch_add(1, AtomicOrdering::AcqRel) as u64
    }
}

//**************************************************************************************************
// Display
//**************************************************************************************************

pub fn format_delim<T: fmt::Display, I: IntoIterator<Item = T>>(items: I, delim: &str) -> String {
    items
        .into_iter()
        .map(|item| format!("{}", item))
        .collect::<Vec<_>>()
        .join(delim)
}

pub fn format_comma<T: fmt::Display, I: IntoIterator<Item = T>>(items: I) -> String {
    format_delim(items, ", ")
}

//**************************************************************************************************
// Flags
//**************************************************************************************************

pub fn debug_compiler_env_var() -> bool {
    static DEBUG_COMPILER: Lazy<bool> = Lazy::new(|| {
        read_bool_env_var(cli::MOVE_COMPILER_DEBUG_ENV_VAR)
            || read_bool_env_var(cli::MVC_DEBUG_ENV_VAR)
    });
    *DEBUG_COMPILER
}

pub fn move_compiler_warn_of_deprecation_use_env_var() -> bool {
    static WARN_OF_DEPRECATION: Lazy<bool> =
        Lazy::new(|| read_bool_env_var(cli::MOVE_COMPILER_WARN_OF_DEPRECATION_USE));
    *WARN_OF_DEPRECATION
}

pub fn warn_of_deprecation_use_in_aptos_libs_env_var() -> bool {
    static WARN_OF_DEPRECATION: Lazy<bool> =
        Lazy::new(|| read_bool_env_var(cli::WARN_OF_DEPRECATION_USE_IN_APTOS_LIBS));
    *WARN_OF_DEPRECATION
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
pub struct Flags {
    /// Compile in test mode
    #[clap(
        short = cli::TEST_SHORT,
        long = cli::TEST,
    )]
    test: bool,

    /// Compile in verification mode
    #[clap(
    short = cli::VERIFY_SHORT,
    long = cli::VERIFY,
    )]
    verify: bool,

    /// If set, source files will not shadow dependency files. If the same file is passed to both,
    /// an error will be raised
    #[clap(
        name = "SOURCES_SHADOW_DEPS",
        short = cli::SHADOW_SHORT,
        long = cli::SHADOW,
    )]
    shadow: bool,

    /// Internal flag used by the model builder to maintain functions which would be otherwise
    /// included only in tests, without creating the unit test code regular tests do.
    #[clap(skip)]
    keep_testing_functions: bool,

    /// Do not complain about unknown attributes.
    #[clap(
	long = cli::SKIP_ATTRIBUTE_CHECKS,
    )]
    skip_attribute_checks: bool,

    /// Debug compiler by printing out internal information
    #[clap(long = cli::DEBUG_FLAG, default_value=bool_to_str(debug_compiler_env_var()))]
    debug: bool,

    /// Show warnings about use of deprecated functions, modules, constants, etc.
    /// Note that current value of this constant is "Wdeprecation"
    #[clap(long = cli::MOVE_COMPILER_WARN_OF_DEPRECATION_USE_FLAG,
           default_value=bool_to_str(move_compiler_warn_of_deprecation_use_env_var()))]
    warn_of_deprecation_use: bool,

    /// Show warnings about use of deprecated usage in the Aptos libraries,
    /// which we should generally not bother users with.
    /// Note that current value of this constant is "Wdeprecation-aptos"
    #[clap(long = cli::WARN_OF_DEPRECATION_USE_IN_APTOS_LIBS_FLAG, default_value=bool_to_str(warn_of_deprecation_use_in_aptos_libs_env_var()))]
    warn_of_deprecation_use_in_aptos_libs: bool,

    /// Show warnings about unused functions, fields, constants, etc.
    /// Note that the current value of this constant is "Wunused"
    #[clap(long = cli::WARN_UNUSED_FLAG, default_value="false")]
    warn_unused: bool,

    /// Support Move 2 language features (up to expansion phase)
    #[clap(long = cli::LANG_V2_FLAG)]
    lang_v2: bool,

    /// Language version
    #[clap(long = cli::LANGUAGE_VERSION)]
    language_version: LanguageVersion,
}

impl Flags {
    pub fn empty() -> Self {
        Self {
            test: false,
            verify: false,
            shadow: false,
            keep_testing_functions: false,
            skip_attribute_checks: false,
            debug: debug_compiler_env_var(),
            warn_of_deprecation_use: move_compiler_warn_of_deprecation_use_env_var(),
            warn_of_deprecation_use_in_aptos_libs: warn_of_deprecation_use_in_aptos_libs_env_var(),
            warn_unused: false,
            lang_v2: false,
            language_version: LanguageVersion::V1,
        }
    }

    pub fn testing() -> Self {
        Self {
            test: true,
            ..Self::empty()
        }
    }

    pub fn all_functions() -> Self {
        Self {
            test: true,
            verify: true,
            ..Self::empty()
        }
    }

    pub fn verification() -> Self {
        Self {
            test: false,
            verify: true,
            shadow: true, // allows overlapping between sources and deps
            ..Self::empty()
        }
    }

    pub fn model_compilation() -> Self {
        Self {
            test: false,
            verify: true,
            shadow: true, // allows overlapping between sources and deps
            keep_testing_functions: true,
            lang_v2: true,
            ..Self::empty()
        }
    }

    pub fn set_verify(self, value: bool) -> Self {
        Self {
            verify: value,
            ..self
        }
    }

    pub fn set_keep_testing_functions(self, value: bool) -> Self {
        Self {
            keep_testing_functions: value,
            ..self
        }
    }

    pub fn set_sources_shadow_deps(self, sources_shadow_deps: bool) -> Self {
        Self {
            shadow: sources_shadow_deps,
            ..self
        }
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::empty()
    }

    pub fn is_testing(&self) -> bool {
        self.test
    }

    pub fn keep_testing_functions(&self) -> bool {
        self.test || self.keep_testing_functions
    }

    pub fn is_verification(&self) -> bool {
        self.verify
    }

    pub fn sources_shadow_deps(&self) -> bool {
        self.shadow
    }

    pub fn skip_attribute_checks(&self) -> bool {
        self.skip_attribute_checks
    }

    pub fn set_skip_attribute_checks(self, new_value: bool) -> Self {
        Self {
            skip_attribute_checks: new_value,
            ..self
        }
    }

    pub fn warn_of_deprecation_use(&self) -> bool {
        self.warn_of_deprecation_use
    }

    pub fn set_warn_of_deprecation_use(self, new_value: bool) -> Self {
        Self {
            warn_of_deprecation_use: new_value,
            ..self
        }
    }

    pub fn warn_of_deprecation_use_in_aptos_libs(&self) -> bool {
        self.warn_of_deprecation_use_in_aptos_libs
    }

    pub fn set_warn_of_deprecation_use_in_aptos_libs(self, new_value: bool) -> Self {
        Self {
            warn_of_deprecation_use_in_aptos_libs: new_value,
            ..self
        }
    }

    pub fn warn_unused(&self) -> bool {
        self.warn_unused
    }

    pub fn set_warn_unused(self, new_value: bool) -> Self {
        Self {
            warn_unused: new_value,
            ..self
        }
    }

    pub fn debug(&self) -> bool {
        self.debug
    }

    pub fn lang_v2(&self) -> bool {
        self.lang_v2
    }

    pub fn language_version(&self) -> LanguageVersion {
        self.language_version
    }

    pub fn set_language_version(self, language_version: LanguageVersion) -> Self {
        Self {
            language_version,
            lang_v2: language_version >= LanguageVersion::V2_0,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum LanguageVersion {
    #[value(name = "1")]
    V1,
    #[value(name = "2")]
    V2, /* V2 is the same as V2_1, here for the parser */
    #[value(name = "2.0")]
    V2_0,
    #[value(name = "2.1")]
    V2_1,
    #[value(name = "2.2")]
    V2_2,
    #[value(name = "2.3")]
    V2_3,
}

impl LanguageVersion {
    fn to_ordinal(self) -> usize {
        use LanguageVersion::*;
        match self {
            V1 => 0,
            V2_0 => 1,
            V2 | V2_1 => 2,
            V2_2 => 3,
            V2_3 => 4,
        }
    }
}

impl PartialEq<LanguageVersion> for LanguageVersion {
    fn eq(&self, other: &LanguageVersion) -> bool {
        self.to_ordinal() == other.to_ordinal()
    }
}

impl Eq for LanguageVersion {}

impl PartialOrd<LanguageVersion> for LanguageVersion {
    fn partial_cmp(&self, other: &LanguageVersion) -> Option<std::cmp::Ordering> {
        Some(self.to_ordinal().cmp(&other.to_ordinal()))
    }
}

impl Ord for LanguageVersion {
    fn cmp(&self, other: &LanguageVersion) -> std::cmp::Ordering {
        self.to_ordinal().cmp(&other.to_ordinal())
    }
}

impl std::fmt::Display for LanguageVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            LanguageVersion::V1 => "1",
            LanguageVersion::V2 => "2",
            LanguageVersion::V2_0 => "2.0",
            LanguageVersion::V2_1 => "2.1",
            LanguageVersion::V2_2 => "2.2",
            LanguageVersion::V2_3 => "2.3",
        })
    }
}

//**************************************************************************************************
// Attributes
//**************************************************************************************************

pub mod known_attributes {
    use once_cell::sync::Lazy;
    use std::{collections::BTreeSet, fmt};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum AttributePosition {
        AddressBlock,
        Module,
        Script,
        Use,
        Friend,
        Constant,
        Struct,
        Function,
        Spec,
    }

    pub trait AttributeKind
    where
        Self: Sized,
    {
        fn add_attribute_names(table: &mut BTreeSet<String>);
        fn name(&self) -> &str;
        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition>;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum KnownAttribute {
        Testing(TestingAttribute),
        Verification(VerificationAttribute),
        Native(NativeAttribute),
        Deprecation(DeprecationAttribute),
        Lint(LintAttribute),
        Execution(ExecutionAttribute),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum TestingAttribute {
        // Can be called by other testing code, and included in compilation in test mode
        TestOnly,
        // Is a test that will be run
        Test,
        // This test is expected to fail
        ExpectedFailure,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum VerificationAttribute {
        // The associated AST node will be included in the compilation in prove mode
        VerifyOnly,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum NativeAttribute {
        // It is a fake native function that actually compiles to a bytecode instruction
        BytecodeInstruction,
        NativeInterface,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum DeprecationAttribute {
        // Marks deprecated functions, types, modules, constants, addresses whose use causes warnings
        Deprecated,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum LintAttribute {
        // Allow the user to suppress a specific subset of lint warnings.
        Allow,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum ExecutionAttribute {
        /// Marks a function as being persistent on upgrade (behave like a public function)
        Persistent,
        /// Marks a function to establish a module reentrancy lock during execution
        ModuleLock,
    }

    impl fmt::Display for AttributePosition {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::AddressBlock => write!(f, "address block"),
                Self::Module => write!(f, "module"),
                Self::Script => write!(f, "script"),
                Self::Use => write!(f, "use"),
                Self::Friend => write!(f, "friend"),
                Self::Constant => write!(f, "constant"),
                Self::Struct => write!(f, "struct"),
                Self::Function => write!(f, "function"),
                Self::Spec => write!(f, "spec"),
            }
        }
    }

    impl KnownAttribute {
        pub fn resolve(attribute_str: impl AsRef<str>) -> Option<Self> {
            Some(match attribute_str.as_ref() {
                TestingAttribute::TEST => Self::Testing(TestingAttribute::Test),
                TestingAttribute::TEST_ONLY => Self::Testing(TestingAttribute::TestOnly),
                TestingAttribute::EXPECTED_FAILURE => {
                    Self::Testing(TestingAttribute::ExpectedFailure)
                },
                VerificationAttribute::VERIFY_ONLY => {
                    Self::Verification(VerificationAttribute::VerifyOnly)
                },
                NativeAttribute::BYTECODE_INSTRUCTION => {
                    Self::Native(NativeAttribute::BytecodeInstruction)
                },
                NativeAttribute::NATIVE_INTERFACE => Self::Native(NativeAttribute::NativeInterface),
                DeprecationAttribute::DEPRECATED_NAME => {
                    Self::Deprecation(DeprecationAttribute::Deprecated)
                },
                LintAttribute::SKIP => Self::Lint(LintAttribute::Allow),
                _ => return None,
            })
        }

        pub fn get_all_attribute_names() -> &'static BTreeSet<String> {
            static KNOWN_ATTRIBUTES_SET: Lazy<BTreeSet<String>> = Lazy::new(|| {
                let mut known_attributes = BTreeSet::new();
                KnownAttribute::add_attribute_names(&mut known_attributes);
                known_attributes
            });
            &KNOWN_ATTRIBUTES_SET
        }
    }

    impl AttributeKind for KnownAttribute {
        fn add_attribute_names(table: &mut BTreeSet<String>) {
            TestingAttribute::add_attribute_names(table);
            VerificationAttribute::add_attribute_names(table);
            NativeAttribute::add_attribute_names(table);
            DeprecationAttribute::add_attribute_names(table);
            LintAttribute::add_attribute_names(table);
            ExecutionAttribute::add_attribute_names(table);
        }

        fn name(&self) -> &str {
            match self {
                Self::Testing(a) => a.name(),
                Self::Verification(a) => a.name(),
                Self::Native(a) => a.name(),
                Self::Deprecation(a) => a.name(),
                Self::Lint(a) => a.name(),
                Self::Execution(a) => a.name(),
            }
        }

        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition> {
            match self {
                Self::Testing(a) => a.expected_positions(),
                Self::Verification(a) => a.expected_positions(),
                Self::Native(a) => a.expected_positions(),
                Self::Deprecation(a) => a.expected_positions(),
                Self::Lint(a) => a.expected_positions(),
                Self::Execution(a) => a.expected_positions(),
            }
        }
    }

    impl TestingAttribute {
        pub const ABORT_CODE_NAME: &'static str = "abort_code";
        const ALL_ATTRIBUTE_NAMES: [&'static str; 3] =
            [Self::TEST, Self::TEST_ONLY, Self::EXPECTED_FAILURE];
        pub const ARITHMETIC_ERROR_NAME: &'static str = "arithmetic_error";
        pub const ERROR_LOCATION: &'static str = "location";
        pub const EXPECTED_FAILURE: &'static str = "expected_failure";
        pub const MAJOR_STATUS_NAME: &'static str = "major_status";
        pub const MINOR_STATUS_NAME: &'static str = "minor_status";
        pub const OUT_OF_GAS_NAME: &'static str = "out_of_gas";
        pub const TEST: &'static str = "test";
        pub const TEST_ONLY: &'static str = "test_only";
        pub const VECTOR_ERROR_NAME: &'static str = "vector_error";

        pub fn expected_failure_cases() -> &'static [&'static str] {
            &[
                Self::ABORT_CODE_NAME,
                Self::ARITHMETIC_ERROR_NAME,
                Self::VECTOR_ERROR_NAME,
                Self::OUT_OF_GAS_NAME,
                Self::MAJOR_STATUS_NAME,
            ]
        }
    }
    impl AttributeKind for TestingAttribute {
        fn add_attribute_names(table: &mut BTreeSet<String>) {
            for str in Self::ALL_ATTRIBUTE_NAMES {
                table.insert(str.to_string());
            }
        }

        fn name(&self) -> &str {
            match self {
                Self::Test => Self::TEST,
                Self::TestOnly => Self::TEST_ONLY,
                Self::ExpectedFailure => Self::EXPECTED_FAILURE,
            }
        }

        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition> {
            static TEST_ONLY_POSITIONS: Lazy<BTreeSet<AttributePosition>> = Lazy::new(|| {
                IntoIterator::into_iter([
                    AttributePosition::AddressBlock,
                    AttributePosition::Module,
                    AttributePosition::Use,
                    AttributePosition::Friend,
                    AttributePosition::Constant,
                    AttributePosition::Struct,
                    AttributePosition::Function,
                ])
                .collect()
            });
            static TEST_POSITIONS: Lazy<BTreeSet<AttributePosition>> =
                Lazy::new(|| IntoIterator::into_iter([AttributePosition::Function]).collect());
            static EXPECTED_FAILURE_POSITIONS: Lazy<BTreeSet<AttributePosition>> =
                Lazy::new(|| IntoIterator::into_iter([AttributePosition::Function]).collect());
            match self {
                TestingAttribute::TestOnly => &TEST_ONLY_POSITIONS,
                TestingAttribute::Test => &TEST_POSITIONS,
                TestingAttribute::ExpectedFailure => &EXPECTED_FAILURE_POSITIONS,
            }
        }
    }

    impl VerificationAttribute {
        const ALL_ATTRIBUTE_NAMES: [&'static str; 1] = [Self::VERIFY_ONLY];
        pub const VERIFY_ONLY: &'static str = "verify_only";
    }
    impl AttributeKind for VerificationAttribute {
        fn add_attribute_names(table: &mut BTreeSet<String>) {
            for str in Self::ALL_ATTRIBUTE_NAMES {
                table.insert(str.to_string());
            }
        }

        fn name(&self) -> &str {
            match self {
                Self::VerifyOnly => Self::VERIFY_ONLY,
            }
        }

        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition> {
            static VERIFY_ONLY_POSITIONS: Lazy<BTreeSet<AttributePosition>> = Lazy::new(|| {
                IntoIterator::into_iter([
                    AttributePosition::AddressBlock,
                    AttributePosition::Module,
                    AttributePosition::Use,
                    AttributePosition::Friend,
                    AttributePosition::Constant,
                    AttributePosition::Struct,
                    AttributePosition::Function,
                ])
                .collect()
            });
            match self {
                Self::VerifyOnly => &VERIFY_ONLY_POSITIONS,
            }
        }
    }

    impl NativeAttribute {
        const ALL_ATTRIBUTE_NAMES: [&'static str; 2] =
            [Self::BYTECODE_INSTRUCTION, Self::NATIVE_INTERFACE];
        pub const BYTECODE_INSTRUCTION: &'static str = "bytecode_instruction";
        pub const NATIVE_INTERFACE: &'static str = "native_interface";
    }
    impl AttributeKind for NativeAttribute {
        fn add_attribute_names(table: &mut BTreeSet<String>) {
            for str in Self::ALL_ATTRIBUTE_NAMES {
                table.insert(str.to_string());
            }
        }

        fn name(&self) -> &str {
            match self {
                NativeAttribute::BytecodeInstruction => Self::BYTECODE_INSTRUCTION,
                NativeAttribute::NativeInterface => Self::NATIVE_INTERFACE,
            }
        }

        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition> {
            static BYTECODE_INSTRUCTION_POSITIONS: Lazy<BTreeSet<AttributePosition>> =
                Lazy::new(|| IntoIterator::into_iter([AttributePosition::Function]).collect());
            static NATIVE_INTERFACE_POSITIONS: Lazy<BTreeSet<AttributePosition>> =
                Lazy::new(|| IntoIterator::into_iter([AttributePosition::Function]).collect());
            match self {
                NativeAttribute::BytecodeInstruction => &BYTECODE_INSTRUCTION_POSITIONS,
                NativeAttribute::NativeInterface => &NATIVE_INTERFACE_POSITIONS,
            }
        }
    }

    impl DeprecationAttribute {
        const ALL_ATTRIBUTE_NAMES: [&'static str; 1] = [Self::DEPRECATED_NAME];
        pub const DEPRECATED_NAME: &'static str = "deprecated";
    }

    impl AttributeKind for DeprecationAttribute {
        fn add_attribute_names(table: &mut BTreeSet<String>) {
            for str in Self::ALL_ATTRIBUTE_NAMES {
                table.insert(str.to_string());
            }
        }

        fn name(&self) -> &str {
            match self {
                Self::Deprecated => Self::DEPRECATED_NAME,
            }
        }

        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition> {
            static DEPRECATED_POSITIONS: Lazy<BTreeSet<AttributePosition>> = Lazy::new(|| {
                IntoIterator::into_iter([
                    AttributePosition::AddressBlock,
                    AttributePosition::Module,
                    AttributePosition::Constant,
                    AttributePosition::Struct,
                    AttributePosition::Function,
                ])
                .collect()
            });
            match self {
                Self::Deprecated => &DEPRECATED_POSITIONS,
            }
        }
    }

    impl LintAttribute {
        const ALL_ATTRIBUTE_NAMES: [&'static str; 1] = [Self::SKIP];
        pub const SKIP: &'static str = "lint::skip";
    }

    impl AttributeKind for LintAttribute {
        fn add_attribute_names(table: &mut BTreeSet<String>) {
            for str in Self::ALL_ATTRIBUTE_NAMES {
                table.insert(str.to_string());
            }
        }

        fn name(&self) -> &str {
            match self {
                Self::Allow => Self::SKIP,
            }
        }

        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition> {
            static ALLOW_POSITIONS: Lazy<BTreeSet<AttributePosition>> = Lazy::new(|| {
                IntoIterator::into_iter([AttributePosition::Module, AttributePosition::Function])
                    .collect()
            });
            match self {
                Self::Allow => &ALLOW_POSITIONS,
            }
        }
    }

    impl ExecutionAttribute {
        const ALL_ATTRIBUTE_NAMES: [&'static str; 2] = [Self::MODULE_LOCK, Self::PERSISTENT];
        pub const MODULE_LOCK: &'static str = "module_lock";
        pub const PERSISTENT: &'static str = "persistent";
    }
    impl AttributeKind for ExecutionAttribute {
        fn add_attribute_names(table: &mut BTreeSet<String>) {
            for str in Self::ALL_ATTRIBUTE_NAMES {
                table.insert(str.to_string());
            }
        }

        fn name(&self) -> &str {
            match self {
                Self::Persistent => Self::PERSISTENT,
                Self::ModuleLock => Self::MODULE_LOCK,
            }
        }

        fn expected_positions(&self) -> &'static BTreeSet<AttributePosition> {
            static POSITIONS: Lazy<BTreeSet<AttributePosition>> =
                Lazy::new(|| IntoIterator::into_iter([AttributePosition::Function]).collect());
            &POSITIONS
        }
    }
}
