// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Exports a compiled Move module as XIR.
//!
//! This is the inverse of [`crate::xir`], which reads XIR into the model. Until
//! now XIR had only one producer — the Lean toolchain — so the reader could
//! only be exercised against hand-written documents. With an exporter the two
//! directions compose, and a Move module can be checked by round-tripping it:
//!
//! ```text
//! source -> stackless -> bytecode                     (direct)
//! source -> stackless -> XIR -> stackless -> bytecode (round trip)
//! ```
//!
//! # What is exported
//!
//! Declarations come from the model; code comes from the stackless bytecode of
//! [`FunctionVariant::Baseline`]. Baseline matters: later variants carry
//! operations the prover pipeline inserts (`Havoc`, `WriteBack`, the `Trace`
//! family) which describe a memory model rather than a program, and which XIR
//! deliberately does not represent.
//!
//! # What is deliberately not exported
//!
//! The assignment kind is dropped. XIR's `assign` means "write the source's
//! value to the destination", which is what the Lean semantics proves; whether
//! that is a copy or a move is a conclusion Move's ability system draws from
//! liveness and declared abilities, and `ability_processor` redraws it on
//! import. Crucially the copy ability is still enforced either way: the arm
//! that infers a copy calls the same `check_copy` as the explicit one, so a
//! type without `copy` cannot be copied by omission.
//!
//! The corpus sweep settled the one case where that is not enough. A local
//! assigned to itself carries no liveness signal to re-infer from — the source
//! is read after the instruction, so inference resolves to a move and the
//! reader rejects moving a local into itself. That form is refused rather than
//! exported; everything else the corpus reaches round-trips byte-identically.

use anyhow::{bail, Context, Result};
use move_binary_format::file_format::Visibility as MoveVisibility;
use move_core_types::ability::AbilitySet;
use move_model::{
    ast::{Attribute, AttributeValue, Value as ModelValue},
    model::{FunId, FunctionEnv, GlobalEnv, ModuleEnv, ModuleId, QualifiedId, StructEnv, StructId},
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_model_exchange::{
    Block, Contract, Field, Instr, IntType, Oper, Term, Type as Ty,
    TypeParameter as TypeParameterDecl, Value as Constant, Variant, XirAttribute, XirAttributeArg,
    XirDialect, XirExternalFunction, XirExternalStruct, XirFunction as FunctionDecl, XirModule,
    XirModuleMetadata, XirStruct as StructDecl, XirVisibility, XIR_SCHEMA, XIR_VERSION,
};
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{Bytecode, Constant as StacklessConstant, Label, Operation},
    stackless_control_flow_graph::{BlockContent, StacklessControlFlowGraph},
};
use std::collections::BTreeMap;

/// The local and foreign declarations a module refers to.
///
/// XIR addresses structs and functions by a single index: ids below the local
/// count select a declaration of this module, and ids at or above it select an
/// entry of the corresponding external table after subtracting that count. The
/// external tables are therefore built as references are encountered, which is
/// why every type conversion below takes this by `&mut`.
struct References {
    module_id: ModuleId,
    local_structs: Vec<StructId>,
    external_structs: Vec<XirExternalStruct>,
    local_functions: Vec<FunId>,
    external_functions: Vec<XirExternalFunction>,
}

/// The functions a document carries, in document order.
///
/// An inline function is expanded at its call sites and never compiled on its
/// own, so it has no body to describe and is absent from the compiled module.
/// It is therefore absent from the document too — and a function id is a
/// position in exactly this sequence, so this is the single place that decides
/// it. Deriving the reference table from the unfiltered list instead gave
/// every declaration after an inline one an id one too high, which resolved a
/// call to a different function of the same module.
fn exported_functions<'a>(module: &'a ModuleEnv) -> impl Iterator<Item = FunctionEnv<'a>> + 'a {
    module.get_functions().filter(|fun| !fun.is_inline())
}

impl References {
    fn new(module: &ModuleEnv) -> Self {
        Self {
            module_id: module.get_id(),
            local_structs: module.get_structs().map(|s| s.get_id()).collect(),
            external_structs: vec![],
            local_functions: exported_functions(module).map(|f| f.get_id()).collect(),
            external_functions: vec![],
        }
    }

