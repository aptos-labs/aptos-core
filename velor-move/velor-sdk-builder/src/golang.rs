// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common;
use velor_types::transaction::{
    ArgumentABI, EntryABI, EntryFunctionABI, TransactionScriptABI, TypeArgumentABI,
};
use heck::ToUpperCamelCase;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use once_cell::sync::Lazy;
use serde_generate::{
    golang,
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig,
};
use std::{
    collections::BTreeMap,
    io,
    io::{Result, Write},
    path::PathBuf,
    str::FromStr,
};

/// Output transaction builders and decoders in Go for the given ABIs.
pub fn output(
    out: &mut dyn Write,
    serde_module_path: Option<String>,
    velor_module_path: Option<String>,
    package_name: String,
    abis: &[EntryABI],
) -> Result<()> {
    let mut emitter = GoEmitter {
        out: IndentedWriter::new(out, IndentConfig::Tab),
        serde_module_path,
        velor_module_path,
        package_name,
    };

    // Some functions have complex types which are not currently supported in bcs or in this
    // generator. Disable those functions for now.
    let abis_vec = abis
        .iter()
        .filter(|abi| {
            if let EntryABI::EntryFunction(sf) = abi {
                sf.module_name().name().as_str() != "code"
                    && sf.name() != "publish_package_txn"
                    && sf.module_name().name().as_str() != "velor_account"
                    && sf.name() != "batch_transfer"
                    && sf.module_name().name().as_str() != "velor_account"
                    && sf.name() != "batch_transfer_coins"
            } else {
                true
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    let abis = abis_vec.as_slice();
    emitter.output_script_call_enum_with_imports(abis)?;

    emitter.output_encode_method(abis)?;
    emitter.output_transaction_script_decode_method()?;
    emitter.output_entry_function_decode_method()?;

    for abi in abis {
        match abi {
            EntryABI::TransactionScript(abi) => {
                emitter.output_transaction_script_encoder_function(abi)?
            },
            EntryABI::EntryFunction(abi) => emitter.output_entry_function_encoder_function(abi)?,
        };
    }

    for abi in abis {
        match abi {
            EntryABI::TransactionScript(abi) => {
                emitter.output_transaction_script_decoder_function(abi)?
            },
            EntryABI::EntryFunction(abi) => emitter.output_entry_function_decoder_function(abi)?,
        };
    }

    for abi in abis {
        emitter.output_code_constant(abi)?;
    }
    emitter.output_transaction_script_decoder_map(&common::transaction_script_abis(abis))?;
    emitter.output_entry_function_decoder_map(&common::entry_function_abis(abis))?;

    emitter.output_encoding_helpers(abis)?;
    emitter.output_decoding_helpers(&common::filter_transaction_scripts(abis))?;

    Ok(())
}

/// Shared state for the Go code generator.
struct GoEmitter<T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Go module path for Serde runtime packages
    /// `None` to use the default path.
    serde_module_path: Option<String>,
    /// Go module path for Velor types.
    /// `None` to use an empty path.
    velor_module_path: Option<String>,
    /// Name of the package owning the generated definitions (e.g. "my_package")
    package_name: String,
}

impl<T> GoEmitter<T>
where
    T: Write,
{
    fn output_script_call_enum_with_imports(&mut self, abis: &[EntryABI]) -> Result<()> {
        let velor_types_package = match &self.velor_module_path {
            Some(path) => format!("{}/velortypes", path),
            None => "velortypes".into(),
        };
        let mut external_definitions =
            crate::common::get_external_definitions(&velor_types_package);
        // We need BCS for argument encoding and decoding
        external_definitions.insert(
            "github.com/velor-chain/serde-reflection/serde-generate/runtime/golang/bcs".to_string(),
            Vec::new(),
        );
        // Add standard imports
        external_definitions.insert("fmt".to_string(), Vec::new());

        let (transaction_script_abis, entry_fun_abis): (Vec<_>, Vec<_>) = abis
            .iter()
            .cloned()
            .partition(|abi| abi.is_transaction_script_abi());

        // Generate `ScriptCall` enums for all old-style transaction scripts
        let mut script_registry: BTreeMap<_, _> = vec![(
            "ScriptCall".to_string(),
            crate::common::make_abi_enum_container(transaction_script_abis.as_slice()),
        )]
        .into_iter()
        .collect();

        // Generate `EntryFunctionCall` enums for all new transaction scripts
        let mut entry_function_registry: BTreeMap<_, _> = vec![(
            "EntryFunctionCall".to_string(),
            crate::common::make_abi_enum_container(entry_fun_abis.as_slice()),
        )]
        .into_iter()
        .collect();

        script_registry.append(&mut entry_function_registry);

        let mut comments: BTreeMap<_, _> = abis
            .iter()
            .map(|abi| {
                (
                    vec![
                        self.package_name.to_string(),
                        if abi.is_transaction_script_abi() {
                            "ScriptCall".to_string()
                        } else {
                            "EntryFunctionCall".to_string()
                        },
                        abi.name().to_upper_camel_case(),
                    ],
                    crate::common::prepare_doc_string(abi.doc()),
                )
            })
            .collect();

        comments.insert(
            vec![self.package_name.to_string(), "ScriptCall".to_string()],
            "Structured representation of a call into a known Move transaction script (legacy)."
                .into(),
        );

        comments.insert(
            vec![
                self.package_name.to_string(),
                "EntryFunctionCall".to_string(),
            ],
            "Structured representation of a call into a known Move entry function.".into(),
        );

        let config = CodeGeneratorConfig::new(self.package_name.to_string())
            .with_comments(comments)
            .with_external_definitions(external_definitions)
            .with_serialization(false);
        let mut generator = golang::CodeGenerator::new(&config);
        if let Some(path) = &self.serde_module_path {
            generator = generator.with_serde_module_path(path.clone());
        }
        generator
            .output(&mut self.out, &script_registry)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)))?;
        Ok(())
    }

    fn output_encode_method(&mut self, abis: &[EntryABI]) -> Result<()> {
        let (transaction_script_abis, entry_fun_abis): (Vec<_>, Vec<_>) = abis
            .iter()
            .cloned()
            .partition(|abi| abi.is_transaction_script_abi());

        if !transaction_script_abis.is_empty() {
            writeln!(
                self.out,
                r#"
// Build an Velor `Script` from a structured object `ScriptCall`.
func EncodeScript(call ScriptCall) velortypes.Script {{"#
            )?;
            self.out.indent();
            writeln!(self.out, "switch call := call.(type) {{")?;
            for abi in transaction_script_abis {
                if let EntryABI::TransactionScript(abi) = abi {
                    let params = std::iter::empty()
                        .chain(abi.ty_args().iter().map(TypeArgumentABI::name))
                        .chain(abi.args().iter().map(ArgumentABI::name))
                        .map(|name| format!("call.{}", name.to_upper_camel_case()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(
                        self.out,
                        r#"case *ScriptCall__{0}:
                return Encode{0}({1})"#,
                        abi.name().to_upper_camel_case(),
                        params,
                    )?;
                }
            }
            writeln!(self.out, "}}")?;
            writeln!(self.out, "panic(\"unreachable\")")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;
        }

        if !entry_fun_abis.is_empty() {
            writeln!(
                self.out,
                r#"
// Build an Velor `TransactionPayload` from a structured object `EntryFunctionCall`.
func EncodeEntryFunction(call EntryFunctionCall) velortypes.TransactionPayload {{"#
            )?;
            self.out.indent();
            writeln!(self.out, "switch call := call.(type) {{")?;
            for abi in entry_fun_abis {
                if let EntryABI::EntryFunction(abi) = abi {
                    let params = std::iter::empty()
                        .chain(abi.ty_args().iter().map(TypeArgumentABI::name))
                        .chain(abi.args().iter().map(ArgumentABI::name))
                        .map(|name| format!("call.{}", name.to_upper_camel_case()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(
                        self.out,
                        r#"case *EntryFunctionCall__{0}{1}:
                return Encode{0}{1}({2})"#,
                        abi.module_name().name().to_string().to_upper_camel_case(),
                        abi.name().to_upper_camel_case(),
                        params,
                    )?;
                }
            }
            writeln!(self.out, "}}")?;
            writeln!(self.out, "panic(\"unreachable\")")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;
        }
        Ok(())
    }

    fn output_transaction_script_decode_method(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"
// Try to recognize an Velor `Script` and convert it into a structured object `ScriptCall`.
func DecodeScript(script *velortypes.Script) (ScriptCall, error) {{
	if helper := script_decoder_map[string(script.Code)]; helper != nil {{
		val, err := helper(script)
                return val, err
	}} else {{
		return nil, fmt.Errorf("Unknown script bytecode: %s", string(script.Code))
	}}
}}"#
        )
    }

    fn output_entry_function_decode_method(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"
// Try to recognize an Velor `TransactionPayload` and convert it into a structured object `EntryFunctionCall`.
func DecodeEntryFunctionPayload(script velortypes.TransactionPayload) (EntryFunctionCall, error) {{
    switch script := script.(type) {{
        case *velortypes.TransactionPayload__EntryFunction:
            if helper := entry_function_decoder_map[string(script.Value.Module.Name) + "_" + string(script.Value.Function)]; helper != nil {{
                    val, err := helper(script)
                    return val, err
            }} else {{
                    return nil, fmt.Errorf("Unknown entry function: %s::%s", script.Value.Module.Name, script.Value.Function)
            }}
        default:
                return nil, fmt.Errorf("Unknown transaction payload encountered when decoding")
    }}
}}"#
        )
    }

    fn output_transaction_script_encoder_function(
        &mut self,
        abi: &TransactionScriptABI,
    ) -> Result<()> {
        writeln!(
            self.out,
            "\n{}\nfunc Encode{}({}) velortypes.Script {{",
            Self::quote_doc(abi.doc()),
            abi.name().to_upper_camel_case(),
            [
                Self::quote_type_parameters(abi.ty_args()),
                Self::quote_parameters(abi.args()),
            ]
            .concat()
            .join(", ")
        )?;
        self.out.indent();
        writeln!(
            self.out,
            r#"return velortypes.Script {{
	Code: append([]byte(nil), {}_code...),
	TyArgs: []velortypes.TypeTag{{{}}},
	Args: []velortypes.TransactionArgument{{{}}},
}}"#,
            abi.name(),
            Self::quote_type_arguments(abi.ty_args()),
            Self::quote_arguments_for_script(abi.args()),
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_entry_function_encoder_function(&mut self, abi: &EntryFunctionABI) -> Result<()> {
        writeln!(
            self.out,
            "\n{}\nfunc Encode{}{}({}) velortypes.TransactionPayload {{",
            Self::quote_doc(abi.doc()),
            abi.module_name().name().to_string().to_upper_camel_case(),
            abi.name().to_upper_camel_case(),
            [
                Self::quote_type_parameters(abi.ty_args()),
                Self::quote_parameters(abi.args()),
            ]
            .concat()
            .join(", ")
        )?;
        self.out.indent();
        writeln!(
            self.out,
            r#"return &velortypes.TransactionPayload__EntryFunction {{
            velortypes.EntryFunction {{
                Module: {},
                Function: {},
                TyArgs: []velortypes.TypeTag{{{}}},
                Args: [][]byte{{{}}},
    }},
}}"#,
            Self::quote_module_id(abi.module_name()),
            Self::quote_identifier(abi.name()),
            Self::quote_type_arguments(abi.ty_args()),
            Self::quote_arguments(abi.args()),
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_transaction_script_decoder_function(
        &mut self,
        abi: &TransactionScriptABI,
    ) -> Result<()> {
        writeln!(
            self.out,
            "\nfunc decode_{}(script *velortypes.Script) (ScriptCall, error) {{",
            abi.name(),
        )?;
        self.out.indent();
        writeln!(
            self.out,
            "if len(script.TyArgs) < {0} {{ return nil, fmt.Errorf(\"Was expecting {0} type arguments\") }}",
            abi.ty_args().len(),
        )?;
        writeln!(
            self.out,
            "if len(script.Args) < {0} {{ return nil, fmt.Errorf(\"Was expecting {0} regular arguments\") }}",
            abi.args().len(),
        )?;
        writeln!(
            self.out,
            "var call ScriptCall__{0}",
            abi.name().to_upper_camel_case(),
        )?;
        for (index, ty_arg) in abi.ty_args().iter().enumerate() {
            writeln!(
                self.out,
                "call.{} = script.TyArgs[{}]",
                ty_arg.name().to_upper_camel_case(),
                index,
            )?;
        }
        for (index, arg) in abi.args().iter().enumerate() {
            writeln!(
                self.out,
                r#"if val, err := decode_{}_argument(script.Args[{}]); err == nil {{
	call.{} = val
}} else {{
	return nil, err
}}
"#,
                common::mangle_type(arg.type_tag()),
                index,
                arg.name().to_upper_camel_case(),
            )?;
        }
        writeln!(self.out, "return &call, nil")?;
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_entry_function_decoder_function(&mut self, abi: &EntryFunctionABI) -> Result<()> {
        writeln!(
            self.out,
            "\nfunc decode_{}_{}(script velortypes.TransactionPayload) (EntryFunctionCall, error) {{",
            abi.module_name().name(),
            abi.name(),
        )?;
        self.out.indent();
        writeln!(self.out, "switch script := interface{{}}(script).(type) {{")?;
        self.out.indent();
        writeln!(
            self.out,
            "case *velortypes.TransactionPayload__EntryFunction:"
        )?;
        self.out.indent();
        writeln!(
            self.out,
            "if len(script.Value.TyArgs) < {0} {{ return nil, fmt.Errorf(\"Was expecting {0} type arguments\") }}",
            abi.ty_args().len(),
        )?;
        writeln!(
            self.out,
            "if len(script.Value.Args) < {0} {{ return nil, fmt.Errorf(\"Was expecting {0} regular arguments\") }}",
            abi.args().len(),
        )?;
        writeln!(
            self.out,
            "var call EntryFunctionCall__{0}{1}",
            abi.module_name().name().to_string().to_upper_camel_case(),
            abi.name().to_upper_camel_case(),
        )?;
        for (index, ty_arg) in abi.ty_args().iter().enumerate() {
            writeln!(
                self.out,
                "call.{} = script.Value.TyArgs[{}]",
                ty_arg.name().to_upper_camel_case(),
                index,
            )?;
        }
        for (index, arg) in abi.args().iter().enumerate() {
            let decoding = match Self::bcs_primitive_type_name(arg.type_tag()) {
                None => {
                    let vec_string_tag =
                        TypeTag::from_str("vector<0x1::string::String>").map_err(|err| {
                            io::Error::new(
                                io::ErrorKind::Other,
                                format!(
                                    "Failed to construct a type tag for vector of strings: {:?}",
                                    err
                                ),
                            )
                        })?;
                    if arg.type_tag() == &vec_string_tag {
                        format!(
                            "bcs.NewDeserializer(script.Value.Args[{}]).DeserializeVecBytes()",
                            index,
                        )
                    } else {
                        let quoted_type = Self::quote_type(arg.type_tag());
                        let splits: Vec<_> = quoted_type.rsplitn(2, '.').collect();
                        let (left, right) = if splits.len() == 2 {
                            (splits[1], splits[0])
                        } else {
                            (splits[0], "")
                        };
                        format!(
                            "{}.BcsDeserialize{}(script.Value.Args[{}])",
                            left, right, index,
                        )
                    }
                },
                Some(type_name) => match type_name {
                    "VecAddress" => format!(
                        r#"
var val {0}
if err := deserializer.IncreaseContainerDepth(); err != nil {{
    return ({0})(val), err
}}
length, err := deserializer.DeserializeI8()
if err != nil {{
    return nil, err
}}
var tmp {0}
for i := 0; i < int(length); i++ {{
    if obj, err := deserialize_array32_u8_array(deserializer); err == nil {{
        tmp = obj
    }} else {{
        return (({0})(val)), err
    }}
    val = append(val, tmp)
}}

deserializer.DecreaseContainerDepth()
call.{1} = val
"#,
                        Self::quote_type(arg.type_tag()),
                        arg.name().to_upper_camel_case()
                    ),
                    _ => format!(
                        "bcs.NewDeserializer(script.Value.Args[{}]).Deserialize{}()",
                        index, type_name
                    ),
                },
            };
            if Self::bcs_primitive_type_name(arg.type_tag()) != Some("VecAddress") {
                writeln!(
                    self.out,
                    r#"
if val, err := {}; err == nil {{
	call.{} = val
}} else {{
	return nil, err
}}
"#,
                    decoding,
                    arg.name().to_upper_camel_case(),
                )?;
            }
        }
        writeln!(self.out, "return &call, nil")?;
        self.out.unindent();
        writeln!(
            self.out,
            r#"default:
    return nil, fmt.Errorf("Unexpected TransactionPayload encountered when decoding a entry function")"#
        )?;

        self.out.unindent();
        writeln!(self.out, "}}")?;
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_transaction_script_decoder_map(
        &mut self,
        abis: &[TransactionScriptABI],
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
var script_decoder_map = map[string]func(*velortypes.Script) (ScriptCall, error) {{"#
        )?;
        self.out.indent();
        for abi in abis {
            writeln!(self.out, "string({0}_code): decode_{0},", abi.name(),)?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_entry_function_decoder_map(&mut self, abis: &[EntryFunctionABI]) -> Result<()> {
        writeln!(
            self.out,
            r#"
var entry_function_decoder_map = map[string]func(velortypes.TransactionPayload) (EntryFunctionCall, error) {{"#
        )?;
        self.out.indent();
        for abi in abis {
            writeln!(
                self.out,
                "\"{0}_{1}\": decode_{0}_{1},",
                abi.module_name().name(),
                abi.name(),
            )?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_encoding_helpers(&mut self, abis: &[EntryABI]) -> Result<()> {
        let required_types = common::get_required_helper_types(abis);
        for required_type in required_types {
            self.output_encoding_helper(required_type)?;
        }
        Ok(())
    }

    fn output_encoding_helper(&mut self, type_tag: &TypeTag) -> Result<()> {
        let encoding = match Self::bcs_primitive_type_name(type_tag) {
            None => {
                if "vecstring".eq(&common::mangle_type(type_tag)) {
                    "return encode_vecbytes_argument(arg)".to_string()
                } else {
                    format!(
                        r#"
    if val, err := arg.BcsSerialize(); err == nil {{
        return val;
    }}
    panic("Unable to serialize argument of type {}");
    "#,
                        common::mangle_type(type_tag)
                    )
                }
            },
            Some(type_name) => match type_name {
                "VecAddress" => r#"
    obj := []byte{ }
	obj = append(obj, byte(len(arg)))
	for _, val := range arg {{
		valBytes := encode_address_argument(val)
		obj = append(obj, valBytes...)
	}}
	return obj"#
                    .into(),
                _ => format!(
                    r#"
    s := bcs.NewSerializer();
    if err := s.Serialize{}(arg); err == nil {{
        return s.GetBytes();
    }}
    panic("Unable to serialize argument of type {}");
    "#,
                    type_name,
                    common::mangle_type(type_tag)
                ),
            },
        };
        writeln!(
            self.out,
            r#"
func encode_{}_argument(arg {}) []byte {{
    {}
}}
"#,
            common::mangle_type(type_tag),
            Self::quote_type(type_tag),
            encoding
        )
    }

    fn output_decoding_helpers(&mut self, abis: &[EntryABI]) -> Result<()> {
        let required_types = common::get_required_helper_types(abis);
        for required_type in required_types {
            self.output_decoding_helper(required_type)?;
        }
        Ok(())
    }

    fn output_decoding_helper(&mut self, type_tag: &TypeTag) -> Result<()> {
        use TypeTag::*;
        let default_stmt = format!("value = {}(*arg)", Self::quote_type(type_tag));
        let (constructor, stmt) = match type_tag {
            Bool => ("Bool", default_stmt),
            U8 => ("U8", default_stmt),
            U16 => ("U16", default_stmt),
            U32 => ("U32", default_stmt),
            U64 => ("U64", default_stmt),
            U128 => ("U128", default_stmt),
            U256 => ("U256", default_stmt),
            Address => ("Address", "value = arg.Value".into()),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => ("U8Vector", default_stmt),
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer | Function(..) => common::type_not_allowed(type_tag),
        };
        writeln!(
            self.out,
            r#"
func decode_{0}_argument(arg velortypes.TransactionArgument) (value {1}, err error) {{
	if arg, ok := arg.(*velortypes.TransactionArgument__{2}); ok {{
		{3}
	}} else {{
		err = fmt.Errorf("Was expecting a {2} argument")
	}}
	return
}}
"#,
            common::mangle_type(type_tag),
            Self::quote_type(type_tag),
            constructor,
            stmt,
        )
    }

    fn output_code_constant(&mut self, abi: &EntryABI) -> Result<()> {
        if let EntryABI::TransactionScript(abi) = abi {
            writeln!(
                self.out,
                "\nvar {}_code = []byte {{{}}};",
                abi.name(),
                abi.code()
                    .iter()
                    .map(|x| format!("{}", x))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        Ok(())
    }

    fn quote_identifier(ident: &str) -> String {
        format!("\"{}\"", ident)
    }

    fn quote_address(address: &AccountAddress) -> String {
        format!(
            "[32]uint8{{ {} }}",
            address
                .to_vec()
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn quote_module_id(module_id: &ModuleId) -> String {
        format!(
            "velortypes.ModuleId {{ Address: {}, Name: {} }}",
            Self::quote_address(module_id.address()),
            Self::quote_identifier(module_id.name().as_str()),
        )
    }

    fn quote_doc(doc: &str) -> String {
        let doc = crate::common::prepare_doc_string(doc);
        textwrap::indent(&doc, "// ").replace("\n\n", "\n//\n")
    }

    fn quote_type_parameters(ty_args: &[TypeArgumentABI]) -> Vec<String> {
        ty_args
            .iter()
            .map(|ty_arg| format!("{} velortypes.TypeTag", ty_arg.name()))
            .collect()
    }

    fn quote_parameters(args: &[ArgumentABI]) -> Vec<String> {
        args.iter()
            .map(|arg| format!("{} {}", arg.name(), Self::quote_type(arg.type_tag())))
            .collect()
    }

    fn quote_type_arguments(ty_args: &[TypeArgumentABI]) -> String {
        ty_args
            .iter()
            .map(|ty_arg| ty_arg.name().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_arguments(args: &[ArgumentABI]) -> String {
        args.iter()
            .map(|arg| Self::quote_transaction_argument(arg.type_tag(), arg.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_arguments_for_script(args: &[ArgumentABI]) -> String {
        args.iter()
            .map(|arg| Self::quote_transaction_argument_for_script(arg.type_tag(), arg.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_type(type_tag: &TypeTag) -> String {
        use TypeTag::*;
        let str_tag: Lazy<StructTag> =
            Lazy::new(|| StructTag::from_str("0x1::string::String").unwrap());
        match type_tag {
            Bool => "bool".into(),
            U8 => "uint8".into(),
            U16 => "uint16".into(),
            U32 => "uint32".into(),
            U64 => "uint64".into(),
            U128 => "serde.Uint128".into(),
            U256 => unimplemented!(),
            Address => "velortypes.AccountAddress".into(),
            Vector(type_tag) => {
                format!("[]{}", Self::quote_type(type_tag))
            },
            Struct(struct_tag) => match struct_tag {
                tag if &**tag == Lazy::force(&str_tag) => "[]uint8".into(),
                _ => common::type_not_allowed(type_tag),
            },
            Signer | Function(..) => common::type_not_allowed(type_tag),
        }
    }

    fn quote_transaction_argument(type_tag: &TypeTag, name: &str) -> String {
        format!(
            "encode_{}_argument({})",
            common::mangle_type(type_tag),
            name
        )
    }

    fn quote_transaction_argument_for_script(type_tag: &TypeTag, name: &str) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => format!("(*velortypes.TransactionArgument__Bool)(&{})", name),
            U8 => format!("(*velortypes.TransactionArgument__U8)(&{})", name),
            U16 => format!("(*velortypes.TransactionArgument__U16)(&{})", name),
            U32 => format!("(*velortypes.TransactionArgument__U32)(&{})", name),
            U64 => format!("(*velortypes.TransactionArgument__U64)(&{})", name),
            U128 => format!("(*velortypes.TransactionArgument__U128)(&{})", name),
            U256 => format!("(*velortypes.TransactionArgument__U256)(&{})", name),
            Address => format!("&velortypes.TransactionArgument__Address{{{}}}", name),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => format!("(*velortypes.TransactionArgument__U8Vector)(&{})", name),
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer | Function(..) => common::type_not_allowed(type_tag),
        }
    }

    // - if a `type_tag` is a primitive type in BCS, we can call
    //   `NewSerializer().Serialize<name>(arg)` and `NewDeserializer().Deserialize<name>(arg)`
    //   to convert into and from `[]byte`.
    // - otherwise, we can use `<arg>.BcsSerialize()`, `<arg>.BcsDeserialize()` to do the work.
    fn bcs_primitive_type_name(type_tag: &TypeTag) -> Option<&'static str> {
        use TypeTag::*;
        let str_tag: Lazy<StructTag> =
            Lazy::new(|| StructTag::from_str("0x1::string::String").unwrap());
        match type_tag {
            Bool => Some("Bool"),
            U8 => Some("U8"),
            U16 => Some("U16"),
            U32 => Some("U32"),
            U64 => Some("U64"),
            U128 => Some("U128"),
            U256 => None,
            Address => None,
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => Some("Bytes"),
                Vector(type_tag) => match type_tag.as_ref() {
                    U8 => Some("VecBytes"),
                    type_tag => Self::bcs_primitive_type_name(type_tag).and(None),
                },
                Address => Some("VecAddress"),
                type_tag => Self::bcs_primitive_type_name(type_tag).and(None),
            },
            Struct(struct_tag) => match struct_tag {
                tag if &**tag == Lazy::force(&str_tag) => Some("Bytes"),
                _ => common::type_not_allowed(type_tag),
            },
            Signer | Function(..) => common::type_not_allowed(type_tag),
        }
    }
}

pub struct Installer {
    install_dir: PathBuf,
    serde_module_path: Option<String>,
    velor_module_path: Option<String>,
}

impl Installer {
    pub fn new(
        install_dir: PathBuf,
        serde_module_path: Option<String>,
        velor_module_path: Option<String>,
    ) -> Self {
        Installer {
            install_dir,
            serde_module_path,
            velor_module_path,
        }
    }
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_transaction_builders(
        &self,
        name: &str,
        abis: &[EntryABI],
    ) -> std::result::Result<(), Self::Error> {
        let dir_path = self.install_dir.join(name);
        std::fs::create_dir_all(&dir_path)?;
        let mut file = std::fs::File::create(dir_path.join("lib.go"))?;
        output(
            &mut file,
            self.serde_module_path.clone(),
            self.velor_module_path.clone(),
            name.to_string(),
            abis,
        )?;
        Ok(())
    }
}
