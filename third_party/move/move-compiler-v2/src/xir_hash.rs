// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The **interface hash**: a digest over everything in an XIR interface that a
//! dependent can observe.
//!
//! Its contract is what makes an incremental build correct:
//!
//! - **Equal hashes ⇒ dependents need not rebuild.** So anything a dependent
//!   compiles against must be *in* the hash. A miss here is a stale build.
//! - **Unequal hashes ⇒ dependents rebuild.** So anything a dependent cannot
//!   observe must be *out*. A miss here is only wasted work, which is why the
//!   undecided cases below resolve toward inclusion.
//!
//! # Canonicalization
//!
//! `MODULAR_COMPILATION.md` §6 argues no canonicalization is needed, because
//! XIR types are structural rather than index-based the way a `.mv`
//! `SignatureToken` is. That is right about *types* and not the whole story.
//!
//! **Foreign type indices are a real hazard.** A `ResourceId` past the local
//! table indexes `external_structs`, which the exporter fills in *first-use*
//! order, so the same dependency can land at a different index in a different
//! build. Types are therefore rendered with foreign references resolved to
//! `address::module::name`, never as an index.
//!
//! **Declaration order is not — today.** The obvious worry is that an interface
//! lists declarations in `BTreeMap<StructId, _>` order, `StructId` wraps
//! `Symbol(usize)`, and symbol numbers are assigned at interning time, so order
//! should follow the source. Measurement says otherwise: reordering every
//! declaration in a module yields a byte-identical export, because expansion
//! holds module members in a `BTreeMap` keyed by the name *string*
//! (`unique_map.rs:15`) and interning inherits that order. Declarations are
//! nonetheless sorted here. It costs nothing, and it does not make a cache's
//! correctness depend on an incidental alignment between two unrelated maps —
//! one keyed by string, one by index — that no one is maintaining on purpose.
//! `tests/xir_hash_stability.rs` pins the alignment so its loss is visible.
//!
//! So this module hashes a *canonical projection*: declarations sorted by name,
//! set-like fields sorted, and foreign references resolved to names. Order is
//! preserved only where it carries meaning — field offsets, variant tags, and
//! type parameter positions.
//!
//! [`canonical_interface`] returns that projection, so a test that fails can
//! show *what* differed rather than only that two digests did.

use anyhow::{Context, Result};
use move_model_exchange::{
    Type as Ty, XirAttribute, XirAttributeArg, XirFunction, XirModule, XirStruct, XirVisibility,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// A digest of `module`'s observable interface, as uppercase hex.
pub fn interface_hash(module: &XirModule) -> Result<String> {
    let canonical = canonical_interface(module)?;
    // Serializing the projection rather than hand-feeding fields to the hasher
    // keeps the hashed bytes and the debuggable form the same artifact: a test
    // can print exactly what was hashed.
    let encoded = serde_json::to_vec(&canonical).context("encoding the canonical interface")?;
    Ok(format!("{:X}", Sha256::digest(&encoded)))
}

/// Combines per-module hashes into one digest for a whole package.
///
/// Sorted by module name, so the order modules were compiled in — which is not
/// fixed — cannot reach the result. Names are included alongside their hashes:
/// hashing only the digests would let a module *rename* go unnoticed, since a
/// rename leaves the per-module hash untouched when nothing else changed.
pub fn package_interface_hash<'a>(modules: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut entries = modules
        .into_iter()
        .map(|(name, hash)| format!("{name} {hash}"))
        .collect::<Vec<_>>();
    entries.sort();
    entries.dedup();
    let mut hasher = Sha256::new();
    for entry in entries {
        hasher.update(entry.as_bytes());
        // A separator, so that concatenation is unambiguous: without it,
        // `("ab", "c")` and `("a", "bc")` would hash alike.
        hasher.update([0u8]);
    }
    format!("{:X}", hasher.finalize())
}

