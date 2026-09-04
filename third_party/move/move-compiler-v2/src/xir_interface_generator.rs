// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers an XIR interface to Move source, so it can be supplied to the
//! compiler as an ordinary dependency.
//!
//! Why text, when the interface is already structured data: name resolution
//! happens in two places that both work from parsed source. Legacy expansion
//! rejects `use 0x1::dep::T` for a module it has not parsed
//! (`expansion/translate.rs`, `Invalid 'use'. Unbound module`), and
//! `ModelBuilder`'s `struct_table`/`fun_table` are populated only from the
//! expansion AST. A module loaded straight into the `GlobalEnv` — which is
//! what [`crate::xir`] does — is therefore invisible to source that refers to
//! it. Generating source is how `.mv` dependencies have always been consumed
//! (`legacy_move_compiler::interface_generator`), and this mirrors it.
//!
//! What the round trip through text costs is parsing; what it saves is the
//! dependency's *bodies*, which are never compiled. What it preserves that the
//! `.mv` route cannot is attributes: a `CompiledModule` carries them only in a
//! metadata section the interface generator does not read, whereas they are
//! first-class in XIR.
//!
//! Function bodies are omitted by declaring every function `native`, exactly as
//! the `.mv` generator does, and each is marked [`NATIVE_INTERFACE`] so that
//! consumers do not mistake a stub for a genuine native.

use anyhow::{bail, Context, Result};
use legacy_move_compiler::interface_generator::NATIVE_INTERFACE;
use move_model_exchange::{
    Field, Type as Ty, TypeParameter, Variant, XirAttribute, XirAttributeArg, XirFunction,
    XirModule, XirStruct, XirVisibility,
};
use std::{fs, path::PathBuf};
use tempfile::TempDir;

/// The generated header, so a reader who stumbles on one of these files in a
/// build directory knows what produced it.
const DISCLAIMER: &str =
    "// Generated from an XIR interface. Functions are declared 'native' because\n\
     // a dependency's bodies are not compiled; they may or may not be native.";

/// Move source generated for a set of XIR dependencies, together with the
/// directory holding it.
///
/// The directory is removed when this value is dropped, which is safe as soon
/// as the compiler has parsed the files: `GlobalEnv::add_source` copies each
/// file's text into the environment, so diagnostics still render afterwards.
pub struct GeneratedInterfaces {
    _dir: TempDir,
    /// Paths to hand to the compiler as ordinary source dependencies.
    pub paths: Vec<String>,
}

/// Reads each `.xir.json` interface and writes the Move source it lowers to.
///
/// Each file is validated on the way through [`crate::xir::parse_interface`],
/// so a malformed or wrong-version document is reported here rather than as a
/// confusing parse error in generated text.
pub fn generate_dependency_sources(xir_dependencies: &[String]) -> Result<GeneratedInterfaces> {
    let dir = tempfile::Builder::new()
        .prefix("move-xir-interfaces")
        .tempdir()
        .context("creating a directory for generated XIR interfaces")?;
    let mut paths = vec![];
    for dependency in xir_dependencies {
        let json = fs::read_to_string(dependency)
            .with_context(|| format!("reading XIR dependency `{dependency}`"))?;
        let source = crate::xir::parse_interface(PathBuf::from(dependency), String::new(), &json)?;
        let module = source.module();
        let path = dir.path().join(format!(
            "{}_{}.move",
            module.module.address.trim_start_matches("0x"),
            module.module.name
        ));
        fs::write(&path, xir_module_to_move_source(module)?)
            .with_context(|| format!("writing the interface generated from `{dependency}`"))?;
        paths.push(path.to_string_lossy().into_owned());
    }
    Ok(GeneratedInterfaces { _dir: dir, paths })
}

