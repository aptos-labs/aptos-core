// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Move-specific CLI types for package compilation options, entry function arguments,
//! script function arguments, chunked publishing, and related functionality.

use crate::commands::{ArgWithType, FunctionArgType, MemberId};
use anyhow::{bail, Context};
use aptos_api_types::ViewFunction;
use aptos_cli_common::{
    dir_default_to_current, load_account_arg, parse_json_file, parse_map, AccountAddressWrapper,
    CliError, CliTypedResult, TransactionOptions,
};
use aptos_framework::chunked_publish::{
    default_large_packages_module_address, CHUNK_SIZE_IN_BYTES,
};
use aptos_rest_client::aptos_api_types::{EntryFunctionId, MoveStructTag, MoveType, ViewRequest};
use aptos_transaction_simulation::SimulationStateStore;
use aptos_transaction_simulation_session::Session;
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        EntryFunction, MultisigTransactionPayload, Script, TransactionArgument, TransactionPayload,
    },
};
use clap::Parser;
use move_compiler_v2::Experiment;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{StructTag, TypeTag, MODULE_SEPARATOR},
};
use move_model::metadata::{
    CompilerVersion, LanguageVersion, LATEST_STABLE_COMPILER_VERSION,
    LATEST_STABLE_LANGUAGE_VERSION,
};
use move_package::source_package::std_lib::StdVersion;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, str::FromStr};

/// Options for optimization level
#[derive(Debug, Clone, Default, Parser)]
pub enum OptimizationLevel {
    /// No optimizations
    None,
    /// Default optimization level
    #[default]
    Default,
    /// Extra optimizations, that may take more time
    Extra,
}

impl FromStr for OptimizationLevel {
    type Err = anyhow::Error;

    /// Parses an optimization level, or default.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "" | "default" => Ok(Self::Default),
            "extra" => Ok(Self::Extra),
            _ => bail!(
                "unrecognized optimization level `{}` (supported versions: `none`, `default`, `extra`)",
                s
            ),
        }
    }
}

/// Options for compiling a move package.
#[derive(Debug, Clone, Parser)]
pub struct MovePackageOptions {
    /// Path to a move package (the folder with a Move.toml file).  Defaults to current directory.
    #[clap(long, value_parser)]
    pub package_dir: Option<PathBuf>,

    /// Path to save the compiled move package
    ///
    /// Defaults to `<package_dir>/build`
    #[clap(long, value_parser)]
    pub output_dir: Option<PathBuf>,

    /// Named addresses for the move binary
    ///
    /// Example: alice=0x1234, bob=0x5678
    ///
    /// Note: This will fail if there are duplicates in the Move.toml file remove those first.
    #[clap(long, value_parser = parse_map::<String, AccountAddressWrapper>, default_value = "")]
    pub named_addresses: BTreeMap<String, AccountAddressWrapper>,

    /// Override the standard library version by mainnet/testnet/devnet
    #[clap(long, value_parser)]
    pub override_std: Option<StdVersion>,

    /// Skip pulling the latest git dependencies
    ///
    /// If you don't have a network connection, the compiler may fail due
    /// to no ability to pull git dependencies.  This will allow overriding
    /// this for local development.
    #[clap(long)]
    pub skip_fetch_latest_git_deps: bool,

    /// Do not complain about unknown attributes in Move code.
    #[clap(long)]
    pub skip_attribute_checks: bool,

    /// Enables dev mode, which uses all dev-addresses and dev-dependencies
    ///
    /// Dev mode allows for changing dependencies and addresses to the preset [dev-addresses] and
    /// [dev-dependencies] fields.  This works both inside and out of tests for using preset values.
    ///
    /// Currently, it also additionally pulls in all test compilation artifacts
    #[clap(long)]
    pub dev: bool,

    /// Skip extended checks (such as checks for the #[view] attribute) on test code.
    #[clap(long, default_value = "false")]
    pub skip_checks_on_test_code: bool,

