// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Exports a module's **interface** as XIR.
//!
//! This is the inverse of the declaration half of [`crate::xir`], and
//! deliberately only that half: function bodies are omitted. A dependency
//! needs signatures, types, abilities, visibility, attributes, and friend
//! declarations in order to be compiled against; it does not need code, and
//! omitting code avoids translating stackless bytecode back into XIR
//! instructions — the expensive part of a full exporter.
//!
//! The result therefore carries `dialect: Stackless` with every function's
//! `blocks` and `loops` empty. That is not a distinct dialect: bodies are
//! simply absent, and the reader accepts absent bodies for a module supplied
//! as a *dependency* while still requiring them of a compilation target.
//!
//! A full-fidelity exporter can be built on this code later by adding body
//! translation.

use anyhow::{bail, Context, Result};
use move_binary_format::file_format::Visibility as MoveVisibility;
use move_core_types::ability::{Ability, AbilitySet};
use move_model::{
    model::{FunctionEnv, ModuleEnv, ModuleId, QualifiedId, StructEnv, StructId},
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_model_exchange::{
    Field, Type as Ty, TypeParameter as TypeParameterDecl, Variant, XirDialect, XirExternalType,
    XirFunction, XirModule, XirModuleMetadata, XirModuleRef, XirStruct, XirVisibility, XIR_SCHEMA,
    XIR_VERSION,
};
use std::collections::BTreeMap;

/// Accumulates the tables that give foreign declarations an index.
///
/// Local declarations occupy `0..locals`, so a reference to another module is
/// appended and addressed at `locals + position` — the convention the reader
/// resolves, and the one a bytecode handle table uses.
struct References<'env> {
    module: &'env ModuleEnv<'env>,
    local_structs: BTreeMap<StructId, usize>,
    external_types: Vec<XirExternalType>,
    external_index: BTreeMap<QualifiedId<StructId>, usize>,
}

impl<'env> References<'env> {
    fn new(module: &'env ModuleEnv<'env>) -> Self {
        let local_structs = module
            .get_structs()
            .enumerate()
            .map(|(index, struct_env)| (struct_env.get_id(), index))
            .collect();
        Self {
            module,
            local_structs,
            external_types: vec![],
            external_index: BTreeMap::new(),
        }
    }

    fn local_count(&self) -> usize {
        self.local_structs.len()
    }

    /// The XIR resource id for a struct, interning a foreign one on first use.
    fn resource_id(&mut self, module_id: ModuleId, struct_id: StructId) -> Result<usize> {
        if module_id == self.module.get_id() {
            return self
                .local_structs
                .get(&struct_id)
                .copied()
                .context("a local struct is missing from the module's own declarations");
        }
        let qid = module_id.qualified(struct_id);
        if let Some(index) = self.external_index.get(&qid) {
            return Ok(self.local_count() + index);
        }
        let env = self.module.env;
        let foreign = env.get_module(module_id);
        let struct_env = foreign.get_struct(struct_id);
        let name = foreign.get_name();
        let index = self.external_types.len();
        self.external_types.push(XirExternalType {
            address: name.addr().expect_numerical().to_hex_literal(),
            module: env.symbol_pool().string(name.name()).to_string(),
            name: env.symbol_pool().string(struct_env.get_name()).to_string(),
        });
        self.external_index.insert(qid, index);
        Ok(self.local_count() + index)
    }
}

fn ability_names(abilities: AbilitySet) -> Vec<String> {
    [
        (Ability::Copy, "copy"),
        (Ability::Drop, "drop"),
        (Ability::Store, "store"),
        (Ability::Key, "key"),
    ]
    .into_iter()
    .filter(|(ability, _)| abilities.has_ability(*ability))
    .map(|(_, name)| name.to_string())
    .collect()
}

fn visibility(visibility: MoveVisibility) -> XirVisibility {
    match visibility {
        MoveVisibility::Private => XirVisibility::Private,
        MoveVisibility::Public => XirVisibility::Public,
        MoveVisibility::Friend => XirVisibility::Friend,
    }
}

