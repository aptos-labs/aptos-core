// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::common;
use diem_types::transaction::{
    ArgumentABI, ScriptABI, ScriptFunctionABI, TransactionScriptABI, TypeArgumentABI,
};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, TypeTag},
};
use serde_generate::{
    indent::{IndentConfig, IndentedWriter},
    swift, CodeGeneratorConfig,
};

use heck::{CamelCase, ShoutySnakeCase};
use std::{
    collections::BTreeMap,
    io::{Result, Write},
    path::PathBuf,
};

pub struct Installer {
    install_dir: PathBuf,
}

/// Shared state for the TypeScript code generator.
struct SwiftEmitter<T> {
    /// Writer.
    out: IndentedWriter<T>,
}

/// Output transaction builders and decoders in TypeScript for the given ABIs.
pub fn output(out: &mut dyn Write, abis: &[ScriptABI]) -> Result<()> {
    writeln!(out, "import DiemTypes")?;
    write_script_calls(out, abis)?;
    write_helpers(out, abis)?;
    Ok(())
}

fn write_helpers(out: &mut dyn Write, abis: &[ScriptABI]) -> Result<()> {
    let mut emitter = SwiftEmitter {
        out: IndentedWriter::new(out, IndentConfig::Space(2)),
    };
    let txn_script_abis = common::transaction_script_abis(abis);
    let script_fun_abis = common::script_function_abis(abis);
    emitter.output_preamble()?;

    for abi in &txn_script_abis {
        emitter.output_script_encoder_function(abi)?;
    }

    for abi in &txn_script_abis {
        emitter.output_script_decoder_function(abi)?;
    }

    emitter.output_script_function_encoders(&script_fun_abis)?;

    emitter.output_decoding_helpers(&common::filter_transaction_scripts(abis))?;
    emitter.output_code_constants(&txn_script_abis)?;

    Ok(())
}

fn write_script_calls(out: &mut dyn Write, abis: &[ScriptABI]) -> Result<()> {
    let txn_script_abis = common::transaction_script_abis(abis);
    let script_fun_abis = common::script_function_abis(abis);
    let external_definitions = crate::common::get_external_definitions("DiemTypes");
    let script_registry: BTreeMap<_, _> = vec![
        (
            "ScriptCall".to_string(),
            common::make_abi_enum_container(
                txn_script_abis
                    .iter()
                    .cloned()
                    .map(ScriptABI::TransactionScript)
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        ),
        (
            "ScriptFunctionCall".to_string(),
            common::make_abi_enum_container(
                script_fun_abis
                    .iter()
                    .cloned()
                    .map(ScriptABI::ScriptFunction)
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        ),
    ]
    .into_iter()
    .collect();
    let mut comments: BTreeMap<_, _> = txn_script_abis
        .iter()
        .map(|abi| {
            let paths = vec!["ScriptCall".to_string(), abi.name().to_camel_case()];
            (paths, crate::common::prepare_doc_string(abi.doc()))
        })
        .chain(script_fun_abis.iter().map(|abi| {
            let paths = vec!["ScriptFunctionCall".to_string(), abi.name().to_camel_case()];
            (paths, crate::common::prepare_doc_string(abi.doc()))
        }))
        .collect();
    comments.insert(
        vec!["ScriptCall".to_string()],
        "Structured representation of a call into a known Move script.".into(),
    );
    comments.insert(
        vec!["ScriptFunctionCall".to_string()],
        "Structured representation of a call into a known Move script function.".into(),
    );

    let config = CodeGeneratorConfig::new("DiemStdlib".to_string())
        .with_comments(comments)
        .with_external_definitions(external_definitions)
        .with_serialization(false);
    swift::CodeGenerator::new(&config)
        .output(out, &script_registry)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)))?;
    Ok(())
}

impl Installer {
    pub fn new(install_dir: PathBuf) -> Self {
        Installer { install_dir }
    }
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_transaction_builders(
        &self,
        name: &str,
        abis: &[ScriptABI],
    ) -> std::result::Result<(), Self::Error> {
        let dir_path = self.install_dir.join("Sources").join(name);
        std::fs::create_dir_all(&dir_path)?;
        let mut file = std::fs::File::create(dir_path.join("DiemStdlib.swift"))?;
        output(&mut file, abis)?;
        Ok(())
    }
}