    /// Resolves a function to its XIR id, interning a foreign one.
    fn function_id(&mut self, env: &GlobalEnv, qid: QualifiedId<FunId>) -> usize {
        if qid.module_id == self.module_id {
            if let Some(index) = self.local_functions.iter().position(|id| *id == qid.id) {
                return index;
            }
        }
        let fun = env.get_function(qid);
        let reference = XirExternalFunction {
            address: fun
                .module_env
                .self_address()
                .expect_numerical()
                .to_hex_literal(),
            module: fun
                .module_env
                .get_name()
                .name()
                .display(env.symbol_pool())
                .to_string(),
            function: fun.get_name().display(env.symbol_pool()).to_string(),
        };
        let existing = self
            .external_functions
            .iter()
            .position(|candidate| *candidate == reference);
        self.local_functions.len()
            + existing.unwrap_or_else(|| {
                self.external_functions.push(reference);
                self.external_functions.len() - 1
            })
    }

    /// Resolves a struct to its XIR id, interning a foreign one.
    fn struct_id(&mut self, env: &GlobalEnv, qid: QualifiedId<StructId>) -> usize {
        if qid.module_id == self.module_id {
            if let Some(index) = self.local_structs.iter().position(|id| *id == qid.id) {
                return index;
            }
        }
        let struct_env = env.get_struct(qid);
        let reference = XirExternalStruct {
            address: struct_env
                .module_env
                .self_address()
                .expect_numerical()
                .to_hex_literal(),
            module: struct_env
                .module_env
                .get_name()
                .name()
                .display(env.symbol_pool())
                .to_string(),
            name: struct_env.get_name().display(env.symbol_pool()).to_string(),
        };
        let existing = self
            .external_structs
            .iter()
            .position(|candidate| *candidate == reference);
        self.local_structs.len()
            + existing.unwrap_or_else(|| {
                self.external_structs.push(reference);
                self.external_structs.len() - 1
            })
    }
}

/// Exports `module` and the bodies of its functions.
///
/// Structs and functions are walked against one [`References`]: the external
/// tables are built as declarations are met, so exporting the two against
/// separate tables would leave the struct field types indexing a table the
/// document does not carry.
///
/// `module` must be one the checking pipeline treated as a target.
/// `acquires_checker` populates the acquired-resource set only for those
/// (`module.is_target()`), and a function without one exports as acquiring
/// nothing — which reads as a fact rather than as missing information.
pub fn export_module(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    module: &ModuleEnv,
) -> Result<XirModule> {
    let refs = &mut References::new(module);
    // Friendship is module-level state that bodies do not imply, so unlike a
    // call graph it cannot be recovered on import. This schema has nowhere to
    // put it, and a document that quietly lost it would reimport as a module
    // whose friends may no longer call it — compiling cleanly into different
    // linkage. Refuse instead.
    if !module.get_friend_decls().is_empty() {
        bail!(
            "`{}` declares friends, which this XIR schema version cannot carry",
            module.get_full_name_str()
        );
    }
    // `XirModuleMetadata` carries address, name and dialect and nothing else,
    // so a module attribute has nowhere to go — the same reason friends and
    // module specifications are refused rather than dropped.
    if !module.get_attributes().is_empty() {
        bail!(
            "`{}` has module attributes, which this XIR schema version cannot carry",
            module.get_full_name_str()
        );
    }
    // A module invariant binds every function of the module, and the document
    // has no slot for it at all.
    if !module.get_spec().conditions.is_empty() {
        bail!(
            "`{}` has a module specification, which this exporter cannot yet translate",
            module.get_full_name_str()
        );
    }
    let structs = module
        .get_structs()
        .map(|s| export_struct(env, &s, refs))
        .collect::<Result<Vec<_>>>()?;
    let functions = exported_functions(module)
        .map(|f| export_function(env, targets, &f, refs))
        .collect::<Result<Vec<_>>>()?;
    Ok(XirModule {
        schema: XIR_SCHEMA.to_owned(),
        version: XIR_VERSION,
        module: XirModuleMetadata {
            address: module.self_address().expect_numerical().to_hex_literal(),
            name: module
                .get_name()
                .name()
                .display(env.symbol_pool())
                .to_string(),
            dialect: XirDialect::Stackless,
        },
        structs,
        functions,
        external_functions: std::mem::take(&mut refs.external_functions),
        external_structs: std::mem::take(&mut refs.external_structs),
    })
}

