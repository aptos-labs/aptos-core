// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! XIR support for compiler-v2.
//!
//! This reader registers a versioned deployable XIR module in the Move model
//! and translates its function bodies directly to baseline stackless bytecode.
//! Source frontends are responsible for producing the JSON; the ordinary
//! compiler-v2 stackless checks, optimizations, file-format generator, and
//! verifier own all later compilation stages.

use anyhow::{bail, ensure, Context, Result};
use codespan::Span;
use move_binary_format::file_format::Visibility as MoveVisibility;
use move_command_line_common::files::FileHash;
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    identifier::Identifier,
};
use move_model::{
    ast::{Address, ModuleName},
    model::{
        FieldData, FunId, FunctionKind, GlobalEnv, Loc, ModuleId, Parameter, QualifiedId, StructId,
        TypeParameter, TypeParameterKind,
    },
    ty::{PrimitiveType, ReferenceKind, Type},
    xir_loader::{
        XirFunctionData as ModelXirFunctionData, XirModuleData as ModelXirModuleData,
        XirStructData as ModelXirStructData, XirVariantData as ModelXirVariantData,
    },
};
use move_model_exchange::{
    Block, Instr, Oper, Term, Type as Ty, TypeParameter as TypeParameterDecl, Value as Constant,
    XirDialect, XirFunction as FunctionDecl, XirModule, XirStruct as StructDecl, XirVisibility,
};
use move_stackless_bytecode::{
    function_target::FunctionData as TargetFunctionData,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{
        AssignKind, AttrId, Bytecode, Constant as StacklessConstant, Label,
        Operation as StacklessOperation,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    rc::Rc,
};

pub struct XirSource {
    path: PathBuf,
    text: String,
    module: XirModule,
}

pub fn parse_source(path: PathBuf, text: String, json: &str) -> Result<XirSource> {
    let module: XirModule = serde_json::from_str(json)
        .with_context(|| format!("invalid XIR from `{}`", path.display()))?;
    validate(&module)?;
    Ok(XirSource { path, text, module })
}

fn validate(module: &XirModule) -> Result<()> {
    module.check_version().map_err(anyhow::Error::msg)?;
    ensure!(
        module.module.dialect == XirDialect::Stackless,
        "only stackless XIR is deployable"
    );
    AccountAddress::from_hex_literal(&module.module.address)
        .with_context(|| format!("invalid module address `{}`", module.module.address))?;
    valid_identifier("module", &module.module.name)?;
    let mut struct_names = BTreeSet::new();
    for decl in &module.structs {
        valid_identifier("struct", &decl.name)?;
        ensure!(
            struct_names.insert(&decl.name),
            "duplicate struct `{}`",
            decl.name
        );
        let mut field_names = BTreeSet::new();
        for field in &decl.fields {
            valid_identifier("field", &field.name)?;
            ensure!(
                field_names.insert(&field.name),
                "duplicate field `{}` in struct `{}`",
                field.name,
                decl.name
            );
        }
    }
    let mut function_names = BTreeSet::new();
    for decl in &module.functions {
        valid_identifier("function", &decl.name)?;
        ensure!(
            function_names.insert(&decl.name),
            "duplicate function `{}`",
            decl.name
        );
        ensure!(
            decl.params <= decl.locals.len(),
            "function `{}` has more parameters than locals",
            decl.name
        );
        ensure!(
            !decl.blocks.is_empty(),
            "function `{}` has no blocks",
            decl.name
        );
        ensure!(
            decl.entry < decl.blocks.len(),
            "entry block is out of range in `{}`",
            decl.name
        );
        ensure!(
            decl.blocks.len() <= u16::MAX as usize,
            "too many blocks in `{}`",
            decl.name
        );
        ensure!(
            decl.locals.len() <= u16::MAX as usize,
            "too many locals in `{}`",
            decl.name
        );
        ensure!(
            decl.loops.is_empty(),
            "loop metadata is not supported by the XIR reader yet in `{}`",
            decl.name
        );
        let _ = &decl.spec;
    }
    Ok(())
}

fn valid_identifier(kind: &str, name: &str) -> Result<()> {
    Identifier::new(name)
        .map(|_| ())
        .with_context(|| format!("invalid {kind} identifier `{name}`"))
}

pub fn import_sources(
    env: &mut GlobalEnv,
    sources: &[XirSource],
    targets: &mut FunctionTargetsHolder,
) -> Result<()> {
    for source in sources {
        import_source(env, source, targets)
            .with_context(|| format!("loading XIR from `{}`", source.path.display()))?;
    }
    env.set_function_size_estimates(targets.compute_function_size_estimates());
    Ok(())
}

fn import_source(
    env: &mut GlobalEnv,
    source: &XirSource,
    targets: &mut FunctionTargetsHolder,
) -> Result<()> {
    let xir = &source.module;
    let address = AccountAddress::from_hex_literal(&xir.module.address)?;
    let module_symbol = env.symbol_pool().make(&xir.module.name);
    let module_name = ModuleName::new(Address::Numerical(address), module_symbol);
    ensure!(
        env.find_module(&module_name).is_none(),
        "duplicate module `{}::{}`",
        xir.module.address,
        xir.module.name
    );
    let file_id = env.add_source(
        FileHash::new(&source.text),
        Rc::new(BTreeMap::new()),
        &source.path.to_string_lossy(),
        &source.text,
        true,
        true,
    );
    let end = u32::try_from(source.text.len()).unwrap_or(u32::MAX);
    let loc = Loc::new(file_id, Span::new(0, end));
    let module_id = ModuleId::new(env.get_module_count());
    let struct_ids = xir
        .structs
        .iter()
        .map(|decl| StructId::new(env.symbol_pool().make(&decl.name)))
        .collect::<Vec<_>>();
    let function_ids = xir
        .functions
        .iter()
        .map(|decl| FunId::new(env.symbol_pool().make(&decl.name)))
        .collect::<Vec<_>>();

    let mut structs = vec![];
    for (decl, struct_id) in xir.structs.iter().zip(&struct_ids) {
        let mut fields = vec![];
        for (offset, field) in decl.fields.iter().enumerate() {
            let field_symbol = env.symbol_pool().make(&field.name);
            fields.push(FieldData {
                name: field_symbol,
                loc: loc.clone(),
                offset,
                variant: None,
                ty: model_type(&field.ty, module_id, &struct_ids)?,
                is_ghost: false,
                init: None,
            });
        }
        let variants = decl.variants.as_ref().map(|variants| {
            variants
                .iter()
                .map(|variant| ModelXirVariantData {
                    name: env.symbol_pool().make(&variant.name),
                    loc: loc.clone(),
                })
                .collect::<Vec<_>>()
        });
        if let Some(variants) = &decl.variants {
            for variant in variants {
                let variant_symbol = env.symbol_pool().make(&variant.name);
                for (offset, field) in variant.fields.iter().enumerate() {
                    fields.push(FieldData {
                        name: env.symbol_pool().make(&field.name),
                        loc: loc.clone(),
                        offset,
                        variant: Some(variant_symbol),
                        ty: model_type(&field.ty, module_id, &struct_ids)?,
                        is_ghost: false,
                        init: None,
                    });
                }
            }
        }
        structs.push(ModelXirStructData {
            name: struct_id.symbol(),
            loc: loc.clone(),
            abilities: ability_set(decl)?,
            type_parameters: model_type_parameters(env, &loc, &decl.type_parameters)?,
            fields,
            variants,
            visibility: MoveVisibility::Private,
        });
    }

    let mut functions = vec![];
    for (decl, fun_id) in xir.functions.iter().zip(&function_ids) {
        let local_types = decl
            .locals
            .iter()
            .map(|ty| model_type(ty, module_id, &struct_ids))
            .collect::<Result<Vec<_>>>()?;
        let params = local_types
            .iter()
            .take(decl.params)
            .enumerate()
            .map(|(index, ty)| {
                Parameter(
                    env.symbol_pool().make(&format!("p{index}")),
                    ty.clone(),
                    loc.clone(),
                )
            })
            .collect();
        let returns = Type::tuple(
            decl.returns
                .iter()
                .map(|ty| model_type(ty, module_id, &struct_ids))
                .collect::<Result<Vec<_>>>()?,
        );
        let acquired = decl
            .acquires
            .iter()
            .map(|id| struct_at(&struct_ids, *id, &decl.name))
            .collect::<Result<BTreeSet<_>>>()?;
        let called = called_functions(decl, module_id, &function_ids)?;
        functions.push(ModelXirFunctionData {
            name: fun_id.symbol(),
            loc: loc.clone(),
            visibility: move_visibility(&decl.visibility),
            kind: if decl.is_entry {
                FunctionKind::Entry
            } else {
                FunctionKind::Regular
            },
            type_parameters: model_type_parameters(env, &loc, &decl.type_parameters)?,
            params,
            result_type: returns,
            acquired_structs: acquired,
            called_funs: called,
        });
    }

    let added_id = env.load_xir_module(ModelXirModuleData {
        loc,
        name: module_name,
        structs,
        functions,
    })?;
    ensure!(
        added_id == module_id,
        "model assigned an unexpected module id"
    );
    for (decl, fun_id) in xir.functions.iter().zip(&function_ids) {
        let qid = module_id.qualified(*fun_id);
        let data = translate_function(env, xir, module_id, &struct_ids, &function_ids, decl, qid)?;
        targets.insert_target_data(&qid, FunctionVariant::Baseline, data);
    }
    Ok(())
}

fn ability_set(decl: &StructDecl) -> Result<AbilitySet> {
    parse_ability_set(&decl.abilities).with_context(|| format!("on struct `{}`", decl.name))
}

fn parse_ability_set(abilities: &[String]) -> Result<AbilitySet> {
    let mut result = AbilitySet::EMPTY;
    for ability in abilities {
        result = result.add(match ability.as_str() {
            "copy" => Ability::Copy,
            "drop" => Ability::Drop,
            "store" => Ability::Store,
            "key" => Ability::Key,
            _ => bail!("unknown ability `{ability}`"),
        });
    }
    Ok(result)
}

fn model_type_parameters(
    env: &GlobalEnv,
    loc: &Loc,
    params: &[TypeParameterDecl],
) -> Result<Vec<TypeParameter>> {
    params
        .iter()
        .map(|param| {
            let abilities = parse_ability_set(&param.abilities)
                .with_context(|| format!("on type parameter `{}`", param.name))?;
            let kind = if param.phantom {
                TypeParameterKind::new_phantom(abilities)
            } else {
                TypeParameterKind::new(abilities)
            };
            Ok(TypeParameter(
                env.symbol_pool().make(&param.name),
                kind,
                loc.clone(),
            ))
        })
        .collect()
}

fn move_visibility(visibility: &XirVisibility) -> MoveVisibility {
    match visibility {
        XirVisibility::Private => MoveVisibility::Private,
        XirVisibility::Public => MoveVisibility::Public,
        XirVisibility::Friend => MoveVisibility::Friend,
    }
}

fn model_type(ty: &Ty, module_id: ModuleId, structs: &[StructId]) -> Result<Type> {
    Ok(match ty {
        Ty::Bool => Type::Primitive(PrimitiveType::Bool),
        Ty::U64 => Type::Primitive(PrimitiveType::U64),
        Ty::Address => Type::Primitive(PrimitiveType::Address),
        Ty::Signer => Type::Primitive(PrimitiveType::Signer),
        Ty::TypeParameter(index) => {
            ensure!(
                *index <= u16::MAX as usize,
                "type parameter index is too large"
            );
            Type::TypeParameter(*index as u16)
        },
        Ty::Struct(id) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("struct id {id} is out of range"))?,
            vec![],
        ),
        Ty::StructInst(id, args) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("struct id {id} is out of range"))?,
            args.iter()
                .map(|arg| model_type(arg, module_id, structs))
                .collect::<Result<Vec<_>>>()?,
        ),
        Ty::Enum(id) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("enum id {id} is out of range"))?,
            vec![],
        ),
        Ty::EnumInst(id, args) => Type::Struct(
            module_id,
            *structs
                .get(*id)
                .with_context(|| format!("enum id {id} is out of range"))?,
            args.iter()
                .map(|arg| model_type(arg, module_id, structs))
                .collect::<Result<Vec<_>>>()?,
        ),
        Ty::Vector(element) => Type::Vector(Box::new(model_type(element, module_id, structs)?)),
        Ty::Ref(referent) => Type::Reference(
            ReferenceKind::Immutable,
            Box::new(model_type(referent, module_id, structs)?),
        ),
        Ty::MutRef(referent) => Type::Reference(
            ReferenceKind::Mutable,
            Box::new(model_type(referent, module_id, structs)?),
        ),
    })
}