    /// Select optimization level.  Choices are "none", "default", or "extra".
    /// Level "extra" may spend more time on expensive optimizations in the future.
    /// Level "none" does no optimizations, possibly leading to use of too many runtime resources.
    /// Level "default" is the recommended level, and the default if not provided.
    #[clap(long, alias = "optimization_level", value_parser = clap::value_parser!(OptimizationLevel))]
    pub optimize: Option<OptimizationLevel>,

    /// Experiments
    #[clap(long, hide(true), num_args = 1.., value_delimiter = ',')]
    pub experiments: Vec<String>,

    /// ...or --bytecode BYTECODE_VERSION
    /// Specify the version of the bytecode the compiler is going to emit.
    /// If not provided, it is inferred from the language version.
    #[clap(long, alias = "bytecode", verbatim_doc_comment)]
    pub bytecode_version: Option<u32>,

    /// ...or --compiler COMPILER_VERSION
    /// Specify the version of the compiler (must be at least 2).
    /// Defaults to the latest stable compiler version.
    #[clap(long, value_parser = clap::value_parser!(CompilerVersion),
           alias = "compiler",
           default_value = LATEST_STABLE_COMPILER_VERSION,
           verbatim_doc_comment)]
    pub compiler_version: Option<CompilerVersion>,

    /// ...or --language LANGUAGE_VERSION
    /// Specify the language version to be supported.
    /// Defaults to the latest stable language version.
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion),
           alias = "language",
           default_value = LATEST_STABLE_LANGUAGE_VERSION,
           verbatim_doc_comment)]
    pub language_version: Option<LanguageVersion>,

    /// Fail the compilation if there are any warnings.
    #[clap(long)]
    pub fail_on_warning: bool,
}

impl Default for MovePackageOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MovePackageOptions {
    pub fn new() -> Self {
        Self {
            dev: false,
            package_dir: None,
            output_dir: None,
            named_addresses: Default::default(),
            override_std: None,
            skip_fetch_latest_git_deps: true,
            bytecode_version: None,
            compiler_version: Some(CompilerVersion::latest_stable()),
            language_version: Some(LanguageVersion::latest_stable()),
            skip_attribute_checks: false,
            skip_checks_on_test_code: false,
            optimize: None,
            fail_on_warning: false,
            experiments: vec![],
        }
    }

    pub fn get_package_path(&self) -> CliTypedResult<PathBuf> {
        dir_default_to_current(self.package_dir.clone())
    }

    /// Retrieve the NamedAddresses, resolving all the account addresses accordingly
    pub fn named_addresses(&self) -> BTreeMap<String, AccountAddress> {
        self.named_addresses
            .clone()
            .into_iter()
            .map(|(key, value)| (key, value.account_address))
            .collect()
    }

    pub fn add_named_address(&mut self, key: String, value: String) {
        self.named_addresses
            .insert(key, AccountAddressWrapper::from_str(&value).unwrap());
    }

    /// Compute the experiments to be used for the compiler.
    pub fn compute_experiments(&self) -> Vec<String> {
        let mut experiments = self.experiments.clone();
        let mut set = |k: &str, v: bool| {
            experiments.push(format!("{}={}", k, if v { "on" } else { "off" }));
        };
        match self.optimize {
            None | Some(OptimizationLevel::Default) => {
                set(Experiment::OPTIMIZE, true);
            },
            Some(OptimizationLevel::None) => {
                set(Experiment::OPTIMIZE, false);
            },
            Some(OptimizationLevel::Extra) => {
                set(Experiment::OPTIMIZE_EXTRA, true);
                set(Experiment::OPTIMIZE, true);
            },
        }
        if self.fail_on_warning {
            set(Experiment::FAIL_ON_WARNING, true);
        }
        experiments
    }
}

