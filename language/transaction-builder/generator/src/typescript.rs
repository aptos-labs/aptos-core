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
    typescript, CodeGeneratorConfig,
};
use serde_reflection::ContainerFormat;

use heck::{CamelCase, MixedCase, ShoutySnakeCase};
use std::{
    collections::BTreeMap,
    io::{Result, Write},
    path::PathBuf,
};
/// Output transaction builders and decoders in TypeScript for the given ABIs.
pub fn output(out: &mut dyn Write, abis: &[ScriptABI]) -> Result<()> {
    write_script_calls(out, abis)?;
    write_helpers(out, abis)
}

fn write_stdlib_helper_interfaces(emitter: &mut TypeScriptEmitter<&mut dyn Write>) -> Result<()> {
    writeln!(
        emitter.out,
        r#"
export interface TypeTagDef {{
  type: Types;
  arrayType?: TypeTagDef;
  name?: string;
  moduleName?: string;
  address?: string;
  typeParams?: TypeTagDef[];
}}

export interface ArgDef {{
  readonly name: string;
  readonly type: TypeTagDef;
  readonly choices?: string[];
  readonly mandatory?: boolean;
}}

export interface ScriptDef {{
  readonly stdlibEncodeFunction: (...args: any[]) => DiemTypes.Script;
  readonly stdlibDecodeFunction: (script: DiemTypes.Script) => ScriptCall;
  readonly codeName: string;
  readonly description: string;
  readonly typeArgs: string[];
  readonly args: ArgDef[];
}}

export interface ScriptFunctionDef {{
  readonly stdlibEncodeFunction: (...args: any[]) => DiemTypes.TransactionPayload;
  readonly description: string;
  readonly typeArgs: string[];
  readonly args: ArgDef[];
}}

export enum Types {{
  Boolean,
  U8,
  U64,
  U128,
  Address,
  Array,
  Struct
}}
"#
    )?;

    Ok(())
}

/// Output transaction helper functions for the given ABIs.
fn write_helpers(out: &mut dyn Write, abis: &[ScriptABI]) -> Result<()> {
    let mut emitter = TypeScriptEmitter {
        out: IndentedWriter::new(out, IndentConfig::Space(2)),
    };
    let txn_script_abis = common::transaction_script_abis(abis);
    let script_fun_abis = common::script_function_abis(abis);
    emitter.output_preamble()?;
    write_stdlib_helper_interfaces(&mut emitter)?;
    writeln!(emitter.out, "\nexport class Stdlib {{")?;
    emitter.out.indent();
    writeln!(emitter.out, "private static fromHexString(hexString: string): Uint8Array {{ return new Uint8Array(hexString.match(/.{{1,2}}/g)!.map((byte) => parseInt(byte, 16)));}}")?;

    for abi in &txn_script_abis {
        emitter.output_script_encoder_function(abi)?;
    }
    for abi in &txn_script_abis {
        emitter.output_script_decoder_function(abi)?;
    }
    for abi in &script_fun_abis {
        emitter.output_script_function_encoder_function(abi)?;
    }

    for abi in &script_fun_abis {
        emitter.output_script_function_decoder_function(abi)?;
    }

    for abi in &txn_script_abis {
        emitter.output_code_constant(abi)?;
    }
    writeln!(
        emitter.out,
        "\nstatic ScriptArgs: {{[name: string]: ScriptDef}} = {{"
    )?;
    emitter.out.indent();
    for abi in &txn_script_abis {
        emitter.output_script_args_definition(abi)?;
    }
    emitter.out.unindent();
    writeln!(emitter.out, "}}")?;

    writeln!(
        emitter.out,
        "\nstatic ScriptFunctionArgs: {{[name: string]: ScriptFunctionDef}} = {{"
    )?;
    emitter.out.indent();
    for abi in &script_fun_abis {
        emitter.output_script_fun_args_definition(abi)?;
    }
    emitter.out.unindent();
    writeln!(emitter.out, "}}")?;

    emitter.out.unindent();
    writeln!(emitter.out, "\n}}\n")?;

    writeln!(emitter.out, "\nexport type ScriptDecoders = {{")?;
    emitter.out.indent();
    writeln!(emitter.out, "User: {{")?;
    emitter.out.indent();
    for abi in &txn_script_abis {
        emitter.output_script_args_callbacks(abi)?;
    }
    writeln!(
        emitter.out,
        "default: (type: keyof ScriptDecoders['User']) => void;"
    )?;
    emitter.out.unindent();
    writeln!(emitter.out, "}};")?;
    emitter.out.unindent();
    writeln!(emitter.out, "}};")
}