fn called_functions(
    decl: &FunctionDecl,
    module_id: ModuleId,
    functions: &[FunId],
) -> Result<BTreeSet<QualifiedId<FunId>>> {
    let mut called = BTreeSet::new();
    for block in &decl.blocks {
        for instr in &block.instrs {
            if let Instr::Call(_, Oper::Function(id) | Oper::FunctionInst(id, _), _) = instr {
                let fun_id = functions.get(*id).with_context(|| {
                    format!("function id {id} is out of range in `{}`", decl.name)
                })?;
                called.insert(module_id.qualified(*fun_id));
            }
        }
    }
    Ok(called)
}

fn struct_at(structs: &[StructId], id: usize, function: &str) -> Result<StructId> {
    structs
        .get(id)
        .copied()
        .with_context(|| format!("struct id {id} is out of range in `{function}`"))
}

fn translate_function(
    env: &GlobalEnv,
    xir: &XirModule,
    module_id: ModuleId,
    struct_ids: &[StructId],
    function_ids: &[FunId],
    decl: &FunctionDecl,
    qid: QualifiedId<FunId>,
) -> Result<TargetFunctionData> {
    let func_env = env.get_function(qid);
    let mut translator = FunctionTranslator {
        env,
        xir,
        module_id,
        struct_ids,
        function_ids,
        decl,
        loc: func_env.get_loc(),
        code: vec![],
        locations: BTreeMap::new(),
        local_types: decl
            .locals
            .iter()
            .map(|ty| model_type(ty, module_id, struct_ids))
            .collect::<Result<Vec<_>>>()?,
        next_attr: 0,
    };
    translator.emit(|attr| Bytecode::Jump(attr, Label::new(decl.entry)))?;
    for (block_id, block) in decl.blocks.iter().enumerate() {
        translator.emit(|attr| Bytecode::Label(attr, Label::new(block_id)))?;
        for (instruction_id, instruction) in block.instrs.iter().enumerate() {
            translator
                .translate_instruction(instruction)
                .with_context(|| {
                    format!(
                        "function `{}`, block {block_id}, instruction {instruction_id}",
                        decl.name
                    )
                })?;
        }
        translator
            .translate_term(&block.term)
            .with_context(|| format!("function `{}`, block {block_id}", decl.name))?;
    }
    let result_type = Type::tuple(
        decl.returns
            .iter()
            .map(|ty| model_type(ty, module_id, struct_ids))
            .collect::<Result<Vec<_>>>()?,
    );
    let acquires = decl
        .acquires
        .iter()
        .map(|id| struct_at(struct_ids, *id, &decl.name))
        .collect::<Result<Vec<_>>>()?;
    let local_names = (0..translator.local_types.len())
        .map(|index| (index, env.symbol_pool().make(&format!("l{index}"))))
        .collect::<BTreeMap<_, _>>();
    let name_to_index = local_names
        .iter()
        .map(|(index, name)| (*name, *index))
        .collect();
    Ok(TargetFunctionData::new(
        &func_env,
        translator.code,
        translator.local_types,
        result_type,
        translator.locations,
        name_to_index,
        acquires,
        BTreeMap::new(),
        BTreeSet::new(),
        local_names,
    ))
}