#[derive(Clone, Debug, Default, Parser)]
pub struct TypeArgVec {
    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u16 u32 u64 u128 u256 bool address vector signer`
    #[clap(long, num_args = 0..)]
    pub type_args: Vec<MoveType>,
}

impl TryFrom<&Vec<String>> for TypeArgVec {
    type Error = CliError;

    fn try_from(value: &Vec<String>) -> Result<Self, Self::Error> {
        let mut type_args = vec![];
        for string_ref in value {
            type_args.push(
                MoveType::from_str(string_ref)
                    .map_err(|err| CliError::UnableToParse("type argument", err.to_string()))?,
            );
        }
        Ok(TypeArgVec { type_args })
    }
}

impl TryInto<Vec<TypeTag>> for TypeArgVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<TypeTag>, Self::Error> {
        let mut type_tags: Vec<TypeTag> = vec![];
        for type_arg in self.type_args.iter() {
            type_tags.push(
                TypeTag::try_from(type_arg)
                    .map_err(|err| CliError::UnableToParse("type argument", err.to_string()))?,
            );
        }
        Ok(type_tags)
    }
}

#[derive(Clone, Debug, Default, Parser)]
pub struct ArgWithTypeVec {
    /// Arguments combined with their type separated by spaces.
    ///
    /// Supported types [address, bool, hex, string, u8, u16, u32, u64, u128, u256, raw]
    ///
    /// Vectors may be specified using JSON array literal syntax (you may need to escape this with
    /// quotes based on your shell interpreter)
    ///
    /// Example: `address:0x1 bool:true u8:0 u256:1234 "bool:[true, false]" 'address:[["0xace", "0xbee"], []]'`
    #[clap(long, num_args = 0..)]
    pub args: Vec<ArgWithType>,
}

impl TryFrom<&Vec<ArgWithTypeJSON>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_from(value: &Vec<ArgWithTypeJSON>) -> Result<Self, Self::Error> {
        let mut args = vec![];
        for arg_json_ref in value {
            // Detect struct/enum types by checking if arg_type contains MODULE_SEPARATOR
            if arg_json_ref.arg_type.contains(MODULE_SEPARATOR) {
                // Struct/enum types require REST API calls to fetch on-chain module bytecode
                // for type validation and field parsing. This cannot be done in synchronous
                // TryFrom implementation. Use the async method instead.
                return Err(CliError::CommandArgumentError(
                    "Struct and enum arguments require REST API access to fetch module bytecode. \
                     Use the async method check_input_style_async() or try_into_with_client() instead."
                        .to_string(),
                ));
            }
            let function_arg_type = FunctionArgType::from_str(&arg_json_ref.arg_type)?;
            args.push(function_arg_type.parse_arg_json(&arg_json_ref.value)?);
        }
        Ok(ArgWithTypeVec { args })
    }
}

impl TryInto<Vec<TransactionArgument>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<TransactionArgument>, Self::Error> {
        let mut args = vec![];
        for arg in self.args {
            args.push(
                (&arg)
                    .try_into()
                    .context(format!("Failed to parse arg {:?}", arg))
                    .map_err(|err| CliError::CommandArgumentError(err.to_string()))?,
            );
        }
        Ok(args)
    }
}

impl TryInto<Vec<Vec<u8>>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<Vec<u8>>, Self::Error> {
        Ok(self
            .args
            .into_iter()
            .map(|arg_with_type| arg_with_type.arg)
            .collect())
    }
}

impl TryInto<Vec<serde_json::Value>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<serde_json::Value>, Self::Error> {
        let mut args = vec![];
        for arg in self.args {
            args.push(arg.to_json()?);
        }
        Ok(args)
    }
}

/// Common options for constructing an entry function transaction payload.
#[derive(Debug, Parser)]
pub struct EntryFunctionArguments {
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x842ed41fad9640a2ad08fdd7d3e4f7f505319aac7d67e1c0dd6a7cce8732c7e3::message::set_message`
    #[clap(long, required_unless_present = "json_file")]
    pub function_id: Option<MemberId>,

    #[clap(flatten)]
    pub type_arg_vec: TypeArgVec,
    #[clap(flatten)]
    pub arg_vec: ArgWithTypeVec,