/// Translates a model type, interning any foreign declaration it names.
fn export_type(refs: &mut References, ty: &Type) -> Result<Ty> {
    Ok(match ty {
        Type::Primitive(primitive) => match primitive {
            PrimitiveType::Bool => Ty::Bool,
            PrimitiveType::U8 => Ty::U8,
            PrimitiveType::U16 => Ty::U16,
            PrimitiveType::U32 => Ty::U32,
            PrimitiveType::U64 => Ty::U64,
            PrimitiveType::U128 => Ty::U128,
            PrimitiveType::U256 => Ty::U256,
            PrimitiveType::I8 => Ty::I8,
            PrimitiveType::I16 => Ty::I16,
            PrimitiveType::I32 => Ty::I32,
            PrimitiveType::I64 => Ty::I64,
            PrimitiveType::I128 => Ty::I128,
            PrimitiveType::I256 => Ty::I256,
            PrimitiveType::Address => Ty::Address,
            PrimitiveType::Signer => Ty::Signer,
            other => bail!("`{other:?}` is a specification-only type and has no XIR form"),
        },
        Type::TypeParameter(index) => Ty::TypeParameter(*index as usize),
        Type::Struct(module_id, struct_id, args) => {
            let id = refs.resource_id(*module_id, *struct_id)?;
            // An enum is a struct with variants; the reader treats the two
            // resource forms alike, so the struct form is always correct here.
            if args.is_empty() {
                Ty::Struct(id)
            } else {
                Ty::StructInst(
                    id,
                    args.iter()
                        .map(|arg| export_type(refs, arg))
                        .collect::<Result<Vec<_>>>()?,
                )
            }
        },
        Type::Vector(element) => Ty::Vector(Box::new(export_type(refs, element)?)),
        Type::Reference(ReferenceKind::Immutable, referent) => {
            Ty::Ref(Box::new(export_type(refs, referent)?))
        },
        Type::Reference(ReferenceKind::Mutable, referent) => {
            Ty::MutRef(Box::new(export_type(refs, referent)?))
        },
        Type::Fun(args, result, abilities) => Ty::Fun(
            Box::new(export_type(refs, args)?),
            Box::new(export_type(refs, result)?),
            ability_names(*abilities),
        ),
        other => bail!("type `{other:?}` has no XIR form"),
    })
}

fn export_type_parameters(
    module: &ModuleEnv,
    parameters: &[move_model::model::TypeParameter],
) -> Vec<TypeParameterDecl> {
    parameters
        .iter()
        .map(|parameter| TypeParameterDecl {
            name: module.env.symbol_pool().string(parameter.0).to_string(),
            abilities: ability_names(parameter.1.abilities),
            phantom: parameter.1.is_phantom,
        })
        .collect()
}