/// Renders `module`'s declarations as Move source.
pub fn xir_module_to_move_source(module: &XirModule) -> Result<String> {
    let interface = Interface::new(module);
    let mut out = String::new();
    out.push_str(DISCLAIMER);
    out.push_str(&format!(
        "\nmodule {}::{} {{\n",
        module.module.address, module.module.name
    ));

    for friend in &module.friends {
        out.push_str(&format!(
            "    friend {}::{};\n",
            friend.address, friend.module
        ));
    }
    if !module.friends.is_empty() {
        out.push('\n');
    }

    for decl in &module.structs {
        out.push_str(
            &interface
                .struct_source(decl)
                .with_context(|| format!("on struct `{}`", decl.name))?,
        );
        out.push('\n');
    }

    for decl in &module.functions {
        out.push_str(
            &interface
                .function_source(decl)
                .with_context(|| format!("on function `{}`", decl.name))?,
        );
        out.push('\n');
    }

    out.push_str("}\n");
    Ok(out)
}

/// Rendering state: the declaration tables a resource id is resolved against.
struct Interface<'a> {
    module: &'a XirModule,
}

impl<'a> Interface<'a> {
    fn new(module: &'a XirModule) -> Self {
        Self { module }
    }

    /// Resolves a resource id to a Move type path. Ids below the local count
    /// name this module's own declarations; the rest index `external_types`,
    /// which is the convention the reader uses.
    fn resource_path(&self, id: usize) -> Result<String> {
        let locals = self.module.structs.len();
        if let Some(decl) = self.module.structs.get(id) {
            return Ok(decl.name.clone());
        }
        let external = self
            .module
            .external_types
            .get(id - locals)
            .with_context(|| format!("resource id {id} is outside the declaration tables"))?;
        Ok(format!(
            "{}::{}::{}",
            external.address, external.module, external.name
        ))
    }

    /// Renders a type. `params` supplies the enclosing declaration's type
    /// parameter names, since XIR refers to them by index.
    fn ty(&self, ty: &Ty, params: &[TypeParameter]) -> Result<String> {
        Ok(match ty {
            Ty::Bool => "bool".to_owned(),
            Ty::U8 => "u8".to_owned(),
            Ty::U16 => "u16".to_owned(),
            Ty::U32 => "u32".to_owned(),
            Ty::U64 => "u64".to_owned(),
            Ty::U128 => "u128".to_owned(),
            Ty::U256 => "u256".to_owned(),
            Ty::I8 => "i8".to_owned(),
            Ty::I16 => "i16".to_owned(),
            Ty::I32 => "i32".to_owned(),
            Ty::I64 => "i64".to_owned(),
            Ty::I128 => "i128".to_owned(),
            Ty::I256 => "i256".to_owned(),
            Ty::Address => "address".to_owned(),
            Ty::Signer => "signer".to_owned(),
            Ty::TypeParameter(index) => params
                .get(*index)
                .with_context(|| format!("type parameter {index} is not in scope"))?
                .name
                .clone(),
            Ty::Struct(id) | Ty::Enum(id) => self.resource_path(*id)?,
            Ty::StructInst(id, args) | Ty::EnumInst(id, args) => format!(
                "{}<{}>",
                self.resource_path(*id)?,
                self.ty_list(args, params)?.join(", ")
            ),
            Ty::Vector(element) => format!("vector<{}>", self.ty(element, params)?),
            Ty::Ref(referent) => format!("&{}", self.ty(referent, params)?),
            Ty::MutRef(referent) => format!("&mut {}", self.ty(referent, params)?),
            Ty::Fun(args, results, abilities) => {
                // `|A, B|(R)`, and `|A, B|` when there is no result.
                //
                // The result is always parenthesized, including at arity one.
                // A bare address-qualified path is not accepted there —
                // `|&0x1::m::S| 0x1::m::T` fails to parse while
                // `|&0x1::m::S| (0x1::m::T)` succeeds — and parentheses are
                // the same production that spells a multi-result type, so one
                // rule covers every arity.
                let results = match self.ty_list(results, params)?.as_slice() {
                    [] => String::new(),
                    several => format!("({})", several.join(", ")),
                };
                let abilities = if abilities.is_empty() {
                    String::new()
                } else {
                    format!(" has {}", abilities.join(" + "))
                };
                format!(
                    "|{}|{}{}",
                    self.ty_list(args, params)?.join(", "),
                    results,
                    abilities
                )
            },
        })
    }