impl<T> SwiftEmitter<T>
where
    T: Write,
{
    fn output_preamble(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"
public enum PayloadDecodingError: Error {{
    case invalidInput(issue: String)
}}
public func into_script_function(payload: DiemTypes.TransactionPayload) throws -> DiemTypes.ScriptFunction {{
    switch payload {{
        case .ScriptFunction(let script_function): return script_function
        default: throw PayloadDecodingError.invalidInput(issue: "Unexpected transaction payload")
    }}
}}
            "#
        )?;
        Ok(())
    }

    // Script functions are grouped by and namespaced by their declaring module. So a script
    // function `f` in a module `M` will be called with `M.encode_f_script_function(...)`.
    // Similarly for decode methods.
    fn output_script_function_encoders(&mut self, abis: &[ScriptFunctionABI]) -> Result<()> {
        let mut abis_by_module: BTreeMap<&ModuleId, Vec<&ScriptFunctionABI>> = BTreeMap::new();

        for abi in abis {
            let module_name = abi.module_name();
            let entry = abis_by_module.entry(module_name).or_insert_with(Vec::new);
            entry.push(abi);
        }

        for (module_name, abis) in abis_by_module {
            writeln!(
                self.out,
                "public enum {} {{",
                module_name.name().to_string().to_camel_case()
            )?;
            self.out.indent();
            for abi in abis {
                self.output_script_function_encoder_function(abi)?;
                self.output_script_function_decoder_function(abi)?;
            }
            self.out.unindent();
            writeln!(self.out, "}}")?;
        }
        Ok(())
    }

    fn output_script_encoder_function(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        self.output_comment(0, &common::prepare_doc_string(abi.doc()))?;
        writeln!(
            self.out,
            "public func encode_{}_script({}) -> DiemTypes.Script {{",
            abi.name(),
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
            "return DiemTypes.Script(code: CodeConstants.{}, ty_args: [{}], args: [{}])",
            abi.name().to_shouty_snake_case(),
            Self::quote_type_arguments(abi.ty_args()),
            Self::quote_arguments_for_script(abi.args()),
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_script_function_encoder_function(&mut self, abi: &ScriptFunctionABI) -> Result<()> {
        self.output_comment(0, &common::prepare_doc_string(abi.doc()))?;
        writeln!(
            self.out,
            "public static func encode_{}_script_function({}) throws -> DiemTypes.TransactionPayload {{",
            abi.name(),
            [
                Self::quote_type_parameters(abi.ty_args()),
                Self::quote_parameters(abi.args()),
            ]
            .concat()
            .join(", ")
        )?;
        self.out.indent();
        writeln!(self.out, "{}", Self::quote_arguments(abi.args()))?;
        writeln!(self.out,
            "return DiemTypes.TransactionPayload.ScriptFunction(DiemTypes.ScriptFunction(module: {}, function: {}, ty_args: [{}], args: [{}]))",
            Self::quote_module_id(abi.module_name()),
            Self::quote_identifier(abi.name()),
            Self::quote_type_arguments(abi.ty_args()),
            abi.args().iter().map(|arg| {
                format!("{}_serialized", arg.name())
            }).collect::<Vec<_>>().join(", ")
            )?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_script_function_decoder_function(&mut self, abi: &ScriptFunctionABI) -> Result<()> {
        writeln!(self.out, "\n public static func decode_{}_script_function(payload: DiemTypes.TransactionPayload) throws -> ScriptFunctionCall {{", abi.name())?;
        self.out.indent();
        if !abi.args().is_empty() || !abi.ty_args().is_empty() {
            writeln!(
                self.out,
                "let script = try into_script_function(payload: payload)"
            )?;
        }
        let ty_params = abi
            .ty_args()
            .iter()
            .enumerate()
            .map(|(i, ty_arg)| format!("{}: script.ty_args[{}]", ty_arg.name(), i));
        let params = abi.args().iter().map(|arg| format!("{0}: {0}", arg.name()));
        for (i, arg) in abi.args().iter().enumerate() {
            let data_access = format!("script.args[{}]", i);
            writeln!(
                self.out,
                "{}",
                Self::quote_deserialize_transaction_argument(
                    arg.type_tag(),
                    arg.name(),
                    &data_access
                )
            )?;
        }
        let params = Self::format_args(ty_params.chain(params));
        writeln!(
            self.out,
            "return ScriptFunctionCall.{}{}",
            abi.name().to_camel_case(),
            params
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_script_decoder_function(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        writeln!(
            self.out,
            "\n public func decode_{}_script(script: DiemTypes.Script) throws -> ScriptCall {{",
            abi.name()
        )?;
        self.out.indent();
        let ty_params = abi
            .ty_args()
            .iter()
            .enumerate()
            .map(|(i, ty_arg)| format!("{}: script.ty_args[{}]", ty_arg.name(), i));
        let params = abi.args().iter().enumerate().map(|(i, arg)| {
            format!(
                "{}: try decode_{}_argument(script.args[{}])",
                arg.name(),
                common::mangle_type(arg.type_tag()),
                i
            )
        });
        writeln!(
            self.out,
            "return ScriptCall.{}{}",
            abi.name().to_camel_case(),
            Self::format_args(ty_params.chain(params))
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_decoding_helpers(&mut self, abis: &[ScriptABI]) -> Result<()> {
        let required_types = common::get_required_helper_types(abis);
        for required_type in required_types {
            self.output_decoding_helper(required_type)?;
        }
        Ok(())
    }

    fn output_decoding_helper(&mut self, type_tag: &TypeTag) -> Result<()> {
        use TypeTag::*;
        let (constructor, expr) = match type_tag {
            Bool => ("Bool", "value".to_string()),
            U8 => ("U8", "value".to_string()),
            U64 => ("U64", "value".to_string()),
            U128 => ("U128", "value".to_string()),
            Address => ("Address", "value".to_string()),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => ("U8Vector", "value".to_string()),
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer => common::type_not_allowed(type_tag),
        };
        writeln!(
            self.out,
            r#"
func decode_{}_argument(_ arg: DiemTypes.TransactionArgument) throws -> {} {{
    switch arg {{
        case .{}(let value): return {}
        default: throw PayloadDecodingError.invalidInput(issue: "Unexpected transaction argument")
    }}
}}
"#,
            common::mangle_type(type_tag),
            Self::quote_type(type_tag),
            constructor,
            expr,
        )
    }

    fn output_code_constants(&mut self, abis: &[TransactionScriptABI]) -> Result<()> {
        writeln!(self.out, "struct CodeConstants {{")?;
        self.out.indent();
        for abi in abis {
            self.output_code_constant(abi)?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_code_constant(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        writeln!(
            self.out,
            "static let {}: [UInt8] = [{}]",
            abi.name().to_shouty_snake_case(),
            abi.code()
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        Ok(())
    }

    fn quote_arguments_for_script(args: &[ArgumentABI]) -> String {
        args.iter()
            .map(|arg| Self::quote_transaction_argument_for_script(arg.type_tag(), arg.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_transaction_argument_for_script(type_tag: &TypeTag, name: &str) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => format!("DiemTypes.TransactionArgument.Bool({})", name),
            U8 => format!("DiemTypes.TransactionArgument.U8({})", name),
            U64 => format!("DiemTypes.TransactionArgument.U64({})", name),
            U128 => format!("DiemTypes.TransactionArgument.U128({})", name),
            Address => format!("DiemTypes.TransactionArgument.Address({})", name),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => format!("DiemTypes.TransactionArgument.U8Vector({})", name),
                _ => common::type_not_allowed(type_tag),
            },

            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn output_comment(&mut self, indentation: usize, doc: &str) -> std::io::Result<()> {
        let prefix = " ".repeat(indentation) + "// ";
        let empty_line = "\n".to_string() + &" ".repeat(indentation) + "///\n";
        let text = textwrap::indent(doc, &prefix).replace("\n\n", &empty_line);
        write!(self.out, "\n{}\n", text)
    }

    fn quote_type_parameters(ty_args: &[TypeArgumentABI]) -> Vec<String> {
        ty_args
            .iter()
            .map(|ty_arg| format!("{}: DiemTypes.TypeTag", ty_arg.name()))
            .collect()
    }

    fn quote_parameters(args: &[ArgumentABI]) -> Vec<String> {
        args.iter()
            .map(|arg| format!("{}: {}", arg.name(), Self::quote_type(arg.type_tag())))
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
            .map(|arg| Self::quote_serialize_transaction_argument(arg.type_tag(), arg.name()))
            .collect::<Vec<_>>()
            .join("")
    }

    fn quote_type(type_tag: &TypeTag) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => "Bool".into(),
            U8 => "UInt8".into(),
            U64 => "UInt64".into(),
            U128 => "BigInt8".into(),
            Address => "DiemTypes.AccountAddress".into(),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => "[UInt8]".into(),
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn quote_deserialize_transaction_argument_type(type_tag: &TypeTag, ser_name: &str) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => format!("{}.deserialize_bool()", ser_name),
            U8 => format!("{}.deserialize_u8()", ser_name),
            U64 => format!("{}.deserialize_u64()", ser_name),
            U128 => format!("{}.deserialize_u128()", ser_name),
            Address => format!(
                "DiemTypes.AccountAddress.deserialize(deserializer: {})",
                ser_name
            ),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => format!("{}.deserialize_bytes()", ser_name),
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn quote_deserialize_transaction_argument(
        type_tag: &TypeTag,
        name: &str,
        data_access: &str,
    ) -> String {
        let deser_name = format!("{}_deserializer", name);
        format!(
            "let {} = BcsDeserializer(input: {})\n\
            let {}: {} = try {}\n",
            deser_name,
            data_access,
            name,
            Self::quote_type(type_tag),
            Self::quote_deserialize_transaction_argument_type(type_tag, &deser_name),
        )
    }

    fn quote_serialize_transaction_argument_type(
        type_tag: &TypeTag,
        ser_name: &str,
        arg_name: &str,
    ) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => format!("{}.serialize_bool(value: {})", ser_name, arg_name),
            U8 => format!("{}.serialize_u8(value: {})", ser_name, arg_name),
            U64 => format!("{}.serialize_u64(value: {})", ser_name, arg_name),
            U128 => format!("{}.serialize_u128(value: {})", ser_name, arg_name),
            Address => format!("{}.serialize(serializer: {})", arg_name, ser_name),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => format!("{}.serialize_bytes(value: {})", ser_name, arg_name),
                _ => common::type_not_allowed(type_tag),
            },
            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn quote_serialize_transaction_argument(type_tag: &TypeTag, name: &str) -> String {
        let ser_name = format!("{}_serializer", name);
        format!(
            "let {0} = BcsSerializer()\n\
            try {1}\n\
            let {2}_serialized: [UInt8] = {0}.get_bytes()\n",
            ser_name,
            Self::quote_serialize_transaction_argument_type(type_tag, &ser_name, name),
            name
        )
    }

    fn quote_module_id(module_id: &ModuleId) -> String {
        format!(
            "DiemTypes.ModuleId(address: {}, name: {})",
            Self::quote_address(module_id.address()),
            Self::quote_identifier(module_id.name().as_str())
        )
    }

    fn quote_address(address: &AccountAddress) -> String {
        format!(
            "DiemTypes.AccountAddress(value: [{}])",
            address
                .to_vec()
                .iter()
                .map(|x| format!("{}", x))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn quote_identifier(ident: &str) -> String {
        format!("DiemTypes.Identifier(value: \"{}\")", ident)
    }

    fn format_args(x: impl Iterator<Item = String>) -> String {
        let pre_args = x.collect::<Vec<_>>();
        if pre_args.is_empty() {
            "".to_string()
        } else {
            format!("({})", pre_args.join(", "))
        }
    }
}