fn export_struct(refs: &mut References, struct_env: &StructEnv) -> Result<XirStruct> {
    let module = &struct_env.module_env;
    let pool = module.env.symbol_pool();
    let variants = if struct_env.has_variants() {
        Some(
            struct_env
                .get_variants()
                .map(|variant| {
                    Ok(Variant {
                        name: pool.string(variant).to_string(),
                        fields: struct_env
                            .get_fields_of_variant(variant)
                            .map(|field| {
                                Ok(Field {
                                    name: pool.string(field.get_name()).to_string(),
                                    ty: export_type(refs, &field.get_type())?,
                                })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        )
    } else {
        None
    };
    // Enum fields live on the variants; the top-level list stays empty.
    let fields = if variants.is_some() {
        vec![]
    } else {
        struct_env
            .get_fields()
            .map(|field| {
                Ok(Field {
                    name: pool.string(field.get_name()).to_string(),
                    ty: export_type(refs, &field.get_type())?,
                })
            })
            .collect::<Result<Vec<_>>>()?
    };
    Ok(XirStruct {
        name: pool.string(struct_env.get_name()).to_string(),
        visibility: visibility(struct_env.get_visibility()),
        abilities: ability_names(struct_env.get_abilities()),
        type_parameters: export_type_parameters(module, struct_env.get_type_parameters()),
        fields,
        variants,
        // Attributes are model-internal `Attribute` values; an interface
        // exporter that needs them must translate them explicitly.
        attributes: vec![],
    })
}

/// Exports one function's declaration surface. `locals` holds exactly the
/// parameter types, since with no body there are no other locals.
fn export_function(refs: &mut References, fun_env: &FunctionEnv) -> Result<XirFunction> {
    let module = &fun_env.module_env;
    let pool = module.env.symbol_pool();
    let parameters = fun_env.get_parameters();
    let locals = parameters
        .iter()
        .map(|parameter| export_type(refs, &parameter.1))
        .collect::<Result<Vec<_>>>()?;
    let result = fun_env.get_result_type();
    let returns = match result {
        Type::Tuple(types) => types
            .iter()
            .map(|ty| export_type(refs, ty))
            .collect::<Result<Vec<_>>>()?,
        Type::Primitive(PrimitiveType::Bool)
        | Type::Primitive(_)
        | Type::Struct(..)
        | Type::Vector(_)
        | Type::Reference(..)
        | Type::TypeParameter(_)
        | Type::Fun(..) => vec![export_type(refs, &result)?],
        other => bail!("result type `{other:?}` has no XIR form"),
    };
    let acquires = fun_env
        .get_acquires_global_resources()
        .unwrap_or_default()
        .into_iter()
        .map(|struct_id| refs.resource_id(module.get_id(), struct_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(XirFunction {
        name: pool.string(fun_env.get_name()).to_string(),
        type_parameters: export_type_parameters(module, fun_env.get_type_parameters_ref()),
        visibility: visibility(fun_env.visibility()),
        is_entry: fun_env.is_entry(),
        is_native: fun_env.is_native(),
        acquires,
        params: locals.len(),
        locals,
        local_names: vec![],
        returns,
        blocks: vec![],
        entry: 0,
        loops: vec![],
        spec: move_model_exchange::Contract {
            requires: vec![],
            aborts_if: vec![],
            ensures: vec![],
            modifies: vec![],
        },
        attributes: vec![],
        source_map: None,
    })
}

/// Exports the interface of `module`.
///
/// Private functions are omitted: a dependent cannot call them, so they are
/// not part of the surface it compiles against. Every struct is included,
/// because a type may be named even where its constructor is not accessible.
pub fn export_interface(module: &ModuleEnv) -> Result<XirModule> {
    let pool = module.env.symbol_pool();
    let name = module.get_name();
    let mut refs = References::new(module);

    let structs = module
        .get_structs()
        .map(|struct_env| {
            export_struct(&mut refs, &struct_env)
                .with_context(|| format!("on struct `{}`", struct_env.get_full_name_str()))
        })
        .collect::<Result<Vec<_>>>()?;

    let functions = module
        .get_functions()
        .filter(|fun_env| fun_env.visibility() != MoveVisibility::Private)
        .map(|fun_env| {
            export_function(&mut refs, &fun_env)
                .with_context(|| format!("on function `{}`", fun_env.get_full_name_str()))
        })
        .collect::<Result<Vec<_>>>()?;

    let friends = module
        .get_friend_decls()
        .iter()
        .map(|decl| XirModuleRef {
            address: decl.module_name.addr().expect_numerical().to_hex_literal(),
            module: pool.string(decl.module_name.name()).to_string(),
        })
        .collect();

    Ok(XirModule {
        schema: XIR_SCHEMA.to_string(),
        version: XIR_VERSION,
        module: XirModuleMetadata {
            address: name.addr().expect_numerical().to_hex_literal(),
            name: pool.string(name.name()).to_string(),
            dialect: XirDialect::Stackless,
            friends,
        },
        structs,
        functions,
        // Empty by construction: with no bodies there are no calls, so this
        // interface names no foreign functions. A full exporter would fill it.
        external_functions: vec![],
        external_types: refs.external_types,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xir::{import_sources, parse_source_with_target};
    use move_model::model::GlobalEnv;
    use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
    use std::path::PathBuf;

    /// Imports XIR documents as targets and returns the populated model.
    fn model_of(documents: &[serde_json::Value]) -> GlobalEnv {
        let sources = documents
            .iter()
            .map(|document| {
                parse_source_with_target(
                    PathBuf::from("test.xir.json"),
                    String::new(),
                    &document.to_string(),
                    true,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &sources, &mut targets).unwrap();
        env
    }

    fn document(
        name: &str,
        structs: serde_json::Value,
        functions: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "schema": XIR_SCHEMA,
            "version": XIR_VERSION,
            "module": {
                "address": "0x42", "name": name, "dialect": "stackless",
                "friends": [{"address": "0x42", "module": "buddy"}],
            },
            "structs": structs,
            "functions": functions,
        })
    }

    fn empty_body() -> serde_json::Value {
        serde_json::json!([{"instrs": [], "term": {"ret": []}}])
    }

    /// Exporting an interface and importing it again yields the same
    /// declarations. This is the exporter's real test: it exercises the
    /// producer and the reader against each other rather than against a
    /// hand-written expectation.
    #[test]
    fn interface_round_trips_through_the_reader() {
        let source = document(
            "m",
            serde_json::json!([{
                "name": "Holder", "visibility": "public",
                "abilities": ["copy", "drop", "store"],
                "type_parameters": [{"name": "T", "abilities": ["drop"], "phantom": false}],
                "fields": [{"name": "value", "ty": {"type_parameter": 0}},
                           {"name": "count", "ty": "i64"}],
            }]),
            serde_json::json!([{
                "name": "visible", "visibility": "public", "is_entry": true,
                "is_native": false, "acquires": [], "params": 2,
                "locals": [{"struct_inst": [0, ["u64"]]}, "i32", "bool"],
                "returns": ["bool"],
                "blocks": [{"instrs": [{"load": [2, {"bool": true}]}],
                            "term": {"ret": [2]}}],
                "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }, {
                "name": "hidden", "visibility": "private", "is_entry": false,
                "is_native": false, "acquires": [], "params": 0,
                "locals": [], "returns": [], "blocks": empty_body(), "entry": 0,
                "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }]),
        );

        let env = model_of(&[source]);
        let module = env.get_modules().next().unwrap();
        let exported = export_interface(&module).unwrap();

        // Private functions are not part of the interface.
        assert_eq!(exported.functions.len(), 1);
        assert_eq!(exported.functions[0].name, "visible");
        assert!(exported.functions[0].is_entry);
        assert_eq!(exported.functions[0].params, 2);
        assert!(
            exported.functions[0].blocks.is_empty(),
            "an interface export carries no bodies"
        );
        assert_eq!(exported.functions[0].returns, vec![Ty::Bool]);
        // The signed width and the generic instantiation survive.
        assert_eq!(exported.functions[0].locals[1], Ty::I32);
        assert_eq!(
            exported.functions[0].locals[0],
            Ty::StructInst(0, vec![Ty::U64])
        );

        let exported_struct = &exported.structs[0];
        assert_eq!(exported_struct.name, "Holder");
        assert_eq!(exported_struct.visibility, XirVisibility::Public);
        assert_eq!(exported_struct.abilities, vec!["copy", "drop", "store"]);
        assert_eq!(exported_struct.type_parameters[0].name, "T");
        assert_eq!(exported_struct.fields[1].ty, Ty::I64);
        assert_eq!(exported.module.friends.len(), 1);

        // The export re-imports as a dependency, bodies absent and all.
        let round_tripped = parse_source_with_target(
            PathBuf::from("exported.xir.json"),
            String::new(),
            &serde_json::to_string(&exported).unwrap(),
            false,
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[round_tripped], &mut targets).unwrap();
        assert_eq!(env.get_module_count(), 1);
        assert_eq!(
            targets.get_funs().count(),
            0,
            "an interface is a dependency, not a compilation target"
        );
    }

    /// A type from another module is interned into `external_types` and
    /// addressed past the local structs, so the export is self-contained.
    #[test]
    fn foreign_types_are_interned_as_external_references() {
        let provider = document(
            "provider",
            serde_json::json!([{
                "name": "Token", "visibility": "public", "abilities": ["drop"],
                "fields": [{"name": "v", "ty": "u64"}],
            }]),
            serde_json::json!([]),
        );
        let consumer = serde_json::json!({
            "schema": XIR_SCHEMA,
            "version": XIR_VERSION,
            "module": {"address": "0x42", "name": "consumer", "dialect": "stackless"},
            "structs": [{
                "name": "Local", "visibility": "public", "abilities": ["drop"],
                "fields": [{"name": "v", "ty": "u64"}],
            }],
            "functions": [{
                "name": "takes", "visibility": "public", "is_entry": false,
                "is_native": false, "acquires": [], "params": 1,
                // Resource id 1 is the first external type.
                "locals": [{"struct": 1}], "returns": [],
                "blocks": empty_body(), "entry": 0, "loops": [],
                "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
            }],
            "external_types": [{"address": "0x42", "module": "provider", "name": "Token"}],
        });

        let env = model_of(&[provider, consumer]);
        let consumer_env = env
            .get_modules()
            .find(|module| {
                env.symbol_pool().string(module.get_name().name()).as_str() == "consumer"
            })
            .unwrap();
        let exported = export_interface(&consumer_env).unwrap();

        assert_eq!(exported.external_types.len(), 1);
        assert_eq!(exported.external_types[0].module, "provider");
        assert_eq!(exported.external_types[0].name, "Token");
        // One local struct, so the external type is addressed at index 1.
        assert_eq!(exported.structs.len(), 1);
        assert_eq!(exported.functions[0].locals[0], Ty::Struct(1));
    }
}