    fn ty_list(&self, types: &[Ty], params: &[TypeParameter]) -> Result<Vec<String>> {
        types.iter().map(|ty| self.ty(ty, params)).collect()
    }

    fn struct_source(&self, decl: &XirStruct) -> Result<String> {
        let mut out = String::new();
        for attribute in &decl.attributes {
            out.push_str(&format!("    {}\n", attribute_source(attribute)?));
        }
        let visibility = match decl.visibility {
            XirVisibility::Public => "public ",
            // A type's `friend` visibility has no source spelling of its own;
            // `public` is the closest, and a dependency's friend list is
            // already declared above, so access stays checked.
            XirVisibility::Friend | XirVisibility::Private => "",
        };
        let abilities = if decl.abilities.is_empty() {
            String::new()
        } else {
            format!(" has {}", decl.abilities.join(", "))
        };
        let keyword = if decl.variants.is_some() {
            "enum"
        } else {
            "struct"
        };
        let head = format!(
            "    {visibility}{keyword} {}{}",
            decl.name,
            type_parameter_source(&decl.type_parameters)
        );

        if let Some(variants) = &decl.variants {
            out.push_str(&format!("{head}{abilities} {{\n"));
            for variant in variants {
                out.push_str(&self.variant_source(variant, &decl.type_parameters)?);
            }
            out.push_str("    }\n");
        } else if is_positional(&decl.fields) {
            // `struct P(u64) has drop;` — the fields are named `0`, `1`, …,
            // which is not spellable in the braced form.
            let types = decl
                .fields
                .iter()
                .map(|field| self.ty(&field.ty, &decl.type_parameters))
                .collect::<Result<Vec<_>>>()?;
            out.push_str(&format!("{head}({}){abilities};\n", types.join(", ")));
        } else {
            out.push_str(&format!("{head}{abilities} {{\n"));
            for field in &decl.fields {
                out.push_str(&format!(
                    "        {}: {},\n",
                    field.name,
                    self.ty(&field.ty, &decl.type_parameters)?
                ));
            }
            out.push_str("    }\n");
        }
        Ok(out)
    }