fn export_struct(
    env: &GlobalEnv,
    struct_env: &StructEnv,
    refs: &mut References,
) -> Result<StructDecl> {
    let name = struct_env.get_name().display(env.symbol_pool()).to_string();
    // `XirStruct` has no visibility, and the reader rebuilds every struct as
    // private. A public struct gets compiler-generated pack, unpack and
    // field-borrow functions (`module_generator.rs`, gated on
    // `is_public_or_friend`), so importing one back as private drops that API
    // from the module while leaving it verifiable.
    if struct_env.get_visibility() != MoveVisibility::Private {
        bail!("`{name}` is not private, and struct visibility has no XIR form");
    }
    // A struct invariant is an obligation a consumer would prove against, and
    // `XirStruct` has nowhere to put it — the same reason a function contract
    // is refused.
    if struct_env.has_conditions() {
        bail!("`{name}` has a specification, which this exporter cannot yet translate");
    }
    let (fields, variants) = if struct_env.has_variants() {
        let variants = struct_env
            .get_variants()
            .map(|variant| {
                Ok(Variant {
                    name: variant.display(env.symbol_pool()).to_string(),
                    fields: struct_env
                        .get_fields_of_variant(variant)
                        .map(|field| export_field(env, &field, refs))
                        .collect::<Result<Vec<_>>>()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        (vec![], Some(variants))
    } else {
        let fields = struct_env
            .get_fields()
            .map(|field| export_field(env, &field, refs))
            .collect::<Result<Vec<_>>>()?;
        (fields, None)
    };
    Ok(StructDecl {
        name,
        abilities: abilities(struct_env.get_abilities()),
        type_parameters: type_parameters(env, struct_env.get_type_parameters()),
        fields,
        variants,
        attributes: attributes(env, struct_env.get_attributes())?,
    })
}

fn export_field(
    env: &GlobalEnv,
    field: &move_model::model::FieldEnv,
    refs: &mut References,
) -> Result<Field> {
    let name = field.get_name().display(env.symbol_pool()).to_string();
    // A positional struct names its fields `0`, `1`, ... in the model, which
    // is not a Move identifier. The reader checks field names against
    // `Identifier::is_valid` and refuses the document, so emitting one would
    // report success for something unloadable. The file-format generator
    // prefixes `_` (`module_generator.rs`), but the schema does not say that
    // is the encoding, and inventing it here would make the reader's model
    // disagree with the source about the field's name.
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        bail!("positional struct fields have no XIR form");
    }
    Ok(Field {
        name,
        ty: export_type(env, &field.get_type(), refs)?,
    })
}

/// Exports a function's signature, leaving its body empty.
fn export_function_declaration(
    env: &GlobalEnv,
    fun: &FunctionEnv,
    refs: &mut References,
) -> Result<FunctionDecl> {
    // XIR carries a contract, and this exporter has no translation for the
    // model's specification expressions yet. An empty `Contract` beside a real
    // `aborts_if` is not a lossy export but a false one: a consumer proving
    // against the document would see no obligations and report success. The
    // round trip cannot catch it either, since the reader ignores the field
    // and a contract never reaches the bytecode.
    if !fun.get_spec().conditions.is_empty() {
        bail!(
            "`{}` has a specification, which this exporter cannot yet translate",
            fun.get_full_name_str()
        );
    }

    let parameters = fun.get_parameters();
    let locals = parameters
        .iter()
        .map(|parameter| export_type(env, &parameter.1, refs))
        .collect::<Result<Vec<_>>>()?;
    // `flatten` splits a tuple return into its components and leaves anything
    // else alone, which is exactly the shape XIR wants.
    let returns = fun
        .get_result_type()
        .flatten()
        .iter()
        .map(|ty| export_type(env, ty, refs))
        .collect::<Result<Vec<_>>>()?;
    Ok(FunctionDecl {
        name: fun.get_name().display(env.symbol_pool()).to_string(),
        type_parameters: type_parameters(env, &fun.get_type_parameters()),
        visibility: match fun.visibility() {
            MoveVisibility::Public => XirVisibility::Public,
            MoveVisibility::Friend => XirVisibility::Friend,
            MoveVisibility::Private => XirVisibility::Private,
        },
        is_entry: fun.is_entry(),
        is_native: fun.is_native(),
        // `get_acquires_global_resources` reads the compiled module, which
        // does not exist yet, and `FunctionData::acquires_global_resources`
        // is left empty by compiler v2's generator — both yield nothing here.
        // The declared set lives in `get_acquired_structs`, and it is not
        // cosmetic: `reference_safety_processor_v3` reads it to reject a call
        // that acquires a resource the caller has mutably borrowed.
        acquires: {
            let module_id = refs.module_id;
            fun.get_acquired_structs()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|sid| refs.struct_id(env, module_id.qualified(sid)))
                .collect()
        },
        params: parameters.len(),
        locals,
        local_names: parameters
            .iter()
            .map(|parameter| Some(parameter.0.display(env.symbol_pool()).to_string()))
            .collect(),
        returns,

        blocks: vec![],
        entry: 0,
        loops: vec![],
        spec: Contract {
            requires: vec![],
            aborts_if: vec![],
            ensures: vec![],
            modifies: vec![],
        },
        attributes: attributes(env, fun.get_attributes())?,
        source_map: None,
    })
}

/// Converts the attributes of a declaration.
fn attributes(env: &GlobalEnv, attrs: &[Attribute]) -> Result<Vec<XirAttribute>> {
    attrs.iter().map(|attr| attribute(env, attr)).collect()
}

/// Converts one attribute.
///
/// The reader rebuilds an assignment only from a lone numeric or boolean
/// argument (`xir.rs`, `model_attribute_apply`), so the remaining assigned
/// forms — an address, a name path — have no XIR spelling. They are refused
/// rather than reshaped into something that reads back as a different
/// attribute. `#[persistent]` and `#[module_lock]` reach the file format
/// (`module_generator.rs`, `function_attributes`), so dropping any of this
/// silently would change the compiled module.
fn attribute(env: &GlobalEnv, attr: &Attribute) -> Result<XirAttribute> {
    let name = |symbol: &Symbol| symbol.display(env.symbol_pool()).to_string();
    Ok(match attr {
        Attribute::Apply(_, symbol, args) => XirAttribute {
            name: name(symbol),
            args: args
                .iter()
                .map(|arg| attribute_arg(env, arg))
                .collect::<Result<Vec<_>>>()?,
        },
        Attribute::Assign(_, symbol, AttributeValue::Value(_, ModelValue::Number(value))) => {
            XirAttribute {
                name: name(symbol),
                args: vec![XirAttributeArg::Num {
                    value: value.to_string(),
                }],
            }
        },
        Attribute::Assign(_, symbol, AttributeValue::Value(_, ModelValue::Bool(value))) => {
            XirAttribute {
                name: name(symbol),
                args: vec![XirAttributeArg::Bool { value: *value }],
            }
        },
        Attribute::Assign(_, symbol, _) => bail!(
            "attribute `{}` is assigned a value with no XIR form",
            name(symbol)
        ),
    })
}

/// Converts a nested attribute argument.
///
/// Only an application nests: the reader refuses a bare literal in argument
/// position, so an inner assignment such as `#[event(scope = 1)]` cannot be
/// carried.
fn attribute_arg(env: &GlobalEnv, attr: &Attribute) -> Result<XirAttributeArg> {
    match attr {
        Attribute::Apply(_, symbol, args) => Ok(XirAttributeArg::Name {
            name: symbol.display(env.symbol_pool()).to_string(),
            args: args
                .iter()
                .map(|arg| attribute_arg(env, arg))
                .collect::<Result<Vec<_>>>()?,
        }),
        Attribute::Assign(_, symbol, _) => bail!(
            "attribute argument `{}` assigns, which XIR carries only at the top level",
            symbol.display(env.symbol_pool())
        ),
    }
}

/// `Ability`'s `Display` spells these the way `parse_ability_set` reads them.
/// Refuses an operation on a struct belonging to another module.
///
/// These operations carry no struct id: the reader recovers it from the
/// operand or destination type, and `struct_from_type` resolves only structs
/// of the module being translated. Move 2.4's `public struct` lets a
/// dependency's struct be packed, destructured and read here, and such a
/// document exports cleanly and then cannot be loaded — the id space that
/// would name it is exactly what the operation drops.
fn local_struct(refs: &References, mid: ModuleId, what: &str) -> Result<()> {
    if mid != refs.module_id {
        bail!("`{what}` on another module's struct has no XIR form");
    }
    Ok(())
}

fn abilities(set: AbilitySet) -> Vec<String> {
    set.iter().map(|ability| ability.to_string()).collect()
}

fn type_parameters(
    env: &GlobalEnv,
    parameters: &[move_model::model::TypeParameter],
) -> Vec<TypeParameterDecl> {
    parameters
        .iter()
        .map(|parameter| TypeParameterDecl {
            name: parameter.0.display(env.symbol_pool()).to_string(),
            abilities: abilities(parameter.1.abilities),
            phantom: parameter.1.is_phantom,
        })
        .collect()
}

/// Converts a model type, interning any foreign struct it names.
fn export_type(env: &GlobalEnv, ty: &Type, refs: &mut References) -> Result<Ty> {
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
            // `Num`, `Range` and the event-store types exist only in
            // specifications, which this format does not carry.
            other => bail!("`{other:?}` has no XIR form; it is specification-only"),
        },
        Type::Vector(element) => Ty::Vector(Box::new(export_type(env, element, refs)?)),
        Type::Reference(ReferenceKind::Immutable, referent) => {
            Ty::Ref(Box::new(export_type(env, referent, refs)?))
        },
        Type::Reference(ReferenceKind::Mutable, referent) => {
            Ty::MutRef(Box::new(export_type(env, referent, refs)?))
        },
        Type::TypeParameter(index) => Ty::TypeParameter(*index as usize),
        Type::Struct(module_id, struct_id, type_args) => {
            let qid = module_id.qualified(*struct_id);
            let is_enum = env.get_struct(qid).has_variants();
            let id = refs.struct_id(env, qid);
            let args = type_args
                .iter()
                .map(|arg| export_type(env, arg, refs))
                .collect::<Result<Vec<_>>>()?;
            match (is_enum, args.is_empty()) {
                (true, true) => Ty::Enum(id),
                (true, false) => Ty::EnumInst(id, args),
                (false, true) => Ty::Struct(id),
                (false, false) => Ty::StructInst(id, args),
            }
        },
        // Function values arrived in language version 2.2; this schema carries
        // no type for them, so a signature naming one cannot be exported.
        Type::Fun(..) => bail!("function value types have no XIR form"),
        other => bail!("`{other:?}` has no XIR form"),
    })
}