    /// JSON file specifying public entry function ID, type arguments, and arguments.
    #[clap(long, value_parser, conflicts_with_all = &["function_id", "args", "type_args"])]
    pub json_file: Option<PathBuf>,
}

impl EntryFunctionArguments {
    /// Get instance as if all fields passed from command line, parsing JSON input file if needed.
    fn check_input_style(self) -> CliTypedResult<EntryFunctionArguments> {
        if let Some(json_path) = self.json_file {
            Ok(parse_json_file::<EntryFunctionArgumentsJSON>(&json_path)?.try_into()?)
        } else {
            Ok(self)
        }
    }

    /// Extended version of check_input_style that supports struct/enum arguments.
    ///
    /// This method is async because it may need to query the blockchain via REST API to:
    /// - Fetch module bytecode for struct/enum type validation
    /// - Verify that struct/enum types exist and are accessible
    /// - Parse field definitions for proper BCS encoding
    ///
    /// Use this method when the JSON file might contain struct/enum arguments (types
    /// containing `::` separator). For simple primitive arguments, the synchronous
    /// `check_input_style()` method can be used instead.
    pub async fn check_input_style_async(
        self,
        rest_client: &aptos_rest_client::Client,
    ) -> CliTypedResult<EntryFunctionArguments> {
        if let Some(json_path) = self.json_file {
            let json_args = parse_json_file::<EntryFunctionArgumentsJSON>(&json_path)?;

            // Check if there are struct/enum arguments
            if json_args.has_struct_or_enum_args() {
                // Parse with REST client for module bytecode access
                Ok(json_args.try_into_with_client(rest_client).await?)
            } else {
                // Use synchronous parsing for backward compatibility with primitive types
                Ok(json_args.try_into()?)
            }
        } else {
            Ok(self)
        }
    }

    /// Convert to EntryFunction with async support for struct/enum arguments.
    pub async fn try_into_entry_function_async(
        self,
        rest_client: &aptos_rest_client::Client,
    ) -> CliTypedResult<EntryFunction> {
        let parsed_arguments = self.check_input_style_async(rest_client).await?;
        let function_id: MemberId = (&parsed_arguments).try_into()?;
        Ok(EntryFunction::new(
            function_id.module_id,
            function_id.member_id,
            parsed_arguments.type_arg_vec.try_into()?,
            parsed_arguments.arg_vec.try_into()?,
        ))
    }

    /// Convert to ViewFunction with async support for struct/enum arguments.
    pub async fn try_into_view_function_async(
        self,
        rest_client: &aptos_rest_client::Client,
    ) -> CliTypedResult<ViewFunction> {
        let view_function_args = self.check_input_style_async(rest_client).await?;
        let function_id: MemberId = (&view_function_args).try_into()?;
        Ok(ViewFunction {
            module: function_id.module_id,
            function: function_id.member_id,
            ty_args: view_function_args.type_arg_vec.try_into()?,
            args: view_function_args.arg_vec.try_into()?,
        })
    }

    /// Parse entry function arguments, using async parsing only if needed.
    ///
    /// This consolidates the common pattern of checking if json_file is present and
    /// conditionally using async or sync parsing. The get_client closure is only
    /// called if async parsing is actually needed (when json_file contains struct/enum arguments).
    ///
    /// For JSON files with only primitive arguments, synchronous parsing is used to avoid
    /// requiring REST API access, preserving backward compatibility and offline workflows.
    pub async fn parse_with_optional_client<F>(self, get_client: F) -> CliTypedResult<EntryFunction>
    where
        F: FnOnce() -> CliTypedResult<aptos_rest_client::Client>,
    {
        if let Some(json_path) = &self.json_file {
            // Parse the JSON file once and use it directly for both the has_struct check and the
            // actual parsing, avoiding a redundant re-parse in the downstream callee.
            let json_args = parse_json_file::<EntryFunctionArgumentsJSON>(json_path)?;

            let parsed_arguments: EntryFunctionArguments = if json_args.has_struct_or_enum_args() {
                // Need REST client for struct/enum parsing
                let rest_client = get_client()?;
                json_args.try_into_with_client(&rest_client).await?
            } else {
                // Only primitive types - use synchronous parsing (no REST needed)
                json_args.try_into()?
            };

            let function_id: MemberId = (&parsed_arguments).try_into()?;
            Ok(EntryFunction::new(
                function_id.module_id,
                function_id.member_id,
                parsed_arguments.type_arg_vec.try_into()?,
                parsed_arguments.arg_vec.try_into()?,
            ))
        } else {
            // No JSON file - use synchronous parsing for command-line arguments
            self.try_into()
        }
    }
}