fn write_script_calls(out: &mut dyn Write, abis: &[ScriptABI]) -> Result<()> {
    let txn_script_abis = common::transaction_script_abis(abis);
    let script_fun_abis = common::script_function_abis(abis);
    let external_definitions = crate::common::get_external_definitions("diemTypes");
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

    let config = CodeGeneratorConfig::new("StdLib".to_string())
        .with_comments(comments)
        .with_external_definitions(external_definitions)
        .with_serialization(false);
    typescript::CodeGenerator::new(&config)
        .output(out, &script_registry)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)))?;
    Ok(())
}

/// Shared state for the TypeScript code generator.
struct TypeScriptEmitter<T> {
    /// Writer.
    out: IndentedWriter<T>,
}

impl<T> TypeScriptEmitter<T>
where
    T: Write,
{
    fn output_preamble(&mut self) -> Result<()> {
        Ok(())
    }

    fn output_script_function_encoder_function(&mut self, abi: &ScriptFunctionABI) -> Result<()> {
        writeln!(
            self.out,
            "\n{}static encode{}ScriptFunction({}): DiemTypes.TransactionPayload {{",
            Self::quote_doc(abi.doc()),
            abi.name().to_camel_case(),
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
            r#"const tyArgs: Seq<DiemTypes.TypeTag> = [{}];
{}const args: Seq<bytes> = [{}];
const module_id: DiemTypes.ModuleId = {};
const function_name: DiemTypes.Identifier = {};
const script = new DiemTypes.ScriptFunction(module_id, function_name, tyArgs, args);
return new DiemTypes.TransactionPayloadVariantScriptFunction(script);"#,
            Self::quote_type_arguments(abi.ty_args()),
            Self::quote_serialize_arguments(abi.args()),
            abi.args()
                .iter()
                .map(|arg| format!("{}_serialized", arg.name()))
                .collect::<Vec<_>>()
                .join(", "),
            Self::quote_module_id(abi.module_name()),
            Self::quote_identifier(abi.name()),
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn quote_serialize_arguments(args: &[ArgumentABI]) -> String {
        args.iter()
            .map(|arg| Self::quote_serialize_transaction_argument(arg.type_tag(), arg.name()))
            .collect::<Vec<_>>()
            .join("")
    }

    fn quote_module_id(module_id: &ModuleId) -> String {
        format!(
            "new DiemTypes.ModuleId({}, {})",
            Self::quote_address(module_id.address()),
            Self::quote_identifier(module_id.name().as_str())
        )
    }

    fn quote_address(address: &AccountAddress) -> String {
        format!(
            "new DiemTypes.AccountAddress([{}])",
            address
                .to_vec()
                .iter()
                .map(|x| format!("[{}]", x))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn quote_identifier(ident: &str) -> String {
        format!("new DiemTypes.Identifier(\"{}\")", ident)
    }

    fn output_script_encoder_function(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        writeln!(
            self.out,
            "\n{}static encode{}Script({}): DiemTypes.Script {{",
            Self::quote_doc(abi.doc()),
            abi.name().to_camel_case(),
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
            r#"const code = Stdlib.{}_CODE;
const tyArgs: Seq<DiemTypes.TypeTag> = [{}];
const args: Seq<DiemTypes.TransactionArgument> = [{}];
return new DiemTypes.Script(code, tyArgs, args);"#,
            abi.name().to_shouty_snake_case(),
            Self::quote_type_arguments(abi.ty_args()),
            Self::quote_arguments(abi.args()),
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_script_function_decoder_function(&mut self, abi: &ScriptFunctionABI) -> Result<()> {
        let arg_name = format!(
            "{}script_fun",
            if abi.ty_args().is_empty() && abi.args().is_empty() {
                "_"
            } else {
                ""
            }
        );
        writeln!(
            self.out,
            "\nstatic decode{}ScriptFunction({}: DiemTypes.TransactionPayload): ScriptFunctionCallVariant{0} {{",
            abi.name().to_camel_case(),
            // prevent warning "unused variable"
            arg_name,
        )?;

        writeln!(
            self.out,
            "if ({} instanceof DiemTypes.TransactionPayloadVariantScriptFunction) {{",
            arg_name
        )?;
        self.out.indent();

        let mut all_args: Vec<String> = Vec::new();
        all_args.extend(
            abi.ty_args()
                .iter()
                .enumerate()
                .map(|(idx, _)| format!("script_fun.value.ty_args[{}]", idx))
                .collect::<Vec<_>>(),
        );
        self.out.indent();
        for (idx, arg) in abi.args().iter().enumerate() {
            writeln!(
                self.out,
                "{}",
                Self::quote_deserialize_transaction_argument(
                    arg.type_tag(),
                    arg.name(),
                    &format!("script_fun.value.args[{}]", idx)
                )
            )?;
            all_args.push(arg.name().to_string())
        }
        writeln!(
            self.out,
            "return new ScriptFunctionCallVariant{}(",
            abi.name().to_camel_case()
        )?;
        self.out.indent();
        writeln!(self.out, "{}", all_args.join(",\n"))?;
        self.out.unindent();
        writeln!(self.out, ");",)?;
        self.out.unindent();

        writeln!(self.out, "}} else {{")?;
        self.out.indent();
        writeln!(
            self.out,
            "throw new Error(\"Transaction payload not a script function payload\")"
        )?;
        self.out.unindent();
        writeln!(self.out, "}}")?;

        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_script_decoder_function(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        writeln!(
            self.out,
            "\nstatic decode{}Script({}script: DiemTypes.Script): ScriptCallVariant{0} {{",
            abi.name().to_camel_case(),
            // prevent warning "unused variable"
            if abi.ty_args().is_empty() && abi.args().is_empty() {
                "_"
            } else {
                ""
            }
        )?;
        let mut all_args: Vec<String> = Vec::new();
        all_args.extend(
            abi.ty_args()
                .iter()
                .enumerate()
                .map(|(idx, _)| format!("script.ty_args[{}]", idx))
                .collect::<Vec<_>>(),
        );
        all_args.extend(
            abi.args()
                .iter()
                .enumerate()
                .map(|(idx, arg)| {
                    format!(
                        "(script.args[{}] as {}).value",
                        idx,
                        Self::quote_transaction_argument_type(arg.type_tag())
                    )
                })
                .collect::<Vec<_>>(),
        );
        self.out.indent();
        writeln!(
            self.out,
            "return new ScriptCallVariant{}(",
            abi.name().to_camel_case()
        )?;
        self.out.indent();
        writeln!(self.out, "{}", all_args.join(",\n"))?;
        self.out.unindent();
        writeln!(self.out, ");",)?;
        self.out.unindent();
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_code_constant(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        writeln!(
            self.out,
            "\nstatic {}_CODE = Stdlib.fromHexString('{}');",
            abi.name().to_shouty_snake_case(),
            abi.code()
                .iter()
                .map(|x| format!("{:02x}", *x as i8))
                .collect::<Vec<_>>()
                .join("")
        )?;
        Ok(())
    }

    fn output_script_fun_args_definition(&mut self, abi: &ScriptFunctionABI) -> Result<()> {
        writeln!(
            self.out,
            r#"
            {0}: {{
  stdlibEncodeFunction: Stdlib.encode{0}ScriptFunction,
  description: "{1}",
  typeArgs: [{2}],
  args: [
    {3}
  ]
}},
            "#,
            abi.name().to_camel_case(),
            abi.doc().replace("\"", "\\\"").replace("\n", "\" + \n \""),
            abi.ty_args()
                .iter()
                .map(|ty_arg| format!("\"{}\"", ty_arg.name()))
                .collect::<Vec<_>>()
                .join(", "),
            abi.args()
                .iter()
                .map(|arg| format!(
                    "{{name: \"{}\", type: {}}}",
                    arg.name(),
                    Self::quote_script_arg_type(arg.type_tag())
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        Ok(())
    }

    fn output_script_args_definition(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        writeln!(self.out, "{}: {{", abi.name().to_camel_case())?;
        writeln!(
            self.out,
            "  stdlibEncodeFunction: Stdlib.encode{}Script,",
            abi.name().to_camel_case()
        )?;
        writeln!(
            self.out,
            "  stdlibDecodeFunction: Stdlib.decode{}Script,",
            abi.name().to_camel_case()
        )?;
        writeln!(
            self.out,
            "  codeName: '{}',",
            abi.name().to_shouty_snake_case()
        )?;
        writeln!(
            self.out,
            "  description: \"{}\",",
            abi.doc().replace("\"", "\\\"").replace("\n", "\" + \n \"")
        )?;
        writeln!(
            self.out,
            "  typeArgs: [{}],",
            abi.ty_args()
                .iter()
                .map(|ty_arg| format!("\"{}\"", ty_arg.name()))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        writeln!(self.out, "  args: [")?;
        writeln!(
            self.out,
            "{}",
            abi.args()
                .iter()
                .map(|arg| format!(
                    "{{name: \"{}\", type: {}}}",
                    arg.name(),
                    Self::quote_script_arg_type(arg.type_tag())
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        writeln!(self.out, "  ]")?;
        writeln!(self.out, "}},")?;
        Ok(())
    }

    fn output_script_args_callbacks(&mut self, abi: &TransactionScriptABI) -> Result<()> {
        let mut args_with_types = abi
            .ty_args()
            .iter()
            .map(|ty_arg| {
                format!(
                    "{}: DiemTypes.TypeTagVariantStruct",
                    ty_arg.name().to_mixed_case()
                )
            })
            .collect::<Vec<_>>();
        args_with_types.extend(
            abi.args()
                .iter()
                .map(|arg| {
                    format!(
                        "{}: {}",
                        arg.name().to_mixed_case(),
                        Self::quote_transaction_argument_type(arg.type_tag())
                    )
                })
                .collect::<Vec<_>>(),
        );
        writeln!(
            self.out,
            "{}: (type: string, {}) => void;",
            abi.name().to_camel_case(),
            args_with_types.join(", ")
        )?;
        Ok(())
    }

    fn quote_doc(doc: &str) -> String {
        let doc = crate::common::prepare_doc_string(doc);
        let text = textwrap::indent(&doc, " * ").replace("\n\n", "\n *\n");
        format!("/**\n{}\n */\n", text)
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
            .map(|arg| Self::quote_transaction_argument(arg.type_tag(), arg.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_type(type_tag: &TypeTag) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => "boolean".into(),
            U8 => "number".into(),
            U64 => "bigint".into(),
            U128 => "bigint".into(),
            Address => "DiemTypes.AccountAddress".into(),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => "Uint8Array".into(),
                _ => common::type_not_allowed(type_tag),
            },

            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn quote_transaction_argument_type(type_tag: &TypeTag) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => "DiemTypes.TransactionArgumentVariantBool".to_string(),
            U8 => "DiemTypes.TransactionArgumentVariantU8".to_string(),
            U64 => "DiemTypes.TransactionArgumentVariantU64".to_string(),
            U128 => "DiemTypes.TransactionArgumentVariantU128".to_string(),
            Address => "DiemTypes.TransactionArgumentVariantAddress".to_string(),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => "DiemTypes.TransactionArgumentVariantU8Vector".to_string(),
                _ => common::type_not_allowed(type_tag),
            },

            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn quote_transaction_argument(type_tag: &TypeTag, name: &str) -> String {
        format!(
            "new {}({})",
            Self::quote_transaction_argument_type(type_tag),
            name
        )
    }

    fn quote_deserialize_transaction_argument_type(type_tag: &TypeTag, ser_name: &str) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => format!("{}.deserializeBool()", ser_name),
            U8 => format!("{}.deserializeU8()", ser_name),
            U64 => format!("{}.deserializeU64()", ser_name),
            U128 => format!("{}.deserializeU128()", ser_name),
            Address => format!("DiemTypes.AccountAddress.deserialize({})", ser_name),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => format!("{}.deserializeBytes()", ser_name),
                // TODO: support vec<vec<u8>>
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
        format!(
            "var deserializer = new BcsDeserializer({});\n\
            const {}: {} = {};\n",
            data_access,
            name,
            Self::quote_type(type_tag),
            Self::quote_deserialize_transaction_argument_type(type_tag, "deserializer"),
        )
    }

    fn quote_serialize_transaction_argument_type(
        type_tag: &TypeTag,
        ser_name: &str,
        arg_name: &str,
    ) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => format!("{}.serializeBool({})", ser_name, arg_name),
            U8 => format!("{}.serializeU8({})", ser_name, arg_name),
            U64 => format!("{}.serializeU64({})", ser_name, arg_name),
            U128 => format!("{}.serializeU128({})", ser_name, arg_name),
            Address => format!("{}.serialize({})", arg_name, ser_name),
            Vector(type_tag) => match type_tag.as_ref() {
                U8 => format!("{}.serializeBytes({})", ser_name, arg_name),
                // TODO: support vec<vec<u8>>
                _ => common::type_not_allowed(type_tag),
            },

            Struct(_) | Signer => common::type_not_allowed(type_tag),
        }
    }

    fn quote_serialize_transaction_argument(type_tag: &TypeTag, name: &str) -> String {
        format!(
            "var serializer = new BcsSerializer();\n\
            {};\n\
            const {1}_serialized: bytes = serializer.getBytes();\n",
            Self::quote_serialize_transaction_argument_type(type_tag, "serializer", name),
            name
        )
    }

    fn quote_script_arg_type(type_tag: &TypeTag) -> String {
        use TypeTag::*;
        match type_tag {
            Bool => "{type: Types.Boolean}".to_string(),
            U8 => "{type: Types.U8}".to_string(),
            U64 => "{type: Types.U64}".to_string(),
            U128 => "{type: Types.U128}".to_string(),
            Address => "{type: Types.Address}".to_string(),
            Vector(type_tag) => format!("{{type: Types.Array, arrayType: {}}}", Self::quote_script_arg_type(type_tag)),
            Struct(struct_tag) => format!("{{type: Types.Struct, name: \"{}\", moduleName: \"{}\", address: \"{}\", typeParams: [{}]}}",
                                          struct_tag.name,
                                          struct_tag.module,
                                          struct_tag.address,
                                          struct_tag.type_params.iter().map(|tt| Self::quote_script_arg_type(tt)).collect::<Vec<_>>().join(", ")),
            Signer => common::type_not_allowed(type_tag),
        }
    }
}

pub struct Installer {
    install_dir: PathBuf,
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
        let dir_path = self.install_dir.join(name);
        std::fs::create_dir_all(&dir_path)?;
        let mut file = std::fs::File::create(dir_path.join("mod.ts"))?;
        output(&mut file, abis)?;
        Ok(())
    }
}

/// Walks through the registry replacing variables known to be named as a
/// javascript keyword, making the resulting codegen invalid.
/// ie: public function: Identifier => public function_name: Identifier
pub fn replace_keywords(registry: &mut BTreeMap<String, ContainerFormat>) {
    swap_keyworded_fields(registry.get_mut("StructTag"));
    swap_keyworded_fields(registry.get_mut("ScriptFunction"));
}

fn swap_keyworded_fields(fields: Option<&mut ContainerFormat>) {
    if let Some(ContainerFormat::Struct(fields)) = fields {
        for entry in fields.iter_mut() {
            match entry.name.as_str() {
                "module" => entry.name = String::from("module_name"),
                "function" => entry.name = String::from("function_name"),
                _ => {}
            }
        }
    }
}