// ---------------------------------------------------------------------------
// Bodies
// ---------------------------------------------------------------------------

/// Exports `module` with function bodies, from the baseline stackless target.
fn export_function(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    fun: &FunctionEnv,
    refs: &mut References,
) -> Result<FunctionDecl> {
    let mut decl = export_function_declaration(env, fun, refs)?;
    // Only a native may be bodyless. It reaches here either with no baseline
    // target or with a target whose code is empty, and the second has to be
    // checked separately because `StacklessControlFlowGraph::new_forward`
    // looks up offset 0 unconditionally and panics on an empty body.
    //
    // For anything else a missing body means the holder does not describe this
    // module. Returning the declaration would produce a document the reader
    // refuses — it rejects a non-native with no blocks — so the exporter would
    // be reporting success for output it cannot read back.
    let body = targets
        .get_data(&fun.get_qualified_id(), &FunctionVariant::Baseline)
        .filter(|data| !data.code.is_empty());
    let Some(data) = body else {
        if fun.is_native() {
            return Ok(decl);
        }
        bail!(
            "`{}` has no stackless body to export",
            fun.get_full_name_str()
        );
    };

    // The declaration only knows the parameters; the body introduces the rest.
    decl.locals = data
        .local_types
        .iter()
        .map(|ty| export_type(env, ty, refs))
        .collect::<Result<Vec<_>>>()?;
    decl.local_names = (0..data.local_types.len())
        .map(|index| {
            data.local_names
                .get(&index)
                .map(|name| name.display(env.symbol_pool()).to_string())
        })
        .collect();
    // `acquires` stays as the declaration set it: compiler v2 leaves
    // `FunctionData::acquires_global_resources` empty, so overriding here
    // would replace the declared resources with nothing.

    let (blocks, entry) = export_blocks(env, data, refs)?;
    decl.blocks = blocks;
    decl.entry = entry;
    Ok(decl)
}