impl TryInto<EntryFunction> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<EntryFunction, Self::Error> {
        let entry_function_args = self.check_input_style()?;
        let function_id: MemberId = (&entry_function_args).try_into()?;
        Ok(EntryFunction::new(
            function_id.module_id,
            function_id.member_id,
            entry_function_args.type_arg_vec.try_into()?,
            entry_function_args.arg_vec.try_into()?,
        ))
    }
}

impl TryInto<ViewFunction> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<ViewFunction, Self::Error> {
        let view_function_args = self.check_input_style()?;
        let function_id: MemberId = (&view_function_args).try_into()?;
        Ok(ViewFunction {
            module: function_id.module_id,
            function: function_id.member_id,
            ty_args: view_function_args.type_arg_vec.try_into()?,
            args: view_function_args.arg_vec.try_into()?,
        })
    }
}

impl TryInto<MultisigTransactionPayload> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<MultisigTransactionPayload, Self::Error> {
        Ok(MultisigTransactionPayload::EntryFunction(self.try_into()?))
    }
}

impl TryInto<MemberId> for &EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<MemberId, Self::Error> {
        self.function_id
            .clone()
            .ok_or_else(|| CliError::CommandArgumentError("No function ID provided".to_string()))
    }
}

impl TryInto<ViewRequest> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<ViewRequest, Self::Error> {
        let entry_function_args = self.check_input_style()?;
        let function_id: MemberId = (&entry_function_args).try_into()?;
        Ok(ViewRequest {
            function: EntryFunctionId {
                module: function_id.module_id.into(),
                name: function_id.member_id.into(),
            },
            type_arguments: entry_function_args.type_arg_vec.type_args,
            arguments: entry_function_args.arg_vec.try_into()?,
        })
    }
}

/// Common options for constructing a script payload
#[derive(Debug, Default, Parser)]
pub struct ScriptFunctionArguments {
    #[clap(flatten)]
    pub type_arg_vec: TypeArgVec,
    #[clap(flatten)]
    pub arg_vec: ArgWithTypeVec,

    /// JSON file specifying type arguments and arguments.
    #[clap(long, value_parser, conflicts_with_all = &["args", "type_args"])]
    pub json_file: Option<PathBuf>,
}

impl ScriptFunctionArguments {
    /// Get instance as if all fields passed from command line, parsing JSON input file if needed.
    fn check_input_style(self) -> CliTypedResult<ScriptFunctionArguments> {
        if let Some(json_path) = self.json_file {
            Ok(parse_json_file::<ScriptFunctionArgumentsJSON>(&json_path)?.try_into()?)
        } else {
            Ok(self)
        }
    }

    pub fn create_script_payload(self, bytecode: Vec<u8>) -> CliTypedResult<TransactionPayload> {
        let script_function_args = self.check_input_style()?;
        Ok(TransactionPayload::Script(Script::new(
            bytecode,
            script_function_args.type_arg_vec.try_into()?,
            script_function_args.arg_vec.try_into()?,
        )))
    }
}

#[derive(Deserialize, Serialize)]
/// JSON file format for function arguments.
pub struct ArgWithTypeJSON {
    #[serde(rename = "type")]
    pub arg_type: String,
    pub value: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
/// JSON file format for entry function arguments.
pub struct EntryFunctionArgumentsJSON {
    pub function_id: String,
    pub type_args: Vec<String>,
    pub args: Vec<ArgWithTypeJSON>,
}

impl EntryFunctionArgumentsJSON {
    /// Check if any arguments are struct/enum types (contain MODULE_SEPARATOR)
    pub fn has_struct_or_enum_args(&self) -> bool {
        self.args
            .iter()
            .any(|arg| arg.arg_type.contains(MODULE_SEPARATOR))
    }

