// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! XAST producer: dumps a module of the move-model `GlobalEnv` — its
//! declarations, typed expressions, and specifications — as a
//! [`move_model_exchange::ast::XastModule`].
//!
//! The producer is a faithful, policy-free walk of the model as compiler v2
//! leaves it after env checking and rewriting (inlining, spec rewriting,
//! match transforms); the consumer (the Lean transpiler) owns all policy.  The
//! only rejections are function-value construction (`Lambda`, closures).
//! Function-typed parameters, their `Invoke` nodes, and behavior predicates
//! are retained for specification helpers.
//! Everything the model does not keep but the consumer needs —
//! the named address a module was declared under and the ordinary comments
//! of its sources — is recovered from the source text.

use anyhow::{anyhow, bail, Result};
use move_binary_format::file_format::Visibility;
use move_core_types::ability::AbilitySet;
use move_model::{
    ast::{
        AbortKind, Address, Attribute, AttributeValue, BehaviorKind, Condition, ConditionKind,
        ExpData, FriendDecl, GlobalInvariant, MemoryRange, Operation, Pattern, PropertyBag,
        PropertyValue, QuantKind, Spec, SpecFunDecl, SpecVarDecl, TraceKind, Value,
    },
    model::{
        FieldEnv, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, NamedConstantEnv, NodeId,
        Parameter, StructEnv, SurfaceSyntax, TypeParameter,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_model_exchange::ast as xast;
use std::{cell::RefCell, collections::BTreeMap};

/// Dumps the module `module_id` of `env` as an XAST document.
pub fn dump_ast_module(env: &GlobalEnv, module_id: ModuleId) -> Result<xast::XastModule> {
    let module_env = env.get_module(module_id);
    let constants: BTreeMap<String, Value> = module_env
        .get_named_constants()
        .map(|c| {
            (
                c.get_name().display(env.symbol_pool()).to_string(),
                c.get_value(),
            )
        })
        .collect();
    let ctx = Ctx {
        env,
        constants,
        files: RefCell::new(vec![]),
        model_modules: RefCell::new(BTreeMap::new()),
        locs: RefCell::new(Interner::default()),
        types: RefCell::new(Interner::default()),
        modules: RefCell::new(Interner::default()),
        names: RefCell::new(Interner::default()),
        scopes: RefCell::new(vec![]),
        params: RefCell::new(vec![]),
    };
    let file_source = |loc: &Loc| env.get_file_source(loc.file_id());
    let pool = env.symbol_pool();

    // Module identity.
    let module_table_id = ctx.module_ref(&module_env);
    let module_ref = ctx.modules.borrow().items[module_table_id].clone();
    let loc = ctx.loc(&module_env.get_loc());
    let named_addresses = env
        .get_address_alias_map()
        .iter()
        .map(|(name, addr)| xast::NamedAddress {
            name: name.display(pool).to_string(),
            address: addr.to_hex_literal(),
        })
        .collect();
    let friends = module_env
        .get_friend_decls()
        .iter()
        .map(|decl| ctx.friend_ref(decl))
        .collect::<Result<Vec<_>>>()?;
    let pragmas = ctx.pragmas(&module_env.get_spec().properties)?;

    // Declarations.
    let mut constants = vec![];
    for constant in module_env.get_named_constants() {
        if constant.is_test_only() {
            continue;
        }
        constants.push(ctx.constant(&constant)?);
    }
    let mut structs = vec![];
    for struct_env in module_env.get_structs() {
        if struct_env.is_test_only() || struct_env.is_ghost_memory() {
            continue;
        }
        structs.push(ctx.struct_decl(&struct_env)?);
    }
    // Unsupported function-value construction is left out, by name and
    // reason, instead of failing the module.
    let is_function_value_error =
        |e: &anyhow::Error| format!("{:#}", e).contains("function values are out of scope");
    let mut functions = vec![];
    let mut skipped = vec![];
    for fun_env in module_env.get_functions() {
        if fun_env.is_test_only() || fun_env.is_lemma() {
            continue;
        }
        match ctx.function(&fun_env) {
            Ok(function) => functions.push(function),
            Err(e) if is_function_value_error(&e) => skipped.push(xast::Skipped {
                name: fun_env.get_name_str().to_string(),
                reason: format!("{:#}", e),
            }),
            Err(e) => return Err(e),
        }
    }
    let mut spec_funs = vec![];
    for (_, decl) in module_env.get_spec_funs() {
        match ctx.spec_fun(decl) {
            Ok(spec_fun) => spec_funs.push(spec_fun),
            Err(e) if is_function_value_error(&e) => skipped.push(xast::Skipped {
                name: ctx.pool().string(decl.name).to_string(),
                reason: format!("{:#}", e),
            }),
            Err(e) => return Err(e),
        }
    }
    let mut spec_vars = vec![];
    for (_, decl) in module_env.get_spec_vars() {
        spec_vars.push(ctx.spec_var(decl)?);
    }
    let mut invariants = vec![];
    for id in env.get_global_invariants_by_module(module_id) {
        let inv = env
            .get_global_invariant(id)
            .ok_or_else(|| anyhow!("dangling global invariant id"))?;
        invariants.push(ctx.global_invariant(inv)?);
    }

    // The file table is complete now; scan the sources for comments.
    let sources: Vec<String> = ctx
        .files
        .borrow()
        .iter()
        .map(|loc| env.get_file(loc.file_id()).to_string_lossy().to_string())
        .collect();
    let mut comments = vec![];
    for (index, loc) in ctx.files.borrow().iter().enumerate() {
        let text = file_source(loc);
        for (start, end) in scan_comments(text) {
            let line_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            comments.push(xast::Comment {
                loc: ctx.locs.borrow_mut().intern(xast::Loc {
                    file: index,
                    start: start as u32,
                    end: end as u32,
                }),
                text: text[start..end].to_string(),
                own_line: text[line_start..start].trim().is_empty(),
            });
        }
    }

    let locs = ctx.locs.into_inner().items;
    let types = ctx.types.into_inner().items;
    let modules = ctx.modules.into_inner().items;
    let names = ctx.names.into_inner().items;
    Ok(xast::XastModule {
        schema: xast::XAST_SCHEMA.to_string(),
        version: xast::XAST_VERSION,
        address: module_ref.address,
        address_alias: module_ref.address_alias,
        name: module_ref.name,
        doc: module_env.get_doc().to_string(),
        loc,
        named_addresses,
        friends,
        pragmas,
        constants,
        structs,
        functions,
        spec_funs,
        spec_vars,
        invariants,
        skipped,
        comments,
        sources,
        locs,
        types,
        modules,
        names,
    })
}

/// A hash-consing table: each distinct value is stored once and referenced by
/// its index.
struct Interner<T: Ord + Clone> {
    items: Vec<T>,
    ids: BTreeMap<T, usize>,
}

impl<T: Ord + Clone> Default for Interner<T> {
    fn default() -> Self {
        Interner {
            items: vec![],
            ids: BTreeMap::new(),
        }
    }
}

impl<T: Ord + Clone> Interner<T> {
    fn intern(&mut self, value: T) -> usize {
        if let Some(id) = self.ids.get(&value) {
            return *id;
        }
        let id = self.items.len();
        self.items.push(value.clone());
        self.ids.insert(value, id);
        id
    }
}

/// Translation context: the environment plus the interned tables being built.
struct Ctx<'a> {
    env: &'a GlobalEnv,
    /// The named constants of the module being dumped, for recovering
    /// constant names at folded value nodes.
    constants: BTreeMap<String, Value>,
    /// File table, in order of first use: one location per file, standing
    /// for its file id.
    files: RefCell<Vec<Loc>>,
    /// Model module ids to interned module references (the header parse is
    /// done once per module).
    model_modules: RefCell<BTreeMap<ModuleId, xast::ModuleId>>,
    locs: RefCell<Interner<xast::Loc>>,
    types: RefCell<Interner<xast::Type>>,
    modules: RefCell<Interner<xast::ModuleRef>>,
    names: RefCell<Interner<xast::QualifiedName>>,
    /// The declared types of the locals in scope (innermost frame last):
    /// pattern-bound variables of blocks, match arms, and quantifiers, spec
    /// `let`s, and spec function parameters.
    scopes: RefCell<Vec<Vec<(Symbol, Type)>>>,
    /// The declared types of the current function's parameters (`Temporary`
    /// indices).
    params: RefCell<Vec<Type>>,
}

impl<'a> Ctx<'a> {
    /// Runs `f` with `vars` bound in a new innermost scope.
    fn with_scope<T>(&self, vars: Vec<(Symbol, Type)>, f: impl FnOnce() -> Result<T>) -> Result<T> {
        self.scopes.borrow_mut().push(vars);
        let result = f();
        self.scopes.borrow_mut().pop();
        result
    }

    /// Runs `f` with `tys` as the current parameter types.
    fn with_params<T>(&self, tys: Vec<Type>, f: impl FnOnce() -> Result<T>) -> Result<T> {
        let saved = std::mem::replace(&mut *self.params.borrow_mut(), tys);
        let result = f();
        *self.params.borrow_mut() = saved;
        result
    }

    /// The variables a pattern binds, with their declared types.
    fn pattern_vars(&self, pat: &Pattern) -> Vec<(Symbol, Type)> {
        pat.vars()
            .into_iter()
            .map(|(id, sym)| (sym, self.env.get_node_type(id)))
            .collect()
    }

    fn declared_type(&self, name: Symbol) -> Option<Type> {
        self.scopes.borrow().iter().rev().find_map(|frame| {
            frame
                .iter()
                .rev()
                .find(|(s, _)| *s == name)
                .map(|(_, t)| t.clone())
        })
    }

    /// A variable read at type `num` whose declared type is a bounded
    /// integer: the model's implicit specification widening, exported as an
    /// explicit `cast` so that no conversion is implicit in the XAST.
    fn widened(
        &self,
        id: NodeId,
        node_ty: &Type,
        declared: Option<Type>,
        node: xast::ExpNode,
    ) -> Result<xast::Exp> {
        let is_bounded_int = |t: &Type| {
            matches!(
                t,
                Type::Primitive(
                    PrimitiveType::U8
                        | PrimitiveType::U16
                        | PrimitiveType::U32
                        | PrimitiveType::U64
                        | PrimitiveType::U128
                        | PrimitiveType::U256
                        | PrimitiveType::I8
                        | PrimitiveType::I16
                        | PrimitiveType::I32
                        | PrimitiveType::I64
                        | PrimitiveType::I128
                        | PrimitiveType::I256
                )
            )
        };
        let loc = self.node_loc(id);
        match declared {
            Some(declared)
                if matches!(node_ty, Type::Primitive(PrimitiveType::Num))
                    && is_bounded_int(&declared) =>
            {
                Ok(xast::Exp {
                    ty: self.ty(node_ty)?,
                    loc,
                    node: xast::ExpNode::Call {
                        op: xast::Operation::Cast,
                        inst: vec![],
                        args: vec![xast::Exp {
                            ty: self.ty(&declared)?,
                            loc,
                            node,
                        }],
                        surface: None,
                    },
                })
            },
            _ => Ok(xast::Exp {
                ty: self.ty(node_ty)?,
                loc,
                node,
            }),
        }
    }

    fn pool(&self) -> &'a move_model::symbol::SymbolPool {
        self.env.symbol_pool()
    }

    fn name(&self, sym: Symbol) -> String {
        sym.display(self.pool()).to_string()
    }

    fn file_index(&self, loc: &Loc) -> usize {
        let mut files = self.files.borrow_mut();
        if let Some(i) = files.iter().position(|f| f.file_id() == loc.file_id()) {
            i
        } else {
            files.push(loc.clone());
            files.len() - 1
        }
    }

    fn loc(&self, loc: &Loc) -> xast::LocId {
        let span = loc.span();
        let value = xast::Loc {
            file: self.file_index(loc),
            start: span.start().0,
            end: span.end().0,
        };
        self.locs.borrow_mut().intern(value)
    }

    fn node_loc(&self, id: NodeId) -> xast::LocId {
        self.loc(&self.env.get_node_loc(id))
    }

    // ---------------------------------------------------------------------------------------------
    // Names and addresses

    /// The `0x`-hex form of an address and, for a symbolic address, its name.
    fn address(&self, addr: &Address) -> Result<(String, Option<String>)> {
        match addr {
            Address::Numerical(a) => Ok((a.to_hex_literal(), None)),
            Address::Symbolic(sym) => {
                let name = self.name(*sym);
                let value = self
                    .env
                    .get_address_alias_map()
                    .get(sym)
                    .ok_or_else(|| anyhow!("unknown named address `{}`", name))?;
                Ok((value.to_hex_literal(), Some(name)))
            },
        }
    }

    /// The module reference, with the named address the module was declared
    /// under.  The model resolves named addresses to numbers, so the alias
    /// is read from the module header in the source (`module alias::name`);
    /// a module without source, or declared under a numeric address, has
    /// none.
    fn intern_module(&self, r: xast::ModuleRef) -> xast::ModuleId {
        self.modules.borrow_mut().intern(r)
    }

    fn module_ref(&self, module_env: &ModuleEnv) -> xast::ModuleId {
        if let Some(id) = self.model_modules.borrow().get(&module_env.get_id()) {
            return *id;
        }
        let name = module_env.get_name();
        let (address, symbolic_alias) = self
            .address(name.addr())
            .unwrap_or_else(|_| (String::from("0x0"), None));
        let alias = symbolic_alias.or_else(|| {
            let source = self.env.get_source(&module_env.get_loc()).ok()?;
            let declared = header_alias(source)?;
            let value = self
                .env
                .get_address_alias_map()
                .get(&self.pool().make(&declared))?;
            (value.to_hex_literal() == address).then_some(declared)
        });
        let id = self.intern_module(xast::ModuleRef {
            address,
            address_alias: alias,
            name: self.name(name.name()),
        });
        self.model_modules
            .borrow_mut()
            .insert(module_env.get_id(), id);
        id
    }

    fn module_ref_of(&self, module_id: ModuleId) -> xast::ModuleId {
        self.module_ref(&self.env.get_module(module_id))
    }

    fn friend_ref(&self, decl: &FriendDecl) -> Result<xast::ModuleId> {
        if let Some(id) = decl.module_id {
            return Ok(self.module_ref_of(id));
        }
        let (address, address_alias) = self.address(decl.module_name.addr())?;
        Ok(self.intern_module(xast::ModuleRef {
            address,
            address_alias,
            name: self.name(decl.module_name.name()),
        }))
    }

    fn qualified(&self, module_id: ModuleId, name: Symbol) -> xast::NameId {
        let value = xast::QualifiedName {
            module: self.module_ref_of(module_id),
            name: self.name(name),
        };
        self.names.borrow_mut().intern(value)
    }

    fn struct_name(
        &self,
        module_id: ModuleId,
        struct_id: move_model::model::StructId,
    ) -> xast::NameId {
        let struct_env = self.env.get_module(module_id).into_struct(struct_id);
        self.qualified(module_id, struct_env.get_name())
    }

    fn function_name(&self, module_id: ModuleId, fun_id: move_model::model::FunId) -> xast::NameId {
        let fun_env = self.env.get_module(module_id).into_function(fun_id);
        self.qualified(module_id, fun_env.get_name())
    }

    fn field_name(
        &self,
        module_id: ModuleId,
        struct_id: move_model::model::StructId,
        field_id: move_model::model::FieldId,
    ) -> String {
        let struct_env = self.env.get_module(module_id).into_struct(struct_id);
        self.name(struct_env.get_field(field_id).get_name())
    }

    // ---------------------------------------------------------------------------------------------
    // Types and values

    fn abilities(&self, set: AbilitySet) -> Vec<xast::Ability> {
        let mut out = vec![];
        if set.has_copy() {
            out.push(xast::Ability::Copy);
        }
        if set.has_drop() {
            out.push(xast::Ability::Drop);
        }
        if set.has_store() {
            out.push(xast::Ability::Store);
        }
        if set.has_key() {
            out.push(xast::Ability::Key);
        }
        out
    }

    fn type_params(&self, params: &[TypeParameter]) -> Vec<xast::TypeParam> {
        params
            .iter()
            .map(|TypeParameter(name, kind, _)| xast::TypeParam {
                name: self.name(*name),
                abilities: self.abilities(kind.abilities),
                is_phantom: kind.is_phantom,
            })
            .collect()
    }

    fn params(&self, params: &[Parameter]) -> Result<Vec<xast::Param>> {
        params
            .iter()
            .map(|Parameter(name, ty, _)| {
                Ok(xast::Param {
                    name: self.name(*name),
                    ty: self.ty(ty)?,
                })
            })
            .collect()
    }

    fn ty(&self, ty: &Type) -> Result<xast::TypeId> {
        use xast::Type as X;
        let value = match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::Bool => X::Bool,
                PrimitiveType::U8 => X::U8,
                PrimitiveType::U16 => X::U16,
                PrimitiveType::U32 => X::U32,
                PrimitiveType::U64 => X::U64,
                PrimitiveType::U128 => X::U128,
                PrimitiveType::U256 => X::U256,
                PrimitiveType::I8 => X::I8,
                PrimitiveType::I16 => X::I16,
                PrimitiveType::I32 => X::I32,
                PrimitiveType::I64 => X::I64,
                PrimitiveType::I128 => X::I128,
                PrimitiveType::I256 => X::I256,
                PrimitiveType::Address => X::Address,
                PrimitiveType::Signer => X::Signer,
                PrimitiveType::Num => X::Num,
                PrimitiveType::Range => X::Range,
                PrimitiveType::EventStore => X::EventStore,
            },
            Type::Tuple(ts) => X::Tuple(self.tys(ts)?),
            Type::Vector(t) => X::Vector(self.ty(t)?),
            Type::Struct(mid, sid, args) => X::Struct {
                name: self.struct_name(*mid, *sid),
                args: self.tys(args)?,
            },
            Type::TypeParameter(i) => X::TypeParam(*i),
            Type::Fun(args, result, abilities) => X::Function {
                args: self.ty(args)?,
                result: self.ty(result)?,
                abilities: self.abilities(*abilities),
            },
            Type::Reference(kind, t) => X::Reference {
                mutable: *kind == ReferenceKind::Mutable,
                ty: self.ty(t)?,
            },
            Type::TypeDomain(t) => X::TypeDomain(self.ty(t)?),
            Type::ResourceDomain(mid, sid, args) => X::ResourceDomain {
                name: self.struct_name(*mid, *sid),
                args: match args {
                    Some(args) => Some(self.tys(args)?),
                    None => None,
                },
            },
            Type::StateDomain => X::StateDomain,
            Type::Error => bail!("error type in checked model"),
            Type::Var(_) => bail!("unresolved type variable in checked model"),
        };
        Ok(self.types.borrow_mut().intern(value))
    }

    fn tys(&self, tys: &[Type]) -> Result<Vec<xast::TypeId>> {
        tys.iter().map(|t| self.ty(t)).collect()
    }

    fn value(&self, value: &Value) -> Result<xast::Value> {
        Ok(match value {
            Value::Address(a) => xast::Value::Address(self.address(a)?.0),
            Value::Number(n) => xast::Value::Number(n.to_string()),
            Value::Bool(b) => xast::Value::Bool(*b),
            Value::ByteArray(bytes) => xast::Value::Vector(
                bytes
                    .iter()
                    .map(|b| xast::Value::Number(b.to_string()))
                    .collect(),
            ),
            Value::AddressArray(addrs) => xast::Value::Vector(
                addrs
                    .iter()
                    .map(|a| Ok(xast::Value::Address(self.address(a)?.0)))
                    .collect::<Result<Vec<_>>>()?,
            ),
            Value::Vector(vs) => xast::Value::Vector(self.values(vs)?),
            Value::Tuple(vs) => xast::Value::Tuple(self.values(vs)?),
        })
    }

    fn values(&self, values: &[Value]) -> Result<Vec<xast::Value>> {
        values.iter().map(|v| self.value(v)).collect()
    }

    // ---------------------------------------------------------------------------------------------
    // Pragmas, properties, attributes

    fn pragmas(&self, bag: &PropertyBag) -> Result<Vec<xast::Pragma>> {
        bag.iter()
            .map(|(name, value)| {
                Ok(xast::Pragma {
                    name: self.name(*name),
                    value: match value {
                        PropertyValue::Value(v) => xast::PragmaValue::Value(self.value(v)?),
                        PropertyValue::Symbol(s) => xast::PragmaValue::Name(self.name(*s)),
                        PropertyValue::QualifiedSymbol(qs) => {
                            xast::PragmaValue::QualifiedName(format!(
                                "{}::{}",
                                qs.module_name.display_full(self.env),
                                self.name(qs.symbol)
                            ))
                        },
                    },
                })
            })
            .collect()
    }

    /// The resolved pragma view of a function: its own properties, and the
    /// module's where the function does not set them.
    fn resolved_pragmas(&self, fun_env: &FunctionEnv) -> Result<Vec<xast::Pragma>> {
        let own = fun_env.get_spec();
        let module = fun_env.module_env.get_spec();
        let mut merged: BTreeMap<Symbol, PropertyValue> = module.properties.clone();
        for (name, value) in &own.properties {
            merged.insert(*name, value.clone());
        }
        self.pragmas(&merged)
    }

    fn attributes(&self, attrs: &[Attribute]) -> Result<Vec<xast::Attribute>> {
        attrs
            .iter()
            .map(|attr| {
                Ok(match attr {
                    Attribute::Apply(_, name, args) => xast::Attribute::Apply {
                        name: self.name(*name),
                        args: self.attributes(args)?,
                    },
                    Attribute::Assign(_, name, value) => xast::Attribute::Assign {
                        name: self.name(*name),
                        value: match value {
                            AttributeValue::Value(_, v) => {
                                xast::AttributeValue::Value(self.value(v)?)
                            },
                            AttributeValue::Name(_, module, sym) => xast::AttributeValue::Name {
                                module: match module {
                                    Some(m) => {
                                        let (address, address_alias) = self.address(m.addr())?;
                                        Some(self.intern_module(xast::ModuleRef {
                                            address,
                                            address_alias,
                                            name: self.name(m.name()),
                                        }))
                                    },
                                    None => None,
                                },
                                name: self.name(*sym),
                            },
                        },
                    },
                })
            })
            .collect()
    }

    // ---------------------------------------------------------------------------------------------
    // Declarations

    fn constant(&self, constant: &NamedConstantEnv) -> Result<xast::Constant> {
        Ok(xast::Constant {
            name: self.name(constant.get_name()),
            doc: constant.get_doc().to_string(),
            loc: self.loc(&constant.get_loc()),
            ty: self.ty(&constant.get_type())?,
            value: self.value(&constant.get_value())?,
        })
    }

    fn field(&self, field: &FieldEnv) -> Result<xast::Field> {
        Ok(xast::Field {
            name: self.name(field.get_name()),
            doc: field.get_doc().to_string(),
            ty: self.ty(&field.get_type())?,
        })
    }

    fn intrinsic(&self, struct_env: &StructEnv) -> Option<xast::Intrinsic> {
        let decl = self
            .env
            .get_intrinsics()
            .get_decl_for_struct(&struct_env.get_qualified_id())?;
        let move_functions = decl
            .get_move_fun_bindings(self.env)
            .into_iter()
            .map(|(role, target)| xast::IntrinsicBinding {
                role,
                target: self.function_name(target.module_id, target.id),
            })
            .collect();
        let spec_functions = decl
            .get_spec_fun_bindings(self.env)
            .into_iter()
            .map(|(role, target)| {
                let module = self.env.get_module(target.module_id);
                let spec_fun = module.get_spec_fun(target.id);
                xast::IntrinsicBinding {
                    role,
                    target: self.qualified(target.module_id, spec_fun.name),
                }
            })
            .collect();
        Some(xast::Intrinsic {
            name: decl.get_intrinsic_type_name(self.env),
            move_functions,
            spec_functions,
        })
    }

    fn struct_decl(&self, struct_env: &StructEnv) -> Result<xast::Struct> {
        let (fields, variants) = if struct_env.has_variants() {
            let mut variants = vec![];
            for variant in struct_env.get_variants() {
                let fields = struct_env
                    .get_fields_of_variant(variant)
                    .map(|f| self.field(&f))
                    .collect::<Result<Vec<_>>>()?;
                variants.push(xast::Variant {
                    name: self.name(variant),
                    loc: self.loc(struct_env.get_variant_loc(variant)),
                    fields,
                });
            }
            (vec![], Some(variants))
        } else {
            (
                struct_env
                    .get_fields()
                    .map(|f| self.field(&f))
                    .collect::<Result<Vec<_>>>()?,
                None,
            )
        };
        Ok(xast::Struct {
            name: self.name(struct_env.get_name()),
            doc: struct_env.get_doc().to_string(),
            loc: self.loc(&struct_env.get_loc()),
            abilities: self.abilities(struct_env.get_abilities()),
            type_params: self.type_params(struct_env.get_type_parameters()),
            attributes: self.attributes(struct_env.get_attributes())?,
            is_native: struct_env.is_native(),
            fields,
            variants,
            spec: self.spec(&struct_env.get_spec())?,
            intrinsic: self.intrinsic(struct_env),
        })
    }

    fn function(&self, fun_env: &FunctionEnv) -> Result<xast::Function> {
        let visibility = if fun_env.has_package_visibility() {
            xast::Visibility::Package
        } else {
            match fun_env.visibility() {
                Visibility::Private => xast::Visibility::Private,
                Visibility::Public => xast::Visibility::Public,
                Visibility::Friend => xast::Visibility::Friend,
            }
        };
        let kind = if fun_env.is_native() {
            xast::FunctionKind::Native
        } else if fun_env.is_inline() {
            xast::FunctionKind::InlineRetained
        } else {
            xast::FunctionKind::Regular
        };
        let param_types = fun_env
            .get_parameters()
            .iter()
            .map(|Parameter(_, ty, _)| ty.clone())
            .collect();
        let (body, spec) = self.with_params(param_types, || {
            let body = match fun_env.get_def() {
                Some(def) => Some(self.exp(def.as_ref()).map_err(|e| {
                    anyhow!("in function `{}`: {:#}", fun_env.get_full_name_str(), e)
                })?),
                None => None,
            };
            Ok((body, self.spec(&fun_env.get_spec())?))
        })?;
        Ok(xast::Function {
            name: self.name(fun_env.get_name()),
            doc: fun_env.get_doc().to_string(),
            loc: self.loc(&fun_env.get_loc()),
            visibility,
            is_entry: fun_env.is_entry(),
            kind,
            is_receiver: fun_env.is_receiver_function(),
            attributes: self.attributes(fun_env.get_attributes())?,
            type_params: self.type_params(&fun_env.get_type_parameters()),
            params: self.params(&fun_env.get_parameters())?,
            result: self.ty(&fun_env.get_result_type())?,
            pragmas: self.resolved_pragmas(fun_env)?,
            spec,
            body,
        })
    }

    fn spec_fun(&self, decl: &SpecFunDecl) -> Result<xast::SpecFun> {
        // Spec function parameters are read as locals (by name) or as
        // temporaries (by index).
        let param_types: Vec<Type> = decl
            .params
            .iter()
            .map(|Parameter(_, ty, _)| ty.clone())
            .collect();
        let param_vars = decl
            .params
            .iter()
            .map(|Parameter(name, ty, _)| (*name, ty.clone()))
            .collect();
        let (body, spec) = self.with_params(param_types, || {
            self.with_scope(param_vars, || {
                let body = match &decl.body {
                    Some(body) => Some(self.exp(body.as_ref())?),
                    None => None,
                };
                Ok((body, self.spec(&decl.spec.borrow())?))
            })
        })?;
        Ok(xast::SpecFun {
            name: self.name(decl.name),
            doc: self.env.get_doc(&decl.loc).to_string(),
            loc: self.loc(&decl.loc),
            type_params: self.type_params(&decl.type_params),
            params: self.params(&decl.params)?,
            result: self.ty(&decl.result_type)?,
            uninterpreted: decl.uninterpreted,
            is_native: decl.is_native,
            is_move_fun: decl.is_move_fun,
            uses_old: decl.uses_old,
            body,
            spec,
        })
    }

    fn spec_var(&self, decl: &SpecVarDecl) -> Result<xast::SpecVar> {
        Ok(xast::SpecVar {
            name: self.name(decl.name),
            loc: self.loc(&decl.loc),
            type_params: self.type_params(&decl.type_params),
            ty: self.ty(&decl.type_)?,
            init: match &decl.init {
                Some(init) => Some(self.exp(init.as_ref())?),
                None => None,
            },
        })
    }

    fn global_invariant(&self, inv: &GlobalInvariant) -> Result<xast::Invariant> {
        let (kind, type_params) = match &inv.kind {
            ConditionKind::GlobalInvariant(tps) => (xast::InvariantKind::Global, tps),
            ConditionKind::GlobalInvariantUpdate(tps) => (xast::InvariantKind::GlobalUpdate, tps),
            ConditionKind::Axiom(tps) => (xast::InvariantKind::Axiom, tps),
            kind => bail!("unexpected global invariant kind {:?}", kind),
        };
        Ok(xast::Invariant {
            kind,
            loc: self.loc(&inv.loc),
            type_params: type_params.iter().map(|(s, _)| self.name(*s)).collect(),
            properties: self.pragmas(&inv.properties)?,
            exp: self.exp(inv.cond.as_ref())?,
        })
    }

    // ---------------------------------------------------------------------------------------------
    // Specifications

    fn spec(&self, spec: &Spec) -> Result<xast::Spec> {
        let frame = match &spec.frame_spec {
            Some(fs) => Some(xast::Frame {
                modifies: fs
                    .modifies_targets
                    .iter()
                    .map(|e| self.exp(e.as_ref()))
                    .collect::<Result<Vec<_>>>()?,
                reads: fs
                    .reads_targets
                    .iter()
                    .map(|q| self.ty(&Type::Struct(q.module_id, q.id, q.inst.clone())))
                    .collect::<Result<Vec<_>>>()?,
                modifies_all: fs.modifies_all,
                reads_all: fs.reads_all,
            }),
            None => None,
        };
        // A spec `let` binds its name, at its value's type, for the
        // conditions after it.
        let conditions = self.with_scope(vec![], || {
            let mut conditions = Vec::new();
            for cond in &spec.conditions {
                conditions.push(self.condition(cond)?);
                if let ConditionKind::LetPre(name, _) | ConditionKind::LetPost(name, _) = &cond.kind
                {
                    let ty = self.env.get_node_type(cond.exp.node_id());
                    if let Some(frame) = self.scopes.borrow_mut().last_mut() {
                        frame.push((*name, ty));
                    }
                }
            }
            Ok(conditions)
        })?;
        Ok(xast::Spec {
            loc: spec.loc.as_ref().map(|l| self.loc(l)),
            pragmas: self.pragmas(&spec.properties)?,
            conditions,
            frame,
        })
    }

    fn condition(&self, cond: &Condition) -> Result<xast::Condition> {
        let names = |syms: &[(Symbol, Loc)]| syms.iter().map(|(s, _)| self.name(*s)).collect();
        let kind = match &cond.kind {
            ConditionKind::LetPost(name, _) => xast::ConditionKind::LetPost {
                name: self.name(*name),
            },
            ConditionKind::LetPre(name, _) => xast::ConditionKind::LetPre {
                name: self.name(*name),
            },
            ConditionKind::Assert => xast::ConditionKind::Assert,
            ConditionKind::Assume => xast::ConditionKind::Assume,
            ConditionKind::Decreases => xast::ConditionKind::Decreases,
            ConditionKind::AbortsIf => xast::ConditionKind::AbortsIf,
            ConditionKind::AbortsWith => xast::ConditionKind::AbortsWith,
            ConditionKind::SucceedsIf => xast::ConditionKind::SucceedsIf,
            ConditionKind::Emits => xast::ConditionKind::Emits,
            ConditionKind::Ensures => xast::ConditionKind::Ensures,
            ConditionKind::Requires => xast::ConditionKind::Requires,
            ConditionKind::StructInvariant => xast::ConditionKind::StructInvariant,
            ConditionKind::FunctionInvariant => xast::ConditionKind::FunctionInvariant,
            ConditionKind::LoopInvariant => xast::ConditionKind::LoopInvariant,
            ConditionKind::GlobalInvariant(tps) => xast::ConditionKind::GlobalInvariant {
                type_params: names(tps),
            },
            ConditionKind::GlobalInvariantUpdate(tps) => {
                xast::ConditionKind::GlobalInvariantUpdate {
                    type_params: names(tps),
                }
            },
            ConditionKind::SchemaInvariant => xast::ConditionKind::SchemaInvariant,
            ConditionKind::Axiom(tps) => xast::ConditionKind::Axiom {
                type_params: names(tps),
            },
            ConditionKind::Update => xast::ConditionKind::Update,
        };
        let additional = cond
            .additional_exps
            .iter()
            .map(|e| self.exp(e.as_ref()))
            .collect::<Result<Vec<_>>>()?;
        let mut out = xast::Condition {
            kind,
            loc: self.loc(&cond.loc),
            properties: self.pragmas(&cond.properties)?,
            exp: self.exp(cond.exp.as_ref())?,
            abort_code: None,
            additional_codes: vec![],
            emits_handle: None,
            emits_condition: None,
            update_target: None,
        };
        match &cond.kind {
            ConditionKind::AbortsIf => {
                let mut it = additional.into_iter();
                out.abort_code = it.next();
                if it.next().is_some() {
                    bail!("`aborts_if` with more than one code");
                }
            },
            ConditionKind::AbortsWith => out.additional_codes = additional,
            ConditionKind::Emits => {
                let mut it = additional.into_iter();
                out.emits_handle = it.next();
                out.emits_condition = it.next();
            },
            ConditionKind::Update => {
                let mut it = additional.into_iter();
                out.update_target = it.next();
            },
            _ => {
                if !additional.is_empty() {
                    bail!("unexpected additional expressions on {:?}", cond.kind);
                }
            },
        }
        Ok(out)
    }

    // ---------------------------------------------------------------------------------------------
    // Expressions

    fn exps(&self, exps: &[move_model::ast::Exp]) -> Result<Vec<xast::Exp>> {
        exps.iter().map(|e| self.exp(e.as_ref())).collect()
    }

    fn boxed(&self, exp: &ExpData) -> Result<Box<xast::Exp>> {
        Ok(Box::new(self.exp(exp)?))
    }

    fn exp(&self, exp: &ExpData) -> Result<xast::Exp> {
        let id = exp.node_id();
        let node = match exp {
            ExpData::Invalid(_) => bail!("invalid expression in checked model"),
            ExpData::Value(_, v) => xast::ExpNode::Value {
                value: self.value(v)?,
                constant: self.constant_name(id, v),
            },
            ExpData::LocalVar(_, name) => {
                let node = xast::ExpNode::Local {
                    name: self.name(*name),
                };
                return self.widened(
                    id,
                    &self.env.get_node_type(id),
                    self.declared_type(*name),
                    node,
                );
            },
            ExpData::Temporary(_, index) => {
                let node = xast::ExpNode::Param { index: *index };
                let declared = self.params.borrow().get(*index).cloned();
                return self.widened(id, &self.env.get_node_type(id), declared, node);
            },
            ExpData::Call(_, op, args) => {
                let surface = if self.env.has_surface_syntax(id, SurfaceSyntax::ReceiverCall) {
                    Some(xast::SurfaceSyntax::ReceiverCall)
                } else if self
                    .env
                    .has_surface_syntax(id, SurfaceSyntax::IndexNotation)
                {
                    Some(xast::SurfaceSyntax::IndexNotation)
                } else {
                    None
                };
                xast::ExpNode::Call {
                    op: self.operation(op)?,
                    inst: self.tys(&self.env.get_node_instantiation(id))?,
                    args: self.exps(args)?,
                    surface,
                }
            },
            ExpData::Invoke(_, function, args) => xast::ExpNode::Invoke {
                function: Box::new(self.exp(function.as_ref())?),
                args: self.exps(args)?,
            },
            ExpData::Lambda(..) => {
                bail!("lambda is not supported by XAST (function values are out of scope)")
            },
            ExpData::Quant(_, kind, ranges, triggers, condition, body) => {
                let quant = match kind {
                    QuantKind::Forall => xast::QuantKind::Forall,
                    QuantKind::Exists => xast::QuantKind::Exists,
                    QuantKind::Choose => xast::QuantKind::Choose,
                    QuantKind::ChooseMin => xast::QuantKind::ChooseMin,
                };
                let xranges = ranges
                    .iter()
                    .map(|(pat, domain)| {
                        Ok(xast::QuantRange {
                            pattern: self.pattern(pat)?,
                            domain: self.exp(domain.as_ref())?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let vars = ranges
                    .iter()
                    .flat_map(|(pat, _)| self.pattern_vars(pat))
                    .collect();
                self.with_scope(vars, || {
                    Ok(xast::ExpNode::Quant {
                        quant,
                        ranges: xranges,
                        triggers: triggers
                            .iter()
                            .map(|t| self.exps(t))
                            .collect::<Result<Vec<_>>>()?,
                        condition: match condition {
                            Some(c) => Some(self.boxed(c.as_ref())?),
                            None => None,
                        },
                        body: self.boxed(body.as_ref())?,
                    })
                })?
            },
            ExpData::Block(_, pat, binding, body) => {
                let pattern = self.pattern(pat)?;
                let binding = match binding {
                    Some(b) => Some(self.boxed(b.as_ref())?),
                    None => None,
                };
                let body = self.with_scope(self.pattern_vars(pat), || self.boxed(body.as_ref()))?;
                xast::ExpNode::Block {
                    pattern,
                    binding,
                    body,
                }
            },
            ExpData::IfElse(_, c, t, e) => xast::ExpNode::If {
                cond: self.boxed(c.as_ref())?,
                then_branch: self.boxed(t.as_ref())?,
                else_branch: self.boxed(e.as_ref())?,
            },
            ExpData::Match(_, scrutinee, arms) => xast::ExpNode::Match {
                scrutinee: self.boxed(scrutinee.as_ref())?,
                arms: arms
                    .iter()
                    .map(|arm| {
                        let pattern = self.pattern(&arm.pattern)?;
                        self.with_scope(self.pattern_vars(&arm.pattern), || {
                            Ok(xast::MatchArm {
                                loc: self.loc(&arm.loc),
                                pattern,
                                guard: match &arm.condition {
                                    Some(c) => Some(self.exp(c.as_ref())?),
                                    None => None,
                                },
                                body: self.exp(arm.body.as_ref())?,
                            })
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
            ExpData::Return(_, value) => xast::ExpNode::Return {
                value: self.boxed(value.as_ref())?,
            },
            ExpData::Sequence(_, exps) => xast::ExpNode::Sequence {
                exps: self.exps(exps)?,
            },
            ExpData::Loop(_, body) => xast::ExpNode::Loop {
                body: self.boxed(body.as_ref())?,
            },
            ExpData::LoopCont(_, nest, is_continue) => xast::ExpNode::LoopCont {
                nest: *nest,
                is_continue: *is_continue,
            },
            ExpData::Assign(_, pat, value) => xast::ExpNode::Assign {
                pattern: self.pattern(pat)?,
                value: self.boxed(value.as_ref())?,
            },
            ExpData::Mutate(_, target, value) => xast::ExpNode::Mutate {
                target: self.boxed(target.as_ref())?,
                value: self.boxed(value.as_ref())?,
            },
            ExpData::SpecBlock(_, spec) => xast::ExpNode::SpecBlock {
                spec: self.spec(spec)?,
            },
        };
        Ok(xast::Exp {
            ty: self.ty(&self.env.get_node_type(id))?,
            loc: self.node_loc(id),
            node,
        })
    }

    /// The module constant the source names at a value node: the node's span
    /// is an identifier and a constant of that name has this value.
    fn constant_name(&self, id: NodeId, value: &Value) -> Option<String> {
        let loc = self.env.get_node_loc(id);
        let text = self.env.get_source(&loc).ok()?.trim();
        let is_ident = !text.is_empty()
            && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !text.chars().next().is_some_and(|c| c.is_ascii_digit());
        if !is_ident {
            return None;
        }
        match self.constants.get(text) {
            Some(v) if v == value => Some(text.to_string()),
            _ => None,
        }
    }

    fn pattern(&self, pat: &Pattern) -> Result<xast::Pattern> {
        let id = pat.node_id();
        let node = match pat {
            Pattern::Var(_, name) => xast::PatternNode::Var {
                name: self.name(*name),
            },
            Pattern::Wildcard(_) => xast::PatternNode::Wildcard,
            Pattern::Tuple(_, pats) => xast::PatternNode::Tuple {
                elements: pats
                    .iter()
                    .map(|p| self.pattern(p))
                    .collect::<Result<Vec<_>>>()?,
            },
            Pattern::Struct(_, qid, variant, pats) => xast::PatternNode::Struct {
                name: self.struct_name(qid.module_id, qid.id),
                inst: self.tys(&qid.inst)?,
                variant: variant.map(|v| self.name(v)),
                fields: pats
                    .iter()
                    .map(|p| self.pattern(p))
                    .collect::<Result<Vec<_>>>()?,
            },
            Pattern::LiteralValue(_, value) => xast::PatternNode::Literal {
                value: self.value(value)?,
            },
            Pattern::Range(_, lower, upper, inclusive) => xast::PatternNode::Range {
                lower: match lower {
                    Some(v) => Some(self.value(v)?),
                    None => None,
                },
                upper: match upper {
                    Some(v) => Some(self.value(v)?),
                    None => None,
                },
                inclusive: *inclusive,
            },
            Pattern::Error(_) => bail!("error pattern in checked model"),
        };
        Ok(xast::Pattern {
            ty: self.ty(&self.env.get_node_type(id))?,
            loc: self.node_loc(id),
            node,
        })
    }

    fn memory_range(&self, range: &MemoryRange) -> xast::MemoryRange {
        xast::MemoryRange {
            pre: range.pre.map(|l| l.as_usize() as u64),
            post: range.post.map(|l| l.as_usize() as u64),
        }
    }

    fn operation(&self, op: &Operation) -> Result<xast::Operation> {
        use xast::Operation as X;
        Ok(match op {
            Operation::MoveFunction(mid, fid) => X::MoveFunction(self.function_name(*mid, *fid)),
            Operation::Pack(mid, sid, variant) => X::Pack {
                name: self.struct_name(*mid, *sid),
                variant: variant.map(|v| self.name(v)),
            },
            Operation::Closure(..) => {
                bail!("closures are not supported by XAST (function values are out of scope)")
            },
            Operation::Tuple => X::Tuple,
            Operation::Select(mid, sid, fid) => X::Select {
                name: self.struct_name(*mid, *sid),
                field: self.field_name(*mid, *sid, *fid),
            },
            Operation::SelectVariants(mid, sid, fids) => X::SelectVariants {
                name: self.struct_name(*mid, *sid),
                fields: fids
                    .iter()
                    .map(|fid| self.field_name(*mid, *sid, *fid))
                    .collect(),
            },
            Operation::TestVariants(mid, sid, variants) => X::TestVariants {
                name: self.struct_name(*mid, *sid),
                variants: variants.iter().map(|v| self.name(*v)).collect(),
            },
            Operation::SpecFunction(mid, sfid, range) => {
                let module_env = self.env.get_module(*mid);
                let decl = module_env.get_spec_fun(*sfid);
                X::SpecFunction {
                    name: self.qualified(*mid, decl.name),
                    range: self.memory_range(range),
                }
            },
            Operation::UpdateField(mid, sid, fid) => X::UpdateField {
                name: self.struct_name(*mid, *sid),
                field: self.field_name(*mid, *sid, *fid),
            },
            Operation::Behavior(kind, range) => X::Behavior {
                kind: match kind {
                    BehaviorKind::RequiresOf => xast::BehaviorKind::RequiresOf,
                    BehaviorKind::AbortsOf => xast::BehaviorKind::AbortsOf,
                    BehaviorKind::EnsuresOf => xast::BehaviorKind::EnsuresOf,
                    BehaviorKind::ResultOf => xast::BehaviorKind::ResultOf,
                    BehaviorKind::UnchangedOf => xast::BehaviorKind::UnchangedOf,
                    BehaviorKind::FoldsOf => xast::BehaviorKind::FoldsOf,
                    BehaviorKind::WriteOf(index) => xast::BehaviorKind::WriteOf(*index),
                },
                range: self.memory_range(range),
            },
            Operation::Result(i) => X::Result(*i),
            Operation::Index => X::Index,
            Operation::Slice => X::Slice,
            Operation::Range => X::Range,
            Operation::Implies => X::Implies,
            Operation::Iff => X::Iff,
            Operation::Identical => X::Identical,
            Operation::Add => X::Add,
            Operation::Sub => X::Sub,
            Operation::Mul => X::Mul,
            Operation::Mod => X::Mod,
            Operation::Div => X::Div,
            Operation::BitOr => X::BitOr,
            Operation::BitAnd => X::BitAnd,
            Operation::Xor => X::Xor,
            Operation::Shl => X::Shl,
            Operation::Shr => X::Shr,
            Operation::And => X::And,
            Operation::Or => X::Or,
            Operation::Eq => X::Eq,
            Operation::Neq => X::Neq,
            Operation::Lt => X::Lt,
            Operation::Gt => X::Gt,
            Operation::Le => X::Le,
            Operation::Ge => X::Ge,
            Operation::Copy => X::Copy,
            Operation::Move => X::Move,
            Operation::Not => X::Not,
            Operation::Cast => X::Cast,
            Operation::Negate => X::Negate,
            Operation::Exists(label) => X::Exists(label.map(|l| l.as_usize() as u64)),
            Operation::BorrowGlobal(kind) => X::BorrowGlobal(self.ref_kind(*kind)),
            Operation::Borrow(kind) => X::Borrow(self.ref_kind(*kind)),
            Operation::Deref => X::Deref,
            Operation::MoveTo => X::MoveTo,
            Operation::MoveFrom => X::MoveFrom,
            Operation::Freeze(explicit) => X::Freeze(*explicit),
            Operation::Abort(kind) => X::Abort(match kind {
                AbortKind::Code => xast::AbortKind::Code,
                AbortKind::Message => xast::AbortKind::Message,
            }),
            Operation::Vector => X::Vector,
            Operation::Len => X::Len,
            Operation::TypeValue => X::TypeValue,
            Operation::TypeDomain => X::TypeDomain,
            Operation::ResourceDomain => X::ResourceDomain,
            Operation::StateDomain => X::StateDomain,
            Operation::Global(label) => X::Global(label.map(|l| l.as_usize() as u64)),
            Operation::CanModify => X::CanModify,
            Operation::Old => X::Old,
            Operation::SaveStateAnchor(l) => X::SaveStateAnchor(l.as_usize() as u64),
            Operation::WithStateAnchor(l) => X::WithStateAnchor(l.as_usize() as u64),
            Operation::FoldsCaptureAnchor(l) => X::FoldsCaptureAnchor(l.as_usize() as u64),
            Operation::InlineCallSummary => X::InlineCallSummary,
            Operation::Trace(kind) => X::Trace(match kind {
                TraceKind::User => xast::TraceKind::User,
                TraceKind::Auto => xast::TraceKind::Auto,
                TraceKind::SubAuto => xast::TraceKind::SubAuto,
            }),
            Operation::SpecPublish(r) => X::SpecPublish(self.memory_range(r)),
            Operation::SpecRemove(r) => X::SpecRemove(self.memory_range(r)),
            Operation::SpecUpdate(r) => X::SpecUpdate(self.memory_range(r)),
            Operation::EmptyVec => X::EmptyVec,
            Operation::SingleVec => X::SingleVec,
            Operation::UpdateVec => X::UpdateVec,
            Operation::ConcatVec => X::ConcatVec,
            Operation::IndexOfVec => X::IndexOfVec,
            Operation::ContainsVec => X::ContainsVec,
            Operation::InRangeRange => X::InRangeRange,
            Operation::InRangeVec => X::InRangeVec,
            Operation::RangeVec => X::RangeVec,
            Operation::MaxU8 => X::MaxU8,
            Operation::MaxU16 => X::MaxU16,
            Operation::MaxU32 => X::MaxU32,
            Operation::MaxU64 => X::MaxU64,
            Operation::MaxU128 => X::MaxU128,
            Operation::MaxU256 => X::MaxU256,
            Operation::Bv2Int => X::Bv2Int,
            Operation::Int2Bv => X::Int2Bv,
            Operation::AbortFlag => X::AbortFlag,
            Operation::AbortCode => X::AbortCode,
            Operation::WellFormed => X::WellFormed,
            Operation::BoxValue => X::BoxValue,
            Operation::UnboxValue => X::UnboxValue,
            Operation::EmptyEventStore => X::EmptyEventStore,
            Operation::ExtendEventStore => X::ExtendEventStore,
            Operation::EventStoreIncludes => X::EventStoreIncludes,
            Operation::EventStoreIncludedIn => X::EventStoreIncludedIn,
            Operation::NoOp => X::NoOp,
        })
    }

    fn ref_kind(&self, kind: ReferenceKind) -> xast::RefKind {
        match kind {
            ReferenceKind::Immutable => xast::RefKind::Immutable,
            ReferenceKind::Mutable => xast::RefKind::Mutable,
        }
    }
}

// =================================================================================================
// Source recovery

/// The named address of a module header `module <alias>::<name>`, if the
/// header uses an identifier rather than a numeric address.  `source` starts
/// at the module declaration.
fn header_alias(source: &str) -> Option<String> {
    let rest = source.trim_start();
    let rest = rest.strip_prefix("module")?;
    let rest = rest.strip_prefix(|c: char| c.is_whitespace())?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    let addr = &rest[..end];
    let after = rest[end..].trim_start();
    if !after.starts_with("::") {
        return None;
    }
    let first = addr.chars().next()?;
    if addr.starts_with("0x") || first.is_ascii_digit() {
        return None;
    }
    Some(addr.to_string())
}

/// The byte spans of the ordinary (non-doc) comments of a Move source text:
/// `// ...` line comments other than `///` doc comments (`////` is ordinary
/// again) and `/* ... */` block comments other than `/** ... */` doc comments
/// (`/**/` is ordinary), with nesting, skipping string literals.  Mirrors the
/// lexer's classification in `trim_whitespace_and_comments`.
pub fn scan_comments(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                // String or byte-string literal: skip to the closing quote.
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            },
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                let start = i;
                let is_doc = bytes.get(i + 2) == Some(&b'/') && bytes.get(i + 3) != Some(&b'/');
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                if !is_doc {
                    out.push((start, i));
                }
            },
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                let start = i;
                let is_doc = bytes.get(i + 2) == Some(&b'*') && bytes.get(i + 3) != Some(&b'/');
                let mut depth = 1;
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if !is_doc {
                    out.push((start, i.min(bytes.len())));
                }
            },
            _ => i += 1,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_alias_is_recognized() {
        assert_eq!(
            header_alias("module aptos_framework::coin {"),
            Some("aptos_framework".to_string())
        );
        assert_eq!(header_alias("module 0x42::basic_coin {"), None);
        assert_eq!(
            header_alias("module std :: vector {"),
            Some("std".to_string())
        );
        assert_eq!(header_alias("address 0x1 { module m {} }"), None);
    }

    #[test]
    fn comments_are_classified() {
        let text =
            "// a\n/// doc\n//// not doc\nlet s = \"// no\"; /* b /* c */ */ /** d */ /**/ x";
        let spans = scan_comments(text);
        let texts: Vec<&str> = spans.iter().map(|(s, e)| &text[*s..*e]).collect();
        assert_eq!(texts, vec![
            "// a",
            "//// not doc",
            "/* b /* c */ */",
            "/**/"
        ]);
    }
}