    fn variant_source(&self, variant: &Variant, params: &[TypeParameter]) -> Result<String> {
        if variant.fields.is_empty() {
            return Ok(format!("        {},\n", variant.name));
        }
        // A variant is positional or braced independently of its enum:
        // `enum Result<T, E> { Ok(T), Err(E) }` in `move-stdlib` has both
        // variants positional, and their fields are named `0`.
        if is_positional(&variant.fields) {
            let types = variant
                .fields
                .iter()
                .map(|field| self.ty(&field.ty, params))
                .collect::<Result<Vec<_>>>()?;
            return Ok(format!("        {}({}),\n", variant.name, types.join(", ")));
        }
        let fields = variant
            .fields
            .iter()
            .map(|field| Ok(format!("{}: {}", field.name, self.ty(&field.ty, params)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(format!(
            "        {} {{ {} }},\n",
            variant.name,
            fields.join(", ")
        ))
    }

    fn function_source(&self, decl: &XirFunction) -> Result<String> {
        let mut out = String::new();
        for attribute in &decl.attributes {
            out.push_str(&format!("    {}\n", attribute_source(attribute)?));
        }
        out.push_str(&format!("    #[{NATIVE_INTERFACE}]\n"));

        let visibility = match decl.visibility {
            XirVisibility::Public => "public ",
            XirVisibility::Friend => "public(friend) ",
            // The exporter omits private functions; one reaching here would
            // be unreferenceable, so it is a bug rather than a no-op.
            XirVisibility::Private => {
                bail!("a private function does not belong in an interface")
            },
        };
        let entry = if decl.is_entry { "entry " } else { "" };

        let params = decl
            .locals
            .get(..decl.params)
            .with_context(|| "function declares more parameters than locals")?
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                Ok(format!(
                    "{}: {}",
                    parameter_name(decl, index),
                    self.ty(ty, &decl.type_parameters)?
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        let returns = match self
            .ty_list(&decl.returns, &decl.type_parameters)?
            .as_slice()
        {
            [] => String::new(),
            [single] => format!(": {single}"),
            several => format!(": ({})", several.join(", ")),
        };

        out.push_str(&format!(
            "    native {visibility}{entry}fun {}{}({}){};\n",
            decl.name,
            type_parameter_source(&decl.type_parameters),
            params.join(", "),
            returns
        ));
        Ok(out)
    }
}

/// A parameter's declared name, or a positional fallback. The name is only
/// documentation — callers pass arguments positionally — but a stub still
/// needs *some* identifier, and a generated one must not collide with a
/// declared one, hence the leading underscore.
fn parameter_name(decl: &XirFunction, index: usize) -> String {
    match decl.local_names.get(index) {
        Some(Some(name)) => name.clone(),
        _ => format!("_p{index}"),
    }
}

/// Whether the fields are those of a positional struct, which `move-model`
/// names `0`, `1`, … Mixed forms do not occur: Move has no way to declare one.
fn is_positional(fields: &[Field]) -> bool {
    !fields.is_empty()
        && fields
            .iter()
            .all(|field| field.name.bytes().all(|byte| byte.is_ascii_digit()))
}

fn type_parameter_source(params: &[TypeParameter]) -> String {
    if params.is_empty() {
        return String::new();
    }
    let rendered = params
        .iter()
        .map(|param| {
            let phantom = if param.phantom { "phantom " } else { "" };
            let constraints = if param.abilities.is_empty() {
                String::new()
            } else {
                format!(": {}", param.abilities.join(" + "))
            };
            format!("{phantom}{}{constraints}", param.name)
        })
        .collect::<Vec<_>>();
    format!("<{}>", rendered.join(", "))
}

fn attribute_source(attribute: &XirAttribute) -> Result<String> {
    Ok(format!(
        "#[{}]",
        attribute_body(&attribute.name, &attribute.args)?
    ))
}

fn attribute_body(name: &str, args: &[XirAttributeArg]) -> Result<String> {
    if args.is_empty() {
        return Ok(name.to_owned());
    }
    // A lone literal is an assignment: `#[randomness = 7]`. This mirrors the
    // reader's rule, which is the only way the top-level form can spell one.
    if let [arg @ (XirAttributeArg::Num { .. } | XirAttributeArg::Bool { .. })] = args {
        return Ok(format!("{name} = {}", attribute_arg(arg)?));
    }
    let args = args.iter().map(attribute_arg).collect::<Result<Vec<_>>>()?;
    Ok(format!("{name}({})", args.join(", ")))
}

fn attribute_arg(arg: &XirAttributeArg) -> Result<String> {
    Ok(match arg {
        XirAttributeArg::Name { name, args } => attribute_body(name, args)?,
        XirAttributeArg::Assign { assign, value } => {
            format!("{assign} = {}", attribute_arg(value)?)
        },
        XirAttributeArg::Num { value } => value.clone(),
        XirAttributeArg::Bool { value } => value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_model_exchange::{XirModuleMetadata, XirModuleRef, XIR_SCHEMA, XIR_VERSION};

    fn module(structs: Vec<XirStruct>, functions: Vec<XirFunction>) -> XirModule {
        XirModule {
            schema: XIR_SCHEMA.to_string(),
            version: XIR_VERSION,
            module: XirModuleMetadata {
                address: "0x42".to_string(),
                name: "m".to_string(),
                dialect: move_model_exchange::XirDialect::Stackless,
            },
            structs,
            functions,
            friends: vec![],
            external_functions: vec![],
            external_types: vec![],
        }
    }

    fn struct_decl(name: &str, fields: Vec<Field>) -> XirStruct {
        XirStruct {
            name: name.to_string(),
            visibility: XirVisibility::Public,
            abilities: vec!["drop".to_string()],
            type_parameters: vec![],
            fields,
            variants: None,
            attributes: vec![],
        }
    }

    fn field(name: &str, ty: Ty) -> Field {
        Field {
            name: name.to_string(),
            ty,
        }
    }

    /// A function type's result is parenthesized at every arity.
    ///
    /// At arity one this is not cosmetic: `|&0x1::m::S| 0x1::m::T` does not
    /// parse, while the parenthesized form does. Emitting the same shape for
    /// one result as for several keeps a single rule.
    #[test]
    fn function_type_results_are_parenthesized() {
        let external = |name: &str| Ty::Struct(1 + usize::from(name == "T"));
        let source = xir_module_to_move_source(&module(
            vec![
                struct_decl("Holder", vec![
                    field(
                        "0",
                        Ty::Fun(
                            vec![Ty::Ref(Box::new(external("S")))],
                            vec![external("T")],
                            vec![],
                        ),
                    ),
                    field(
                        "1",
                        Ty::Fun(vec![Ty::U64, Ty::Bool], vec![], vec![
                            "copy".into(),
                            "drop".into(),
                        ]),
                    ),
                    field("2", Ty::Fun(vec![], vec![Ty::U64, Ty::Bool], vec![])),
                ]),
                struct_decl("S", vec![field("v", Ty::U64)]),
                struct_decl("T", vec![field("v", Ty::U64)]),
            ],
            vec![],
        ))
        .unwrap();
        assert!(
            source.contains(
                "struct Holder(|&S|(T), |u64, bool| has copy + drop, ||(u64, bool)) has drop;"
            ),
            "{source}"
        );
    }

    /// Positional declarations keep the parenthesized form, because their
    /// fields are named `0`, `1`, … and a braced body cannot spell that.
    /// A variant's form is independent of its enum's.
    #[test]
    fn positional_declarations_use_the_parenthesized_form() {
        let source = xir_module_to_move_source(&module(
            vec![
                struct_decl("P", vec![field("0", Ty::U64), field("1", Ty::Bool)]),
                XirStruct {
                    variants: Some(vec![
                        Variant {
                            name: "Ok".to_string(),
                            fields: vec![field("0", Ty::U64)],
                        },
                        Variant {
                            name: "Named".to_string(),
                            fields: vec![field("v", Ty::Bool)],
                        },
                        Variant {
                            name: "Empty".to_string(),
                            fields: vec![],
                        },
                    ]),
                    ..struct_decl("E", vec![])
                },
            ],
            vec![],
        ))
        .unwrap();
        assert!(source.contains("struct P(u64, bool) has drop;"), "{source}");
        assert!(source.contains("Ok(u64),"), "{source}");
        assert!(source.contains("Named { v: bool },"), "{source}");
        assert!(source.contains("Empty,"), "{source}");
        assert!(source.contains("enum E has drop {"), "{source}");
    }

    /// Attributes are carried through, which is the reason to describe a
    /// dependency with XIR rather than with its compiled bytecode.
    #[test]
    fn attributes_and_friends_are_carried_through() {
        let mut xir = module(
            vec![XirStruct {
                attributes: vec![
                    XirAttribute {
                        name: "resource_group_member".to_string(),
                        args: vec![XirAttributeArg::Assign {
                            assign: "group".to_string(),
                            value: Box::new(XirAttributeArg::Name {
                                name: "0x1::object::ObjectGroup".to_string(),
                                args: vec![],
                            }),
                        }],
                    },
                    XirAttribute {
                        name: "randomness".to_string(),
                        args: vec![XirAttributeArg::Num {
                            value: "7".to_string(),
                        }],
                    },
                ],
                ..struct_decl("S", vec![field("v", Ty::U64)])
            }],
            vec![],
        );
        xir.friends = vec![XirModuleRef {
            address: "0x1".to_string(),
            module: "other".to_string(),
        }];
        let source = xir_module_to_move_source(&xir).unwrap();
        assert!(
            source.contains("#[resource_group_member(group = 0x1::object::ObjectGroup)]"),
            "{source}"
        );
        // A lone literal argument is an assignment, matching the reader.
        assert!(source.contains("#[randomness = 7]"), "{source}");
        assert!(source.contains("friend 0x1::other;"), "{source}");
    }
}