/// Partitions the flat instruction list into XIR's basic blocks.
///
/// Stackless code is a list with `Label`/`Jump`/`Branch`; XIR wants blocks that
/// each end in a terminator. `StacklessControlFlowGraph` already computes that
/// partition, so this maps its blocks into code-layout order — XIR requires the
/// entry to be block `0` and ids to follow layout — and synthesizes a `Jump`
/// where a block falls through to its successor.
fn export_blocks(
    env: &GlobalEnv,
    data: &FunctionData,
    refs: &mut References,
) -> Result<(Vec<Block>, usize)> {
    let cfg = StacklessControlFlowGraph::new_forward(&data.code);
    let mut ranges = vec![];
    for block in cfg.blocks() {
        if let BlockContent::Basic { lower, upper } = *cfg.content(block) {
            ranges.push((lower, upper));
        }
    }
    ranges.sort_by_key(|(lower, _)| *lower);

    // Every jump names a label; map each label to the block it opens.
    let mut block_of_label = BTreeMap::new();
    for (index, (lower, _)) in ranges.iter().enumerate() {
        if let Bytecode::Label(_, label) = &data.code[*lower as usize] {
            block_of_label.insert(*label, index);
        }
    }
    let block_of = |label: &Label| -> Result<usize> {
        block_of_label
            .get(label)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("jump to a label that opens no block"))
    };

    let mut blocks = Vec::with_capacity(ranges.len());
    for (index, (lower, upper)) in ranges.iter().enumerate() {
        let mut instrs = vec![];
        let mut term = None;
        for offset in *lower..=*upper {
            let bytecode = &data.code[offset as usize];
            match bytecode {
                Bytecode::Label(..) => continue,
                Bytecode::Ret(_, srcs) => term = Some(Term::Ret(srcs.clone())),
                Bytecode::Jump(_, label) => term = Some(Term::Jump(block_of(label)?)),
                Bytecode::Branch(_, then_label, else_label, cond) => {
                    term = Some(Term::Branch(
                        *cond,
                        block_of(then_label)?,
                        block_of(else_label)?,
                    ))
                },
                // `Term::Abort` carries a code and nothing else. A second
                // operand is the abort message, which compiler v2 fills in the
                // baseline variant for `abort(v)` and the file-format
                // generator turns into `AbortMsg` rather than `Abort`
                // (`function_generator.rs`), so dropping it changes the
                // compiled module.
                Bytecode::Abort(_, code, message) => {
                    if message.is_some() {
                        bail!("abort messages have no XIR form");
                    }
                    term = Some(Term::Abort(*code))
                },
                // An inline specification emits no instruction of its own, but
                // dropping one is not neutral: it references locals, which
                // keeps them live, which changes coalescing downstream.
                // Measured on `bytecode-generator/inline_specs.move`, omitting
                // them moved the module from 180 bytes to 174. Refuse instead.
                //
                // This is also where a loop invariant is caught. It becomes a
                // `Prop` recorded in `FunctionData::loop_invariants` only under
                // the model's own stackless generator; compiler v2's reaches
                // here as a spec block, so `Loop::invariants` staying empty
                // never hides one.
                Bytecode::SpecBlock(..) => {
                    bail!("inline specifications have no XIR form")
                },
                other => instrs.push(export_instruction(env, other, data, refs)?),
            }
        }
        // No terminator means control falls into the next block.
        let term = match term {
            Some(term) => term,
            None => Term::Jump(index + 1),
        };
        blocks.push(Block { instrs, term });
    }
    Ok((blocks, 0))
}