struct FunctionTranslator<'a> {
    env: &'a GlobalEnv,
    xir: &'a XirModule,
    module_id: ModuleId,
    struct_ids: &'a [StructId],
    function_ids: &'a [FunId],
    decl: &'a FunctionDecl,
    loc: Loc,
    code: Vec<Bytecode>,
    locations: BTreeMap<AttrId, Loc>,
    local_types: Vec<Type>,
    next_attr: usize,
}

impl FunctionTranslator<'_> {
    fn emit(&mut self, make: impl FnOnce(AttrId) -> Bytecode) -> Result<()> {
        ensure!(self.next_attr <= u16::MAX as usize, "function is too large");
        let attr = AttrId::new(self.next_attr);
        self.next_attr += 1;
        self.locations.insert(attr, self.loc.clone());
        self.code.push(make(attr));
        Ok(())
    }

    fn local(&self, id: usize) -> Result<&Type> {
        self.local_types
            .get(id)
            .with_context(|| format!("local l{id} is out of range in `{}`", self.decl.name))
    }

    fn block(&self, id: usize) -> Result<&Block> {
        self.decl
            .blocks
            .get(id)
            .with_context(|| format!("block {id} is out of range in `{}`", self.decl.name))
    }

    fn fresh_local(&mut self, ty: Type) -> usize {
        let id = self.local_types.len();
        self.local_types.push(ty);
        id
    }

    fn translate_instruction(&mut self, instruction: &Instr) -> Result<()> {
        match instruction {
            Instr::Load(dst, constant) => {
                self.local(*dst)?;
                let constant = stackless_constant(constant)?;
                self.emit(|attr| Bytecode::Load(attr, *dst, constant))
            },
            Instr::Assign(dst, src) => {
                self.local(*dst)?;
                self.local(*src)?;
                self.emit(|attr| Bytecode::Assign(attr, *dst, *src, AssignKind::Inferred))
            },
            Instr::Call(dsts, oper, srcs) => self.translate_call(dsts, oper, srcs),
            Instr::Nop => self.emit(Bytecode::Nop),
        }
    }

    fn translate_call(&mut self, dsts: &[usize], oper: &Oper, srcs: &[usize]) -> Result<()> {
        for id in dsts.iter().chain(srcs) {
            self.local(*id)?;
        }
        match oper {
            Oper::VecLen => {
                arity(dsts, srcs, 1, 1, oper)?;
                let element = self.vector_element_type(self.local(srcs[0])?)?;
                let reference = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(Type::Vector(Box::new(element.clone()))),
                ));
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![reference],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                let operation = self.vector_function("length", element)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, vec![reference], None)
                })
            },
            Oper::VecGet => {
                arity(dsts, srcs, 1, 2, oper)?;
                let element = self.vector_element_type(self.local(srcs[0])?)?;
                let vector_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(Type::Vector(Box::new(element.clone()))),
                ));
                let element_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(element.clone()),
                ));
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![vector_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                let operation = self.vector_function("borrow", element)?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![element_ref],
                        operation,
                        vec![vector_ref, srcs[1]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        dsts.to_vec(),
                        StacklessOperation::ReadRef,
                        vec![element_ref],
                        None,
                    )
                })
            },
            Oper::VecSet | Oper::VecPush | Oper::VecPop => {
                self.translate_functional_vector_update(dsts, oper, srcs)
            },
            Oper::BorrowVecElem => {
                arity(dsts, srcs, 1, 2, oper)?;
                let element = self.vector_element_type(self.local(srcs[0])?)?;
                let mutable = matches!(
                    self.local(dsts[0])?,
                    Type::Reference(ReferenceKind::Mutable, _)
                );
                let operation =
                    self.vector_function(if mutable { "borrow_mut" } else { "borrow" }, element)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, srcs.to_vec(), None)
                })
            },
            Oper::TestVariant(variant) | Oper::TestVariantInst(variant, _) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let enum_type = self.local(srcs[0])?.clone();
                let sid = self.struct_from_type(&enum_type)?;
                let enum_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(enum_type),
                ));
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![enum_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                let args = match oper {
                    Oper::TestVariantInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let operation = StacklessOperation::TestVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    args,
                );
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, vec![enum_ref], None)
                })
            },
            Oper::GetField(field) | Oper::GetFieldInst(field, _) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let struct_type = self.local(srcs[0])?.clone();
                let field_type = self.local(dsts[0])?.clone();
                let sid = self.struct_from_type(&struct_type)?;
                self.field(sid, *field)?;
                let args = match oper {
                    Oper::GetFieldInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let struct_env = self.env.get_struct(self.module_id.qualified(sid));
                if !struct_env.get_abilities().has_ability(Ability::Drop) {
                    ensure!(
                        struct_env.get_field_count() == 1 && *field == 0,
                        "cannot project field {field} from non-droppable {struct_type:?}; destructure all fields instead"
                    );
                    let module_id = self.module_id;
                    return self.emit(|attr| {
                        Bytecode::Call(
                            attr,
                            dsts.to_vec(),
                            StacklessOperation::Unpack(module_id, sid, args),
                            srcs.to_vec(),
                            None,
                        )
                    });
                }
                let struct_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(struct_type),
                ));
                let field_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(field_type),
                ));
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![struct_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![field_ref],
                        StacklessOperation::BorrowField(module_id, sid, args, *field),
                        vec![struct_ref],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        dsts.to_vec(),
                        StacklessOperation::ReadRef,
                        vec![field_ref],
                        None,
                    )
                })
            },
            Oper::GetGlobal(id) | Oper::GetGlobalInst(id, _) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = struct_at(self.struct_ids, *id, &self.decl.name)?;
                let args = match oper {
                    Oper::GetGlobalInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let reference = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(Type::Struct(self.module_id, sid, args.clone())),
                ));
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![reference],
                        StacklessOperation::BorrowGlobal(module_id, sid, args),
                        srcs.to_vec(),
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        dsts.to_vec(),
                        StacklessOperation::ReadRef,
                        vec![reference],
                        None,
                    )
                })
            },
            Oper::MoveTo(id) | Oper::MoveToInst(id, _) => {
                arity(dsts, srcs, 0, 2, oper)?;
                let sid = struct_at(self.struct_ids, *id, &self.decl.name)?;
                let args = match oper {
                    Oper::MoveToInst(_, args) => self.type_args(args)?,
                    _ => vec![],
                };
                let signer_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(Type::Primitive(PrimitiveType::Signer)),
                ));
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![signer_ref],
                        StacklessOperation::BorrowLoc,
                        vec![srcs[0]],
                        None,
                    )
                })?;
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![],
                        StacklessOperation::MoveTo(module_id, sid, args),
                        vec![signer_ref, srcs[1]],
                        None,
                    )
                })
            },
            Oper::WriteGlobal(id) => {
                arity(dsts, srcs, 0, 2, oper)?;
                let sid = struct_at(self.struct_ids, *id, &self.decl.name)?;
                let reference = self.fresh_local(Type::Reference(
                    ReferenceKind::Mutable,
                    Box::new(Type::Struct(self.module_id, sid, vec![])),
                ));
                let module_id = self.module_id;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![reference],
                        StacklessOperation::BorrowGlobal(module_id, sid, vec![]),
                        vec![srcs[0]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![],
                        StacklessOperation::WriteRef,
                        vec![reference, srcs[1]],
                        None,
                    )
                })
            },
            _ => {
                let operation = self.operation(dsts, oper, srcs)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, dsts.to_vec(), operation, srcs.to_vec(), None)
                })
            },
        }
    }

    fn operation(&self, dsts: &[usize], oper: &Oper, srcs: &[usize]) -> Result<StacklessOperation> {
        Ok(match oper {
            Oper::Add => StacklessOperation::Add,
            Oper::Sub => StacklessOperation::Sub,
            Oper::Mul => StacklessOperation::Mul,
            Oper::Div => StacklessOperation::Div,
            Oper::Mod => StacklessOperation::Mod,
            Oper::Lt => StacklessOperation::Lt,
            Oper::Le => StacklessOperation::Le,
            Oper::Eq => StacklessOperation::Eq,
            Oper::And => StacklessOperation::And,
            Oper::Or => StacklessOperation::Or,
            Oper::Not => StacklessOperation::Not,
            Oper::VecPack => StacklessOperation::Vector,
            Oper::Pack => {
                ensure!(dsts.len() == 1, "pack expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::Pack(self.module_id, sid, vec![])
            },
            Oper::PackInst(args) => {
                ensure!(dsts.len() == 1, "pack expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::Pack(self.module_id, sid, self.type_args(args)?)
            },
            Oper::Unpack => {
                ensure!(srcs.len() == 1, "unpack expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::Unpack(self.module_id, sid, vec![])
            },
            Oper::UnpackInst(args) => {
                ensure!(srcs.len() == 1, "unpack expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::Unpack(self.module_id, sid, self.type_args(args)?)
            },
            Oper::PackVariant(variant) => {
                ensure!(dsts.len() == 1, "pack_variant expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::PackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    vec![],
                )
            },
            Oper::PackVariantInst(variant, args) => {
                ensure!(dsts.len() == 1, "pack_variant expects one destination");
                let sid = self.struct_from_type(self.local(dsts[0])?)?;
                StacklessOperation::PackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    self.type_args(args)?,
                )
            },
            Oper::UnpackVariant(variant) => {
                ensure!(srcs.len() == 1, "unpack_variant expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::UnpackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    vec![],
                )
            },
            Oper::UnpackVariantInst(variant, args) => {
                ensure!(srcs.len() == 1, "unpack_variant expects one source");
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                StacklessOperation::UnpackVariant(
                    self.module_id,
                    sid,
                    self.variant(sid, *variant)?,
                    self.type_args(args)?,
                )
            },
            Oper::GetField(field) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::GetField(self.module_id, sid, vec![], *field)
            },
            Oper::GetFieldInst(field, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::GetField(self.module_id, sid, self.type_args(args)?, *field)
            },
            Oper::MoveTo(id) => StacklessOperation::MoveTo(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                vec![],
            ),
            Oper::MoveToInst(id, args) => StacklessOperation::MoveTo(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                self.type_args(args)?,
            ),
            Oper::MoveFrom(id) => StacklessOperation::MoveFrom(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                vec![],
            ),
            Oper::MoveFromInst(id, args) => StacklessOperation::MoveFrom(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                self.type_args(args)?,
            ),
            Oper::Exists(id) => StacklessOperation::Exists(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                vec![],
            ),
            Oper::ExistsInst(id, args) => StacklessOperation::Exists(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                self.type_args(args)?,
            ),
            Oper::Function(id) => StacklessOperation::Function(
                self.module_id,
                *self
                    .function_ids
                    .get(*id)
                    .with_context(|| format!("function id {id} is out of range"))?,
                vec![],
            ),
            Oper::FunctionInst(id, args) => StacklessOperation::Function(
                self.module_id,
                *self
                    .function_ids
                    .get(*id)
                    .with_context(|| format!("function id {id} is out of range"))?,
                self.type_args(args)?,
            ),
            Oper::BorrowLoc => StacklessOperation::BorrowLoc,
            Oper::BorrowField(field) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::BorrowField(self.module_id, sid, vec![], *field)
            },
            Oper::BorrowFieldInst(field, args) => {
                arity(dsts, srcs, 1, 1, oper)?;
                let sid = self.struct_from_type(self.local(srcs[0])?)?;
                self.field(sid, *field)?;
                StacklessOperation::BorrowField(self.module_id, sid, self.type_args(args)?, *field)
            },
            Oper::BorrowGlobal(id) => StacklessOperation::BorrowGlobal(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                vec![],
            ),
            Oper::BorrowGlobalInst(id, args) => StacklessOperation::BorrowGlobal(
                self.module_id,
                struct_at(self.struct_ids, *id, &self.decl.name)?,
                self.type_args(args)?,
            ),
            Oper::ReadRef => StacklessOperation::ReadRef,
            Oper::WriteRef => StacklessOperation::WriteRef,
            Oper::FreezeRef => StacklessOperation::FreezeRef(true),
            unsupported => bail!("unsupported XIR operation {unsupported:?}"),
        })
    }

    fn type_args(&self, args: &[Ty]) -> Result<Vec<Type>> {
        args.iter()
            .map(|arg| model_type(arg, self.module_id, self.struct_ids))
            .collect()
    }

    fn vector_element_type(&self, ty: &Type) -> Result<Type> {
        match ty {
            Type::Vector(element) => Ok(element.as_ref().clone()),
            Type::Reference(_, referent) => self.vector_element_type(referent),
            other => bail!("expected a vector type, got {other:?}"),
        }
    }

    fn vector_function(&self, name: &str, element: Type) -> Result<StacklessOperation> {
        let module = self
            .env
            .get_modules()
            .find(|module| module.is_std_vector())
            .context("the Move model has no standard vector module")?;
        let function = module
            .find_function(self.env.symbol_pool().make(name))
            .with_context(|| format!("the standard vector module has no `{name}` function"))?;
        Ok(StacklessOperation::Function(
            module.get_id(),
            function.get_id(),
            vec![element],
        ))
    }

    fn translate_functional_vector_update(
        &mut self,
        dsts: &[usize],
        oper: &Oper,
        srcs: &[usize],
    ) -> Result<()> {
        let (vector_dst, value_dst) = match oper {
            Oper::VecSet => {
                arity(dsts, srcs, 1, 3, oper)?;
                (dsts[0], None)
            },
            Oper::VecPush => {
                arity(dsts, srcs, 1, 2, oper)?;
                (dsts[0], None)
            },
            Oper::VecPop => {
                arity(dsts, srcs, 2, 1, oper)?;
                (dsts[0], Some(dsts[1]))
            },
            _ => unreachable!(),
        };
        let element = self.vector_element_type(self.local(srcs[0])?)?;
        self.emit(|attr| Bytecode::Assign(attr, vector_dst, srcs[0], AssignKind::Inferred))?;
        let vector_ref = self.fresh_local(Type::Reference(
            ReferenceKind::Mutable,
            Box::new(Type::Vector(Box::new(element.clone()))),
        ));
        self.emit(|attr| {
            Bytecode::Call(
                attr,
                vec![vector_ref],
                StacklessOperation::BorrowLoc,
                vec![vector_dst],
                None,
            )
        })?;
        match oper {
            Oper::VecSet => {
                let element_ref = self.fresh_local(Type::Reference(
                    ReferenceKind::Mutable,
                    Box::new(element.clone()),
                ));
                let borrow = self.vector_function("borrow_mut", element)?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![element_ref],
                        borrow,
                        vec![vector_ref, srcs[1]],
                        None,
                    )
                })?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![],
                        StacklessOperation::WriteRef,
                        vec![element_ref, srcs[2]],
                        None,
                    )
                })
            },
            Oper::VecPush => {
                let push = self.vector_function("push_back", element)?;
                self.emit(|attr| {
                    Bytecode::Call(attr, vec![], push, vec![vector_ref, srcs[1]], None)
                })
            },
            Oper::VecPop => {
                let pop = self.vector_function("pop_back", element)?;
                self.emit(|attr| {
                    Bytecode::Call(
                        attr,
                        vec![value_dst.expect("checked vec_pop destination")],
                        pop,
                        vec![vector_ref],
                        None,
                    )
                })
            },
            _ => unreachable!(),
        }
    }

    fn struct_from_type(&self, ty: &Type) -> Result<StructId> {
        match ty {
            Type::Struct(mid, sid, _) if *mid == self.module_id => Ok(*sid),
            Type::Reference(_, referent) => self.struct_from_type(referent),
            other => bail!("expected a local struct type, got {other:?}"),
        }
    }

    fn field(&self, sid: StructId, field: usize) -> Result<()> {
        let index = self
            .struct_ids
            .iter()
            .position(|candidate| *candidate == sid)
            .context("unknown local struct")?;
        self.xir.structs[index]
            .fields
            .get(field)
            .with_context(|| format!("field id {field} is out of range"))?;
        Ok(())
    }

    fn variant(&self, sid: StructId, variant: usize) -> Result<move_model::symbol::Symbol> {
        let index = self
            .struct_ids
            .iter()
            .position(|candidate| *candidate == sid)
            .context("unknown local enum")?;
        let variants = self.xir.structs[index]
            .variants
            .as_ref()
            .context("variant operation used with a non-enum type")?;
        let variant = variants
            .get(variant)
            .with_context(|| format!("variant id {variant} is out of range"))?;
        Ok(self.env.symbol_pool().make(&variant.name))
    }

    fn translate_term(&mut self, term: &Term) -> Result<()> {
        match term {
            Term::Jump(target) => {
                self.block(*target)?;
                self.emit(|attr| Bytecode::Jump(attr, Label::new(*target)))
            },
            Term::Branch(cond, then_block, else_block) => {
                self.local(*cond)?;
                self.block(*then_block)?;
                self.block(*else_block)?;
                self.emit(|attr| {
                    Bytecode::Branch(
                        attr,
                        Label::new(*then_block),
                        Label::new(*else_block),
                        *cond,
                    )
                })
            },
            Term::Ret(values) => {
                ensure!(
                    values.len() == self.decl.returns.len(),
                    "return arity mismatch"
                );
                for value in values {
                    self.local(*value)?;
                }
                self.emit(|attr| Bytecode::Ret(attr, values.clone()))
            },
            Term::Abort(code) => {
                self.local(*code)?;
                self.emit(|attr| Bytecode::Abort(attr, *code, None))
            },
        }
    }
}