    /// Convert to EntryFunctionArguments with async support for struct/enum types.
    pub async fn try_into_with_client(
        self,
        rest_client: &aptos_rest_client::Client,
    ) -> Result<EntryFunctionArguments, CliError> {
        let args = self.parse_arguments(rest_client).await?;

        Ok(EntryFunctionArguments {
            function_id: Some(MemberId::from_str(&self.function_id)?),
            type_arg_vec: TypeArgVec::try_from(&self.type_args)?,
            arg_vec: ArgWithTypeVec { args },
            json_file: None,
        })
    }

    /// Parse all arguments from JSON to ArgWithType.
    async fn parse_arguments(
        &self,
        rest_client: &aptos_rest_client::Client,
    ) -> Result<Vec<ArgWithType>, CliError> {
        use crate::struct_arg_parser::StructArgParser;

        let parser = StructArgParser::new(rest_client.clone());
        let mut args = vec![];

        for arg_json in &self.args {
            let parsed_arg = Self::parse_single_argument(&parser, arg_json).await?;
            args.push(parsed_arg);
        }

        Ok(args)
    }

    /// Parse Option<T> in legacy vector format: [] for None, [value] for Some.
    async fn parse_option_vector_format(
        parser: &crate::struct_arg_parser::StructArgParser,
        struct_tag: &StructTag,
        array: &[serde_json::Value],
    ) -> Result<ArgWithType, CliError> {
        use crate::struct_arg_parser::StructArgParser;

        let (variant, fields) = StructArgParser::convert_option_array_to_enum_format(array)?;

        let bcs_bytes = parser
            .construct_enum_argument(struct_tag, variant, &fields, 0)
            .await?;

        Ok(ArgWithType {
            _ty: FunctionArgType::Enum {
                type_tag: struct_tag.clone(),
                variant: variant.to_string(),
            },
            _vector_depth: 0,
            arg: bcs_bytes,
        })
    }