/// The canonical projection of an interface — the exact input to the hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalModule {
    /// `address::name`.
    pub module: String,
    /// Sorted; a friend list is a set.
    pub friends: Vec<String>,
    /// Sorted by name.
    pub structs: Vec<CanonicalStruct>,
    /// Sorted by name.
    pub functions: Vec<CanonicalFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalStruct {
    pub name: String,
    pub visibility: String,
    /// Sorted; abilities are a set.
    pub abilities: Vec<String>,
    pub type_parameters: Vec<CanonicalTypeParameter>,
    /// **Order preserved**: a field's position is its offset, which callers
    /// depend on.
    pub fields: Vec<CanonicalField>,
    /// **Order preserved**: a variant's position is its tag.
    pub variants: Option<Vec<CanonicalVariant>>,
    /// Sorted; attribute order is not meaningful.
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalVariant {
    pub name: String,
    pub fields: Vec<CanonicalField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalField {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalTypeParameter {
    pub name: String,
    /// Sorted; a constraint set.
    pub abilities: Vec<String>,
    pub phantom: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CanonicalFunction {
    pub name: String,
    pub visibility: String,
    pub is_entry: bool,
    pub is_native: bool,
    pub type_parameters: Vec<CanonicalTypeParameter>,
    /// **Order preserved**: argument positions.
    pub params: Vec<String>,
    /// **Order preserved**: result positions.
    pub returns: Vec<String>,
    /// Sorted; `acquires` is a set.
    ///
    /// Included although it does not affect a caller's type-checking. §10.2
    /// leaves this open on the grounds that it may be unobservable; the cost of
    /// being wrong is asymmetric — including it can only cause a rebuild that
    /// was not needed, whereas excluding it risks a stale one — so it goes in.
    pub acquires: Vec<String>,
    /// Sorted.
    pub attributes: Vec<String>,
}

/// Builds the canonical projection.
///
/// Deliberately absent, beyond the obvious bodies (`blocks`, `loops`,
/// `locals`, `local_names`, `entry`) and `source_map`:
///
/// - **Specifications.** A dependent that only *compiles* against a module is
///   not coupled to its contract; one that *verifies* against it is. The prover
///   uses the monolithic path today (`MODULAR_COMPILATION.md` §10.4), so
///   excluding specs is correct now and becomes wrong the day the prover
///   consumes interfaces. That is the trigger to revisit this.
/// - **Private and inline functions.** The exporter never emits them, so there
///   is nothing to exclude here; noted so the two filters are not confused.
pub fn canonical_interface(module: &XirModule) -> Result<CanonicalModule> {
    let names = Names::new(module);

    let mut friends = module
        .friends
        .iter()
        .map(|friend| format!("{}::{}", friend.address, friend.module))
        .collect::<Vec<_>>();
    friends.sort();
    friends.dedup();

    let mut structs = module
        .structs
        .iter()
        .map(|decl| names.canonical_struct(decl))
        .collect::<Result<Vec<_>>>()?;
    structs.sort_by(|a, b| a.name.cmp(&b.name));

    let mut functions = module
        .functions
        .iter()
        .map(|decl| names.canonical_function(decl))
        .collect::<Result<Vec<_>>>()?;
    functions.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(CanonicalModule {
        module: format!("{}::{}", module.module.address, module.module.name),
        friends,
        structs,
        functions,
    })
}

/// Resolves resource ids to names, which is what makes the hash independent of
/// the order the exporter happened to intern foreign types in.
struct Names<'a> {
    module: &'a XirModule,
}

impl<'a> Names<'a> {
    fn new(module: &'a XirModule) -> Self {
        Self { module }
    }

    fn resource(&self, id: usize) -> Result<String> {
        if let Some(decl) = self.module.structs.get(id) {
            return Ok(decl.name.clone());
        }
        let external = self
            .module
            .external_structs
            .get(id - self.module.structs.len())
            .with_context(|| format!("resource id {id} is outside the declaration tables"))?;
        Ok(format!(
            "{}::{}::{}",
            external.address, external.module, external.name
        ))
    }

    /// Renders a type. Type parameters are rendered *positionally* (`#0`)
    /// rather than by their declared name: renaming a type parameter does not
    /// change what a caller may instantiate it with, so it is not interface.
    fn ty(&self, ty: &Ty) -> Result<String> {
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
            Ty::TypeParameter(index) => format!("#{index}"),
            Ty::Struct(id) | Ty::Enum(id) => self.resource(*id)?,
            Ty::StructInst(id, args) | Ty::EnumInst(id, args) => {
                format!("{}<{}>", self.resource(*id)?, self.ty_list(args)?.join(","))
            },
            Ty::Vector(element) => format!("vector<{}>", self.ty(element)?),
            Ty::Ref(referent) => format!("&{}", self.ty(referent)?),
            Ty::MutRef(referent) => format!("&mut {}", self.ty(referent)?),
            Ty::Fun(args, results, abilities) => {
                let mut abilities = abilities.clone();
                abilities.sort();
                format!(
                    "|{}|({}) has {}",
                    self.ty_list(args)?.join(","),
                    self.ty_list(results)?.join(","),
                    abilities.join("+")
                )
            },
        })
    }

    fn ty_list(&self, types: &[Ty]) -> Result<Vec<String>> {
        types.iter().map(|ty| self.ty(ty)).collect()
    }

    fn fields(&self, fields: &[move_model_exchange::Field]) -> Result<Vec<CanonicalField>> {
        fields
            .iter()
            .map(|field| {
                Ok(CanonicalField {
                    name: field.name.clone(),
                    ty: self.ty(&field.ty)?,
                })
            })
            .collect()
    }

    fn canonical_struct(&self, decl: &XirStruct) -> Result<CanonicalStruct> {
        let variants = decl
            .variants
            .as_ref()
            .map(|variants| {
                variants
                    .iter()
                    .map(|variant| {
                        Ok(CanonicalVariant {
                            name: variant.name.clone(),
                            fields: self.fields(&variant.fields)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?;
        Ok(CanonicalStruct {
            name: decl.name.clone(),
            visibility: visibility(decl.visibility),
            abilities: sorted(decl.abilities.clone()),
            type_parameters: type_parameters(&decl.type_parameters),
            fields: self.fields(&decl.fields)?,
            variants,
            attributes: attributes(&decl.attributes),
        })
    }

    fn canonical_function(&self, decl: &XirFunction) -> Result<CanonicalFunction> {
        let params = decl
            .locals
            .get(..decl.params)
            .context("function declares more parameters than locals")?;
        let mut acquires = decl
            .acquires
            .iter()
            .map(|id| self.resource(*id))
            .collect::<Result<Vec<_>>>()?;
        acquires.sort();
        acquires.dedup();
        Ok(CanonicalFunction {
            name: decl.name.clone(),
            visibility: visibility(decl.visibility),
            is_entry: decl.is_entry,
            is_native: decl.is_native,
            type_parameters: type_parameters(&decl.type_parameters),
            params: self.ty_list(params)?,
            returns: self.ty_list(&decl.returns)?,
            acquires,
            attributes: attributes(&decl.attributes),
        })
    }
}

fn visibility(visibility: XirVisibility) -> String {
    match visibility {
        XirVisibility::Private => "private",
        XirVisibility::Public => "public",
        XirVisibility::Friend => "friend",
    }
    .to_owned()
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn type_parameters(params: &[move_model_exchange::TypeParameter]) -> Vec<CanonicalTypeParameter> {
    params
        .iter()
        .map(|param| CanonicalTypeParameter {
            // The name *is* kept, unlike in a type position: it is part of the
            // rendered declaration a reader sees, and cheap to include.
            name: param.name.clone(),
            abilities: sorted(param.abilities.clone()),
            phantom: param.phantom,
        })
        .collect()
}

fn attributes(attributes: &[XirAttribute]) -> Vec<String> {
    sorted(
        attributes
            .iter()
            .map(|attribute| render_attribute(&attribute.name, &attribute.args))
            .collect(),
    )
}

fn render_attribute(name: &str, args: &[XirAttributeArg]) -> String {
    if args.is_empty() {
        return name.to_owned();
    }
    let args = args.iter().map(render_attribute_arg).collect::<Vec<_>>();
    format!("{name}({})", args.join(","))
}

fn render_attribute_arg(arg: &XirAttributeArg) -> String {
    match arg {
        XirAttributeArg::Name { name, args } => render_attribute(name, args),
        XirAttributeArg::Assign { assign, value } => {
            format!("{assign}={}", render_attribute_arg(value))
        },
        XirAttributeArg::Num { value } => value.clone(),
        XirAttributeArg::Bool { value } => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    /// A module exercising each hashed element once.
    fn interface() -> Value {
        json!({
            "schema": move_model_exchange::XIR_SCHEMA,
            "version": move_model_exchange::XIR_VERSION,
            "module": {"address": "0x42", "name": "m", "dialect": "stackless"},
            "friends": [{"address": "0x42", "module": "buddy"}],
            "structs": [
                {"name": "Alpha", "visibility": "public", "abilities": ["key", "store"],
                 "type_parameters": [{"name": "T", "abilities": ["drop"], "phantom": false}],
                 "fields": [{"name": "a", "ty": "u64"}, {"name": "b", "ty": {"type_parameter": 0}}],
                 "attributes": [{"name": "resource_group", "args": [
                     {"assign": "scope", "value": {"name": "global"}}]}]},
                {"name": "Beta", "visibility": "friend", "abilities": ["drop"],
                 "fields": [{"name": "x", "ty": {"struct": 2}}]},
            ],
            "functions": [
                {"name": "go", "visibility": "public", "is_entry": true, "is_native": false,
                 "acquires": [0], "params": 2,
                 "locals": ["u64", {"ref": {"struct": 2}}, "bool"],
                 "returns": ["u64"], "blocks": [], "entry": 0, "loops": [],
                 "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []},
                 "attributes": [{"name": "randomness", "args": [{"num": "7"}]}]},
                {"name": "also", "visibility": "friend", "is_entry": false, "is_native": true,
                 "acquires": [], "params": 0, "locals": [], "returns": [],
                 "blocks": [], "entry": 0, "loops": [],
                 "spec": {"requires": [], "modifies": [], "ensures": [], "aborts_if": []}},
            ],
            "external_structs": [
                {"address": "0x1", "module": "string", "name": "String"},
            ],
        })
    }

    fn hash_of(value: &Value) -> String {
        let module: XirModule = serde_json::from_value(value.clone()).unwrap();
        interface_hash(&module).unwrap()
    }

    /// Applies `mutate` to a fresh copy of the baseline and returns its hash.
    fn mutated(mutate: impl FnOnce(&mut Value)) -> String {
        let mut value = interface();
        mutate(&mut value);
        hash_of(&value)
    }

    /// The hash must not depend on declaration order.
    ///
    /// This is not hypothetical. An exported interface lists declarations in
    /// `BTreeMap<StructId, _>` order, `StructId` wraps `Symbol(usize)`, and a
    /// symbol's index is assigned when it is first interned — so the order
    /// tracks which source file happened to be parsed first. Two builds of
    /// identical source can legitimately differ here.
    #[test]
    fn declaration_order_does_not_affect_the_hash() {
        let baseline = hash_of(&interface());

        let reordered = mutated(|value| {
            // Reversing the struct table renumbers it, so every resource id
            // has to move with it — otherwise this would be a *semantic* edit
            // (`acquires: [0]` would come to name a different struct) rather
            // than the reordering under test. Locals 0 and 1 swap; the
            // external at 2 is unaffected.
            value["structs"].as_array_mut().unwrap().reverse();
            value["functions"][0]["acquires"] = json!([1]);

            value["functions"].as_array_mut().unwrap().reverse();
            value["friends"].as_array_mut().unwrap().reverse();
        });
        assert_eq!(
            baseline, reordered,
            "declaration order leaked into the hash"
        );
    }

    /// Nor on the index a foreign type happens to land at.
    ///
    /// `external_structs` is filled in first-use order during export, so the same
    /// dependency can sit at a different index in a different build. Adding an
    /// unreferenced entry ahead of the referenced one shifts every id.
    #[test]
    fn foreign_type_indices_do_not_affect_the_hash() {
        let baseline = hash_of(&interface());

        let shifted = mutated(|value| {
            // Insert an unused external type first, then repoint every
            // reference from id 2 to id 3.
            value["external_structs"].as_array_mut().unwrap().insert(
                0,
                json!({"address": "0x1", "module": "option", "name": "Option"}),
            );
            value["structs"][1]["fields"][0]["ty"] = json!({"struct": 3});
            value["functions"][0]["locals"][1] = json!({"ref": {"struct": 3}});
        });
        assert_eq!(
            baseline, shifted,
            "a foreign type's table position leaked into the hash"
        );
    }

    /// Everything a dependent can observe must move the hash.
    ///
    /// Each case is a real edit to the dependency's API. A case that stops
    /// changing the hash is a stale-build bug, which is the failure mode this
    /// digest exists to prevent.
    #[test]
    fn observable_changes_move_the_hash() {
        let baseline = hash_of(&interface());
        let cases: Vec<(&str, Box<dyn FnOnce(&mut Value)>)> = vec![
            (
                "module name",
                Box::new(|v: &mut Value| v["module"]["name"] = json!("other")),
            ),
            (
                "module address",
                Box::new(|v: &mut Value| v["module"]["address"] = json!("0x43")),
            ),
            (
                "friend added",
                Box::new(|v: &mut Value| {
                    v["friends"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"address": "0x42", "module": "extra"}))
                }),
            ),
            (
                "struct renamed",
                Box::new(|v: &mut Value| v["structs"][0]["name"] = json!("Renamed")),
            ),
            (
                "struct visibility",
                Box::new(|v: &mut Value| v["structs"][1]["visibility"] = json!("public")),
            ),
            (
                "ability removed",
                Box::new(|v: &mut Value| v["structs"][0]["abilities"] = json!(["key"])),
            ),
            (
                "field renamed",
                Box::new(|v: &mut Value| v["structs"][0]["fields"][0]["name"] = json!("renamed")),
            ),
            (
                "field type",
                Box::new(|v: &mut Value| v["structs"][0]["fields"][0]["ty"] = json!("u128")),
            ),
            (
                "field order",
                Box::new(|v: &mut Value| {
                    v["structs"][0]["fields"].as_array_mut().unwrap().reverse()
                }),
            ),
            (
                "type param phantom",
                Box::new(|v: &mut Value| {
                    v["structs"][0]["type_parameters"][0]["phantom"] = json!(true)
                }),
            ),
            (
                "type param constraint",
                Box::new(|v: &mut Value| {
                    v["structs"][0]["type_parameters"][0]["abilities"] = json!(["copy"])
                }),
            ),
            (
                "struct attribute",
                Box::new(|v: &mut Value| {
                    v["structs"][0]["attributes"] = json!([{"name": "other"}])
                }),
            ),
            (
                "function renamed",
                Box::new(|v: &mut Value| v["functions"][0]["name"] = json!("renamed")),
            ),
            (
                "function visibility",
                Box::new(|v: &mut Value| v["functions"][0]["visibility"] = json!("friend")),
            ),
            (
                "is_entry",
                Box::new(|v: &mut Value| v["functions"][0]["is_entry"] = json!(false)),
            ),
            (
                "is_native",
                Box::new(|v: &mut Value| v["functions"][1]["is_native"] = json!(false)),
            ),
            (
                "parameter type",
                Box::new(|v: &mut Value| v["functions"][0]["locals"][0] = json!("u128")),
            ),
            (
                "parameter count",
                Box::new(|v: &mut Value| v["functions"][0]["params"] = json!(1)),
            ),
            (
                "return type",
                Box::new(|v: &mut Value| v["functions"][0]["returns"] = json!(["bool"])),
            ),
            (
                "acquires",
                Box::new(|v: &mut Value| v["functions"][0]["acquires"] = json!([])),
            ),
            (
                "function attribute",
                Box::new(|v: &mut Value| v["functions"][0]["attributes"] = json!([])),
            ),
            (
                "foreign type identity",
                Box::new(|v: &mut Value| {
                    v["external_structs"][0] =
                        json!({"address": "0x1", "module": "ascii", "name": "String"})
                }),
            ),
        ];
        for (label, mutate) in cases {
            assert_ne!(
                baseline,
                mutated(mutate),
                "changing `{label}` left the interface hash unchanged"
            );
        }
    }

    /// What a dependent cannot observe must not move it.
    ///
    /// Every case here is something an author edits routinely; if it forced a
    /// rebuild of every dependent, the cache would rarely hit.
    #[test]
    fn unobservable_changes_do_not_move_the_hash() {
        let baseline = hash_of(&interface());
        let cases: Vec<(&str, Box<dyn FnOnce(&mut Value)>)> = vec![
            (
                "a function body",
                Box::new(|v: &mut Value| {
                    v["functions"][0]["blocks"] = json!([{"instrs": [], "term": {"ret": [0]}}])
                }),
            ),
            (
                "loop metadata",
                Box::new(|v: &mut Value| v["functions"][0]["loops"] = json!([])),
            ),
            (
                "a local beyond the parameters",
                Box::new(|v: &mut Value| {
                    v["functions"][0]["locals"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("u8"))
                }),
            ),
            (
                "local names",
                Box::new(|v: &mut Value| v["functions"][0]["local_names"] = json!(["x", "y", "z"])),
            ),
            (
                "a specification",
                Box::new(|v: &mut Value| {
                    v["functions"][0]["spec"]["ensures"] = json!([{"value": {"bool": true}}])
                }),
            ),
            (
                "an unreferenced foreign type",
                Box::new(|v: &mut Value| {
                    v["external_structs"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"address": "0x1", "module": "option", "name": "Option"}))
                }),
            ),
        ];
        for (label, mutate) in cases {
            assert_eq!(
                baseline,
                mutated(mutate),
                "changing `{label}` moved the interface hash"
            );
        }
    }

    #[test]
    fn the_package_hash_is_order_independent_and_name_sensitive() {
        let a = package_interface_hash([("0x1::a", "AA"), ("0x1::b", "BB")]);
        let reversed = package_interface_hash([("0x1::b", "BB"), ("0x1::a", "AA")]);
        assert_eq!(a, reversed, "module order leaked into the package hash");

        // A rename leaves each module's own hash untouched, so the package
        // hash has to notice it.
        let renamed = package_interface_hash([("0x1::a", "AA"), ("0x1::c", "BB")]);
        assert_ne!(
            a, renamed,
            "a module rename left the package hash unchanged"
        );

        // And the separator does its job.
        assert_ne!(
            package_interface_hash([("0x1::a", "BCC")]),
            package_interface_hash([("0x1::ab", "CC")]),
        );
    }

    /// The projection is what gets hashed, so it is worth being able to read.
    #[test]
    fn the_canonical_projection_is_sorted_and_resolved() {
        let module: XirModule = serde_json::from_value(interface()).unwrap();
        let canonical = canonical_interface(&module).unwrap();
        assert_eq!(canonical.module, "0x42::m");
        assert_eq!(
            canonical
                .structs
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Beta"]
        );
        assert_eq!(
            canonical
                .functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            vec!["also", "go"],
            "functions are sorted by name, not left in declaration order"
        );
        // The foreign type is named, not indexed.
        assert_eq!(canonical.structs[1].fields[0].ty, "0x1::string::String");
        // A type parameter is positional in a type position.
        assert_eq!(canonical.structs[0].fields[1].ty, "#0");
        // Abilities are sorted.
        assert_eq!(canonical.structs[0].abilities, vec!["key", "store"]);
        // `acquires` resolved to a name.
        assert_eq!(canonical.functions[1].acquires, vec!["Alpha"]);
    }
}