fn stackless_constant(constant: &Constant) -> Result<StacklessConstant> {
    Ok(match constant {
        Constant::U64(value) => StacklessConstant::U64(*value),
        Constant::Bool(value) => StacklessConstant::Bool(*value),
        Constant::Address(value) => StacklessConstant::Address(Address::Numerical(
            AccountAddress::from_hex_literal(value)
                .with_context(|| format!("invalid address constant `{value}`"))?,
        )),
        Constant::Vector(_) => bail!("vector values are not valid XIR load constants"),
    })
}

fn arity(
    dsts: &[usize],
    srcs: &[usize],
    expected_dsts: usize,
    expected_srcs: usize,
    oper: &Oper,
) -> Result<()> {
    ensure!(
        dsts.len() == expected_dsts && srcs.len() == expected_srcs,
        "{oper:?} expects {expected_dsts} destinations and {expected_srcs} sources"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use std::fs;

    fn account_golden() -> String {
        fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xir/account.xir.json"),
        )
        .unwrap()
    }

    #[test]
    fn account_golden_decodes() {
        let source = parse_source(
            PathBuf::from("account.xir.json"),
            String::new(),
            &account_golden(),
        )
        .unwrap();
        assert_eq!(source.module.module.name, "AccountTest");
    }

    #[test]
    fn account_golden_runs_production_stackless_and_file_format_pipeline() {
        let source = parse_source(
            PathBuf::from("account.xir.json"),
            String::new(),
            &account_golden(),
        )
        .unwrap();
        let mut env = GlobalEnv::new();
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets).unwrap();
        let options = Options::default();
        env.set_extension(options.clone());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_check_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        crate::run_stackless_bytecode_pipeline(
            &env,
            crate::stackless_bytecode_optimization_pipeline(&options),
            &mut targets,
        );
        assert!(!env.has_errors());
        let units = crate::run_file_format_gen(&mut env, &targets);
        assert!(!env.has_errors());
        assert_eq!(units.len(), 1);
        let legacy_move_compiler::compiled_unit::CompiledUnit::Module(module) = &units[0] else {
            panic!("expected module")
        };
        move_bytecode_verifier::verify_module(&module.module).unwrap();
    }
}