    /// Parse a single argument from JSON to ArgWithType.
    async fn parse_single_argument(
        parser: &crate::struct_arg_parser::StructArgParser,
        arg_json: &ArgWithTypeJSON,
    ) -> Result<ArgWithType, CliError> {
        if arg_json.arg_type.contains(MODULE_SEPARATOR) {
            // Type contains "::" - could be struct/enum or a vector containing struct/enum
            let type_tag = TypeTag::from_str(&arg_json.arg_type)
                .map_err(|e| CliError::UnableToParse("type", e.to_string()))?;

            match type_tag {
                TypeTag::Struct(boxed_struct_tag) => {
                    let struct_tag = *boxed_struct_tag;
                    let is_option = struct_tag.is_option();

                    if is_option && arg_json.value.is_array() {
                        // Option<T> with legacy vector format
                        let array = arg_json.value.as_array().ok_or_else(|| {
                            CliError::CommandArgumentError(format!(
                                "Expected array value for Option type, got: {}",
                                arg_json.value
                            ))
                        })?;
                        Self::parse_option_vector_format(parser, &struct_tag, array).await
                    } else if let Some(obj) = arg_json.value.as_object() {
                        // Value is an object - could be enum variant or struct
                        if obj.len() == 1 {
                            let (potential_variant_name, variant_fields_value) =
                                obj.iter().next().unwrap();

                            if let Some(fields_obj) = variant_fields_value.as_object() {
                                let is_enum = parser.is_enum_type(&struct_tag).await?;

                                if is_enum {
                                    let bcs_bytes = parser
                                        .construct_enum_argument(
                                            &struct_tag,
                                            potential_variant_name,
                                            fields_obj,
                                            0,
                                        )
                                        .await?;

                                    return Ok(ArgWithType {
                                        _ty: FunctionArgType::Enum {
                                            type_tag: struct_tag.clone(),
                                            variant: potential_variant_name.to_string(),
                                        },
                                        _vector_depth: 0,
                                        arg: bcs_bytes,
                                    });
                                }
                                // Not an enum - fall through to struct parsing
                            }
                        }

                        // Parse as struct
                        let bcs_bytes = parser
                            .construct_struct_argument(&struct_tag, obj, 0)
                            .await?;

                        Ok(ArgWithType {
                            _ty: FunctionArgType::Struct {
                                type_tag: struct_tag.clone(),
                            },
                            _vector_depth: 0,
                            arg: bcs_bytes,
                        })
                    } else {
                        // Delegate to parse_value_by_type for framework types and others
                        let move_struct_tag = MoveStructTag::from(&struct_tag);
                        let move_type = MoveType::Struct(move_struct_tag);
                        let bcs_bytes = parser
                            .parse_value_by_type(&move_type, &arg_json.value, 0)
                            .await?;

                        Ok(ArgWithType {
                            _ty: FunctionArgType::Struct {
                                type_tag: struct_tag.clone(),
                            },
                            _vector_depth: 0,
                            arg: bcs_bytes,
                        })
                    }
                },
                _ => {
                    // Not a struct - vector of structs or similar
                    let move_type = MoveType::from(&type_tag);
                    let bcs_bytes = parser
                        .parse_value_by_type(&move_type, &arg_json.value, 0)
                        .await?;

                    let vector_depth = Self::count_vector_depth(&type_tag);
                    let base_type = Self::get_base_function_arg_type(&type_tag)?;

                    Ok(ArgWithType {
                        _ty: base_type,
                        _vector_depth: vector_depth,
                        arg: bcs_bytes,
                    })
                },
            }
        } else {
            // Parse as primitive type (no "::")
            let function_arg_type = FunctionArgType::from_str(&arg_json.arg_type)?;
            function_arg_type.parse_arg_json(&arg_json.value)
        }
    }

    /// Count vector nesting depth for a TypeTag.
    fn count_vector_depth(type_tag: &TypeTag) -> u8 {
        match type_tag {
            TypeTag::Vector(inner) => 1 + Self::count_vector_depth(inner),
            _ => 0,
        }
    }

    /// Extract the base FunctionArgType from a TypeTag (unwrapping vectors).
    fn get_base_function_arg_type(type_tag: &TypeTag) -> CliTypedResult<FunctionArgType> {
        match type_tag {
            TypeTag::Vector(inner) => Self::get_base_function_arg_type(inner),
            TypeTag::Bool => Ok(FunctionArgType::Bool),
            TypeTag::U8 => Ok(FunctionArgType::U8),
            TypeTag::U16 => Ok(FunctionArgType::U16),
            TypeTag::U32 => Ok(FunctionArgType::U32),
            TypeTag::U64 => Ok(FunctionArgType::U64),
            TypeTag::U128 => Ok(FunctionArgType::U128),
            TypeTag::U256 => Ok(FunctionArgType::U256),
            TypeTag::Address => Ok(FunctionArgType::Address),
            TypeTag::Signer => Err(CliError::CommandArgumentError(
                "Signer type not allowed in transaction arguments".to_string(),
            )),
            TypeTag::Struct(struct_tag) => Ok(FunctionArgType::Struct {
                type_tag: *struct_tag.clone(),
            }),
            TypeTag::I8 => Ok(FunctionArgType::I8),
            TypeTag::I16 => Ok(FunctionArgType::I16),
            TypeTag::I32 => Ok(FunctionArgType::I32),
            TypeTag::I64 => Ok(FunctionArgType::I64),
            TypeTag::I128 => Ok(FunctionArgType::I128),
            TypeTag::I256 => Ok(FunctionArgType::I256),
            TypeTag::Function(_) => Err(CliError::CommandArgumentError(
                "Function types not allowed in transaction arguments".to_string(),
            )),
        }
    }
}

impl TryInto<EntryFunctionArguments> for EntryFunctionArgumentsJSON {
    type Error = CliError;