fn export_instruction(
    env: &GlobalEnv,
    bytecode: &Bytecode,
    data: &FunctionData,
    refs: &mut References,
) -> Result<Instr> {
    Ok(match bytecode {
        // The assign kind is deliberately dropped; see the module header. The
        // exception is a local assigned to itself, where the kind is the only
        // thing distinguishing the two readings and the source stays live
        // across the instruction. Inference resolves it to a move, and moving
        // a local into itself while it is still read is rejected — so the
        // document would not load. Refuse here, where the construct has a
        // name. `variable-coalescing/dead_assignment_2.move` is the case.
        Bytecode::Assign(_, dst, src, _) if dst == src => {
            bail!("a local assigned to itself has no XIR form")
        },
        Bytecode::Assign(_, dst, src, _) => Instr::Assign(*dst, *src),
        Bytecode::Load(_, dst, constant) => Instr::Load(*dst, export_constant(constant)?),
        Bytecode::Nop(_) => Instr::Nop,
        Bytecode::Call(_, dsts, operation, srcs, _) => Instr::Call(
            dsts.clone(),
            export_operation(env, operation, srcs, data, refs)?,
            srcs.clone(),
        ),
        other => bail!("`{other:?}` has no XIR instruction form"),
    })
}

fn export_constant(constant: &StacklessConstant) -> Result<Constant> {
    Ok(match constant {
        StacklessConstant::Bool(value) => Constant::Bool(*value),
        StacklessConstant::U8(value) => Constant::Num(value.to_string()),
        StacklessConstant::U16(value) => Constant::Num(value.to_string()),
        StacklessConstant::U32(value) => Constant::Num(value.to_string()),
        StacklessConstant::U64(value) => Constant::Num(value.to_string()),
        StacklessConstant::U128(value) => Constant::Num(value.to_string()),
        StacklessConstant::U256(value) => Constant::Num(value.to_string()),
        StacklessConstant::I8(value) => Constant::Num(value.to_string()),
        StacklessConstant::I16(value) => Constant::Num(value.to_string()),
        StacklessConstant::I32(value) => Constant::Num(value.to_string()),
        StacklessConstant::I64(value) => Constant::Num(value.to_string()),
        StacklessConstant::I128(value) => Constant::Num(value.to_string()),
        StacklessConstant::I256(value) => Constant::Num(value.to_string()),
        StacklessConstant::Address(address) => {
            Constant::Address(address.expect_numerical().to_hex_literal())
        },
        // The schema has a `Vector` value, but the reader refuses to load one
        // (`xir.rs`, "vector values are not valid XIR load constants"), so
        // emitting these would produce a document that cannot be read back.
        // Refuse here instead, where the construct still has a name.
        StacklessConstant::ByteArray(_) => bail!("byte string constants have no XIR form"),
        StacklessConstant::AddressArray(_) => bail!("address vector constants have no XIR form"),
        StacklessConstant::Vector(_) => bail!("vector constants have no XIR form"),
    })
}