    fn try_into(self) -> Result<EntryFunctionArguments, Self::Error> {
        // Check if there are any struct/enum arguments
        if self.has_struct_or_enum_args() {
            return Err(CliError::CommandArgumentError(
                "Struct and enum arguments require async processing with REST client. \
                 Use the async method try_into_with_client() instead."
                    .to_string(),
            ));
        }

        Ok(EntryFunctionArguments {
            function_id: Some(MemberId::from_str(&self.function_id)?),
            type_arg_vec: TypeArgVec::try_from(&self.type_args)?,
            arg_vec: ArgWithTypeVec::try_from(&self.args)?,
            json_file: None,
        })
    }
}

#[derive(Deserialize)]
/// JSON file format for script function arguments.
struct ScriptFunctionArgumentsJSON {
    type_args: Vec<String>,
    args: Vec<ArgWithTypeJSON>,
}

impl TryInto<ScriptFunctionArguments> for ScriptFunctionArgumentsJSON {
    type Error = CliError;

    fn try_into(self) -> Result<ScriptFunctionArguments, Self::Error> {
        Ok(ScriptFunctionArguments {
            type_arg_vec: TypeArgVec::try_from(&self.type_args)?,
            arg_vec: ArgWithTypeVec::try_from(&self.args)?,
            json_file: None,
        })
    }
}

#[derive(Parser)]
pub struct OverrideSizeCheckOption {
    /// Whether to override the check for maximal size of published data
    ///
    /// This won't bypass on chain checks, so if you are not allowed to go over the size check, it
    /// will still be blocked from publishing.
    #[clap(long)]
    pub override_size_check: bool,
}

#[derive(Parser)]
pub struct LargePackagesModuleOption {
    /// Address of the `large_packages` move module for chunked publishing
    ///
    /// By default, on the module is published at `0x0e1ca3011bdd07246d4d16d909dbb2d6953a86c4735d5acf5865d962c630cce7`
    /// on Testnet and Mainnet, and `0x7` on localnest/devnet.
    /// On any custom network where neither is used, you will need to first publish it from the framework
    /// under move-examples/large_packages.
    #[clap(long, value_parser = load_account_arg)]
    pub large_packages_module_address: Option<AccountAddress>,
}

impl LargePackagesModuleOption {
    pub async fn large_packages_module_address(
        &self,
        txn_options: &TransactionOptions,
    ) -> Result<AccountAddress, CliError> {
        if let Some(address) = self.large_packages_module_address {
            return Ok(address);
        }

        let chain_id = match &txn_options.session {
            None => {
                let client = txn_options.rest_client()?;
                ChainId::new(client.get_ledger_information().await?.inner().chain_id)
            },
            Some(session_path) => {
                let sess = Session::load(session_path)?;
                sess.state_store().get_chain_id()?
            },
        };

        AccountAddress::from_str_strict(default_large_packages_module_address(&chain_id)).map_err(
            |err| CliError::UnableToParse("Default Large Package Module Address", err.to_string()),
        )
    }
}

#[derive(Parser)]
pub struct ChunkedPublishOption {
    /// Whether to publish a package in a chunked mode. This may require more than one transaction
    /// for publishing the Move package.
    ///
    /// Use this option for publishing large packages exceeding `MAX_PUBLISH_PACKAGE_SIZE`.
    #[clap(long)]
    pub chunked_publish: bool,

    #[clap(flatten)]
    pub large_packages_module: LargePackagesModuleOption,

    /// Size of the code chunk in bytes for splitting bytecode and metadata of large packages
    ///
    /// By default, the chunk size is set to `CHUNK_SIZE_IN_BYTES`. A smaller chunk size will result
    /// in more transactions required to publish a package, while a larger chunk size might cause
    /// transaction to fail due to exceeding the execution gas limit.
    #[clap(long, default_value_t = CHUNK_SIZE_IN_BYTES)]
    pub chunk_size: usize,
}