/// The index of `variant` among a struct's variants.
///
/// Stackless names a variant by symbol; XIR addresses it by position.
fn variant_index(env: &GlobalEnv, qid: QualifiedId<StructId>, variant: Symbol) -> Result<usize> {
    env.get_struct(qid)
        .get_variants()
        .position(|candidate| candidate == variant)
        .with_context(|| {
            format!(
                "`{}` has no variant `{}`",
                env.get_struct(qid).get_full_name_str(),
                variant.display(env.symbol_pool())
            )
        })
}

/// The numeric width an arithmetic operation works at.
///
/// Stackless arithmetic is untyped — one `Add` for every width — while XIR
/// annotates each operation with its type, so the width is recovered from the
/// operand. Signed and unsigned share an operation family, differing only in
/// the range consulted, which is exactly why XIR carries the type.
fn int_type(data: &FunctionData, temp: usize) -> Result<IntType> {
    let ty = data
        .local_types
        .get(temp)
        .context("operand is not a local of this function")?;
    Ok(match ty {
        Type::Primitive(PrimitiveType::U8) => IntType::U8,
        Type::Primitive(PrimitiveType::U16) => IntType::U16,
        Type::Primitive(PrimitiveType::U32) => IntType::U32,
        Type::Primitive(PrimitiveType::U64) => IntType::U64,
        Type::Primitive(PrimitiveType::U128) => IntType::U128,
        Type::Primitive(PrimitiveType::U256) => IntType::U256,
        Type::Primitive(PrimitiveType::I8) => IntType::I8,
        Type::Primitive(PrimitiveType::I16) => IntType::I16,
        Type::Primitive(PrimitiveType::I32) => IntType::I32,
        Type::Primitive(PrimitiveType::I64) => IntType::I64,
        Type::Primitive(PrimitiveType::I128) => IntType::I128,
        Type::Primitive(PrimitiveType::I256) => IntType::I256,
        other => bail!("`{other:?}` is not a numeric type"),
    })
}

fn export_operation(
    env: &GlobalEnv,
    operation: &Operation,
    srcs: &[usize],
    data: &FunctionData,
    refs: &mut References,
) -> Result<Oper> {
    // The width of an arithmetic operation comes from its first operand.
    let width = || int_type(data, *srcs.first().context("operation has no operands")?);
    let types = |refs: &mut References, args: &[Type]| {
        args.iter()
            .map(|ty| export_type(env, ty, refs))
            .collect::<Result<Vec<_>>>()
    };
    Ok(match operation {
        Operation::Function(mid, fid, args) => {
            let id = refs.function_id(env, mid.qualified(*fid));
            if args.is_empty() {
                Oper::Function(id)
            } else {
                Oper::FunctionInst(id, types(refs, args)?)
            }
        },
        Operation::Pack(mid, sid, args) => {
            local_struct(refs, *mid, "pack")?;
            let _ = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::Pack
            } else {
                Oper::PackInst(types(refs, args)?)
            }
        },
        Operation::Unpack(mid, sid, args) => {
            local_struct(refs, *mid, "unpack")?;
            let _ = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::Unpack
            } else {
                Oper::UnpackInst(types(refs, args)?)
            }
        },
        Operation::MoveTo(mid, sid, args) => {
            let id = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::MoveTo(id)
            } else {
                Oper::MoveToInst(id, types(refs, args)?)
            }
        },
        Operation::MoveFrom(mid, sid, args) => {
            let id = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::MoveFrom(id)
            } else {
                Oper::MoveFromInst(id, types(refs, args)?)
            }
        },
        Operation::Exists(mid, sid, args) => {
            let id = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::Exists(id)
            } else {
                Oper::ExistsInst(id, types(refs, args)?)
            }
        },
        Operation::BorrowGlobal(mid, sid, args) => {
            let id = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::BorrowGlobal(id)
            } else {
                Oper::BorrowGlobalInst(id, types(refs, args)?)
            }
        },
        Operation::GetGlobal(mid, sid, args) => {
            let id = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::GetGlobal(id)
            } else {
                Oper::GetGlobalInst(id, types(refs, args)?)
            }
        },
        Operation::BorrowField(mid, sid, args, offset) => {
            local_struct(refs, *mid, "borrow_field")?;
            let _ = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::BorrowField(*offset)
            } else {
                Oper::BorrowFieldInst(*offset, types(refs, args)?)
            }
        },
        Operation::GetField(mid, sid, args, offset) => {
            local_struct(refs, *mid, "get_field")?;
            let _ = refs.struct_id(env, mid.qualified(*sid));
            if args.is_empty() {
                Oper::GetField(*offset)
            } else {
                Oper::GetFieldInst(*offset, types(refs, args)?)
            }
        },
        Operation::PackVariant(mid, sid, variant, args) => {
            local_struct(refs, *mid, "pack_variant")?;
            let index = variant_index(env, mid.qualified(*sid), *variant)?;
            if args.is_empty() {
                Oper::PackVariant(index)
            } else {
                Oper::PackVariantInst(index, types(refs, args)?)
            }
        },
        Operation::UnpackVariant(mid, sid, variant, args) => {
            local_struct(refs, *mid, "unpack_variant")?;
            let index = variant_index(env, mid.qualified(*sid), *variant)?;
            if args.is_empty() {
                Oper::UnpackVariant(index)
            } else {
                Oper::UnpackVariantInst(index, types(refs, args)?)
            }
        },
        // XIR has two variant tests: `test_variant` takes the enum by value and
        // the reader borrows it, while `test_variant_ref` takes a reference as
        // given. Choosing by the operand's type matters — emitting the value
        // form for an operand that is already a reference makes the reader
        // borrow a second time, which costs an instruction and a signature.
        Operation::TestVariant(mid, sid, variant, args) => {
            local_struct(refs, *mid, "test_variant")?;
            let index = variant_index(env, mid.qualified(*sid), *variant)?;
            let operand = srcs.first().context("test_variant has no operand")?;
            let by_reference = matches!(data.local_types.get(*operand), Some(Type::Reference(..)));
            match (by_reference, args.is_empty()) {
                (true, true) => Oper::TestVariantRef(index),
                (true, false) => Oper::TestVariantRefInst(index, types(refs, args)?),
                (false, true) => Oper::TestVariant(index),
                (false, false) => Oper::TestVariantInst(index, types(refs, args)?),
            }
        },
        Operation::BorrowVariantField(mid, sid, variants, args, offset) => {
            local_struct(refs, *mid, "borrow_variant_field")?;
            let indices = variants
                .iter()
                .map(|variant| variant_index(env, mid.qualified(*sid), *variant))
                .collect::<Result<Vec<_>>>()?;
            if args.is_empty() {
                Oper::BorrowVariantField(indices, *offset)
            } else {
                Oper::BorrowVariantFieldInst(indices, *offset, types(refs, args)?)
            }
        },
        Operation::BorrowLoc => Oper::BorrowLoc,
        Operation::ReadRef => Oper::ReadRef,
        Operation::WriteRef => Oper::WriteRef,
        Operation::FreezeRef(_) => Oper::FreezeRef,
        Operation::Vector => Oper::VecPack,
        Operation::Not => Oper::Not,
        Operation::And => Oper::And,
        Operation::Or => Oper::Or,
        Operation::Eq => Oper::Eq,
        Operation::Neq => Oper::Neq,
        Operation::Lt => Oper::Lt,
        Operation::Le => Oper::Le,
        Operation::Gt => Oper::Gt,
        Operation::Ge => Oper::Ge,
        Operation::Negate => Oper::Negate(width()?),
        Operation::Add => Oper::Add(width()?),
        Operation::Sub => Oper::Sub(width()?),
        Operation::Mul => Oper::Mul(width()?),
        Operation::Div => Oper::Div(width()?),
        Operation::Mod => Oper::Mod(width()?),
        Operation::BitAnd => Oper::BitAnd(width()?),
        Operation::BitOr => Oper::BitOr(width()?),
        Operation::Xor => Oper::BitXor(width()?),
        Operation::Shl => Oper::Shl(width()?),
        Operation::Shr => Oper::Shr(width()?),
        Operation::CastU8 => Oper::Cast(IntType::U8),
        Operation::CastU16 => Oper::Cast(IntType::U16),
        Operation::CastU32 => Oper::Cast(IntType::U32),
        Operation::CastU64 => Oper::Cast(IntType::U64),
        Operation::CastU128 => Oper::Cast(IntType::U128),
        Operation::CastU256 => Oper::Cast(IntType::U256),
        Operation::CastI8 => Oper::Cast(IntType::I8),
        Operation::CastI16 => Oper::Cast(IntType::I16),
        Operation::CastI32 => Oper::Cast(IntType::I32),
        Operation::CastI64 => Oper::Cast(IntType::I64),
        Operation::CastI128 => Oper::Cast(IntType::I128),
        Operation::CastI256 => Oper::Cast(IntType::I256),
        // Closures arrived in language version 2.2 and this schema models no
        // closure or invoke operation. The rest are inserted by the prover
        // pipeline and cannot appear in a baseline target.
        other => bail!("`{other:?}` has no XIR operation form"),
    })
}
