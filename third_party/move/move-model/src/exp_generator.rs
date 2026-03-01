// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ast::{
        Address, BehaviorKind, BehaviorState, Exp, ExpData, MemoryLabel, Operation, Pattern,
        QuantKind, TempIndex, Value,
    },
    model::{
        FieldEnv, FunId, FunctionEnv, GlobalEnv, Loc, ModuleId, NodeId, QualifiedId,
        QualifiedInstId, SpecFunId, StructEnv, StructId,
    },
    symbol::Symbol,
    ty::{PrimitiveType, Type, BOOL_TYPE, NUM_TYPE},
};
use itertools::Itertools;
use move_core_types::{
    ability::AbilitySet, account_address::AccountAddress, function::ClosureMask,
};
use num::{BigInt, Zero};

/// Indicates which bound(s) to check for an out-of-range condition.
pub enum RangeCheckKind {
    /// value > MAX (overflow only)
    Overflow,
    /// value < MIN (underflow only)
    Underflow,
    /// value < MIN || value > MAX (both)
    Both,
}

/// A trait that defines a generator for `Exp`.
pub trait ExpGenerator<'env> {
    /// Get the function environment
    fn function_env(&self) -> &FunctionEnv<'env>;

    /// Get the current location
    fn get_current_loc(&self) -> Loc;

    /// Set the current location
    fn set_loc(&mut self, loc: Loc);

    /// Add a local variable with given type, return the local index.
    fn add_local(&mut self, ty: Type) -> TempIndex;

    /// Get the type of a local given at `temp` index
    fn get_local_type(&self, temp: TempIndex) -> Type;

    /// Get the global environment
    fn global_env(&self) -> &'env GlobalEnv {
        self.function_env().module_env.env
    }

    /// Sets the default location from a node id.
    fn set_loc_from_node(&mut self, node_id: NodeId) {
        let loc = self.global_env().get_node_loc(node_id);
        self.set_loc(loc);
    }

    /// Creates a new expression node id, using current default location, provided type,
    /// and optional instantiation.
    fn new_node(&self, ty: Type, inst_opt: Option<Vec<Type>>) -> NodeId {
        let node_id = self.global_env().new_node(self.get_current_loc(), ty);
        if let Some(inst) = inst_opt {
            self.global_env().set_node_instantiation(node_id, inst);
        }
        node_id
    }

    /// Allocates a new temporary.
    fn new_temp(&mut self, ty: Type) -> TempIndex {
        self.add_local(ty)
    }

    /// Make a boolean constant expression.
    fn mk_bool_const(&self, value: bool) -> Exp {
        let node_id = self.new_node(BOOL_TYPE.clone(), None);
        ExpData::Value(node_id, Value::Bool(value)).into_exp()
    }

    /// Make an address constant.
    fn mk_address_const(&self, value: AccountAddress) -> Exp {
        let node_id = self.new_node(Type::Primitive(PrimitiveType::Address), None);
        ExpData::Value(node_id, Value::Address(Address::Numerical(value))).into_exp()
    }

    /// Makes a Call expression.
    fn mk_call(&self, ty: &Type, oper: Operation, args: Vec<Exp>) -> Exp {
        let node_id = self.new_node(ty.clone(), None);
        ExpData::Call(node_id, oper, args).into_exp()
    }

    /// Makes a Call expression with type instantiation.
    fn mk_call_with_inst(
        &self,
        ty: &Type,
        inst: Vec<Type>,
        oper: Operation,
        args: Vec<Exp>,
    ) -> Exp {
        let node_id = self.new_node(ty.clone(), Some(inst));
        ExpData::Call(node_id, oper, args).into_exp()
    }

    /// Makes an if-then-else expression.
    fn mk_ite(&self, cond: ExpData, if_true: ExpData, if_false: ExpData) -> Exp {
        let node_id = self.new_node(self.global_env().get_node_type(if_true.node_id()), None);
        ExpData::IfElse(
            node_id,
            cond.into_exp(),
            if_true.into_exp(),
            if_false.into_exp(),
        )
        .into_exp()
    }

    /// Makes a Call expression with boolean result type.
    fn mk_bool_call(&self, oper: Operation, args: Vec<Exp>) -> Exp {
        self.mk_call(&BOOL_TYPE, oper, args)
    }

    /// Make a boolean not expression.
    fn mk_not(&self, arg: Exp) -> Exp {
        // Not(Not(x)) → x
        if let ExpData::Call(_, Operation::Not, inner) = arg.as_ref() {
            if inner.len() == 1 {
                return inner[0].clone();
            }
        }
        self.mk_bool_call(Operation::Not, vec![arg])
    }

    /// Make an old(exp) expression for specs.
    /// Wraps the argument in `old()` to reference the pre-state value.
    fn mk_old(&self, arg: Exp) -> Exp {
        let ty = self.global_env().get_node_type(arg.node_id());
        self.mk_call(&ty, Operation::Old, vec![arg])
    }

    /// Make an equality expression.
    fn mk_eq(&self, arg1: Exp, arg2: Exp) -> Exp {
        self.mk_bool_call(Operation::Eq, vec![arg1, arg2])
    }

    /// Make an identical equality expression. This is stronger than `make_equal` because
    /// it requires the exact same representation, not only interpretation.
    fn mk_identical(&self, arg1: Exp, arg2: Exp) -> Exp {
        self.mk_bool_call(Operation::Identical, vec![arg1, arg2])
    }

    /// Make an and expression.
    fn mk_and(&self, arg1: Exp, arg2: Exp) -> Exp {
        self.mk_bool_call(Operation::And, vec![arg1, arg2])
    }

    /// Make an or expression.
    fn mk_or(&self, arg1: Exp, arg2: Exp) -> Exp {
        self.mk_bool_call(Operation::Or, vec![arg1, arg2])
    }

    /// Check if two boolean expressions are complementary (one is the negation of the other).
    /// Uses `ExpData::structural_eq` for comparison, ignoring `NodeId`s.
    fn is_complementary(&self, a: &Exp, b: &Exp) -> bool {
        crate::exp_simplifier::is_complementary(a, b)
    }

    /// Make an implies expression.
    fn mk_implies(&self, arg1: Exp, arg2: Exp) -> Exp {
        self.mk_bool_call(Operation::Implies, vec![arg1, arg2])
    }

    /// Make an iff expression.
    fn mk_iff(&self, arg1: Exp, arg2: Exp) -> Exp {
        self.mk_bool_call(Operation::Iff, vec![arg1, arg2])
    }

    /// Creates an n-ary conjunction (AND) of the given expressions.
    /// Returns true if the list is empty.
    fn mk_and_n(&self, args: Vec<Exp>) -> Exp {
        if args.is_empty() {
            self.mk_bool_const(true)
        } else {
            args.into_iter().reduce(|a, b| self.mk_and(a, b)).unwrap()
        }
    }

    /// Creates an n-ary disjunction (OR) of the given expressions.
    /// Returns false if the list is empty.
    fn mk_or_n(&self, args: Vec<Exp>) -> Exp {
        if args.is_empty() {
            self.mk_bool_const(false)
        } else {
            args.into_iter().reduce(|a, b| self.mk_or(a, b)).unwrap()
        }
    }

    /// Make a numerical expression for some of the builtin constants.
    /// Produces a `Value::Number` literal instead of a `Call(MaxU*, [])` node,
    /// so the value is immediately available for constant folding and comparison
    /// simplification.
    fn mk_builtin_num_const(&self, oper: Operation) -> Exp {
        let prim = match oper {
            Operation::MaxU8 => PrimitiveType::U8,
            Operation::MaxU16 => PrimitiveType::U16,
            Operation::MaxU32 => PrimitiveType::U32,
            Operation::MaxU64 => PrimitiveType::U64,
            Operation::MaxU128 => PrimitiveType::U128,
            Operation::MaxU256 => PrimitiveType::U256,
            _ => panic!("mk_builtin_num_const: unsupported operation {:?}", oper),
        };
        self.mk_num_const(prim.get_max_value().unwrap())
    }

    /// Join an iterator of boolean expressions with a boolean binary operator.
    fn mk_join_bool(&self, oper: Operation, args: impl Iterator<Item = Exp>) -> Option<Exp> {
        args.reduce(|a, b| self.mk_bool_call(oper.clone(), vec![a, b]))
    }

    /// Join two boolean optional expression with binary operator.
    fn mk_join_opt_bool(
        &self,
        oper: Operation,
        arg1: Option<Exp>,
        arg2: Option<Exp>,
    ) -> Option<Exp> {
        match (arg1, arg2) {
            (Some(a1), Some(a2)) => Some(self.mk_bool_call(oper, vec![a1, a2])),
            (Some(a1), None) => Some(a1),
            (None, Some(a2)) => Some(a2),
            _ => None,
        }
    }

    /// Creates a quantifier over the content of a vector. The passed function `f` receives
    /// an expression representing an element of the vector and returns the quantifiers predicate;
    /// if it returns None, this function will also return None, otherwise the quantifier will be
    /// returned.
    fn mk_vector_quant_opt<F>(
        &self,
        kind: QuantKind,
        vector: Exp,
        elem_ty: &Type,
        f: &mut F,
    ) -> Option<Exp>
    where
        F: FnMut(Exp) -> Option<Exp>,
    {
        let elem = self.mk_local("$elem", elem_ty.clone());
        if let Some(body) = f(elem) {
            let range_decl = self.mk_decl(self.mk_symbol("$elem"), elem_ty.clone());
            let node_id = self.new_node(BOOL_TYPE.clone(), None);
            Some(
                ExpData::Quant(
                    node_id,
                    kind,
                    vec![(range_decl, vector)],
                    vec![],
                    None,
                    body,
                )
                .into_exp(),
            )
        } else {
            None
        }
    }

    /// Creates a quantifier over the content of a map. The passed function `f` receives
    /// an expression representing an element of the map and returns the quantifiers predicate;
    /// if it returns None, this function will also return None, otherwise the quantifier will be
    /// returned.
    ///
    /// NOTE: the `spec_fun_get` argument is needed due to the fact that we do not have a native
    /// operator of getting the value by key in from a map. In the `mk_vector_quant_opt` case,
    /// we have the `Index` operator to map from an index to a value. But in the `map` case, we
    /// need to use the spec function for the key-to-value conversion.
    ///
    /// Alternative design choices including:
    /// - re-purpose the `Index` operator to take key and intrinsic maps
    /// - add a new `Get` operator for this specific operation.
    fn mk_map_quant_opt<F>(
        &self,
        kind: QuantKind,
        map: Exp,
        spec_fun_get: QualifiedId<SpecFunId>,
        key_ty: &Type,
        val_ty: &Type,
        f: &mut F,
    ) -> Option<Exp>
    where
        F: FnMut(Exp) -> Option<Exp>,
    {
        let key = self.mk_local("$key", key_ty.clone());
        let val = self.mk_call_with_inst(
            val_ty,
            vec![key_ty.clone(), val_ty.clone()],
            Operation::SpecFunction(spec_fun_get.module_id, spec_fun_get.id, None),
            vec![map.clone(), key.clone()],
        );
        if let Some(body) = self.mk_join_opt_bool(Operation::And, f(key), f(val)) {
            let range_decl = self.mk_decl(self.mk_symbol("$key"), key_ty.clone());
            let node_id = self.new_node(BOOL_TYPE.clone(), None);
            Some(
                ExpData::Quant(node_id, kind, vec![(range_decl, map)], vec![], None, body)
                    .into_exp(),
            )
        } else {
            None
        }
    }

    /// Creates a quantifier over the content of memory. The passed function `f` receives
    //  an expression representing a value in memory and returns the quantifiers predicate;
    //  if it returns None, this function will also return None.
    fn mk_mem_quant_opt<F>(
        &self,
        kind: QuantKind,
        mem: QualifiedId<StructId>,
        f: &mut F,
    ) -> Option<ExpData>
    where
        F: FnMut(Exp) -> Option<Exp>,
    {
        // We generate `forall $val in resources<R>: INV[$val]`. The `resources<R>`
        // quantifier domain is currently only available in the internal expression language,
        // not on user level.
        let struct_env = self
            .global_env()
            .get_module(mem.module_id)
            .into_struct(mem.id);
        let type_inst = (0..struct_env.get_type_parameters().len())
            .map(Type::new_param)
            .collect_vec();
        let struct_ty = Type::Struct(mem.module_id, mem.id, type_inst);
        let value = self.mk_local("$rsc", struct_ty.clone());

        if let Some(body) = f(value) {
            let resource_domain_ty = Type::ResourceDomain(mem.module_id, mem.id, None);
            let resource_domain_node_id =
                self.new_node(resource_domain_ty, Some(vec![struct_ty.clone()]));
            let resource_domain =
                ExpData::Call(resource_domain_node_id, Operation::ResourceDomain, vec![])
                    .into_exp();
            let resource_decl = self.mk_decl(self.mk_symbol("$rsc"), struct_ty);
            let quant_node_id = self.new_node(BOOL_TYPE.clone(), None);
            Some(ExpData::Quant(
                quant_node_id,
                kind,
                vec![(resource_decl, resource_domain)],
                vec![],
                None,
                body,
            ))
        } else {
            None
        }
    }

    /// Creates a quantifier over the content of instantiated memory. The passed function `f`
    /// receives an expression representing a value in memory and returns the quantifiers predicate;
    //  if it returns None, this function will also return None.
    fn mk_inst_mem_quant_opt<F>(
        &self,
        kind: QuantKind,
        mem: &QualifiedInstId<StructId>,
        f: &mut F,
    ) -> Option<Exp>
    where
        F: FnMut(Exp) -> Option<Exp>,
    {
        // We generate `forall $val in resources<R>: INV[$val]`. The `resources<R>`
        // quantifier domain is currently only available in the internal expression language,
        // not on user level.
        let struct_ty = Type::Struct(mem.module_id, mem.id, mem.inst.clone());
        let value = self.mk_local("$rsc", struct_ty.clone());

        if let Some(body) = f(value) {
            let resource_domain_ty =
                Type::ResourceDomain(mem.module_id, mem.id, Some(mem.inst.clone()));
            let resource_domain_node_id =
                self.new_node(resource_domain_ty, Some(vec![struct_ty.clone()]));
            let resource_domain =
                ExpData::Call(resource_domain_node_id, Operation::ResourceDomain, vec![])
                    .into_exp();
            let resource_decl = self.mk_decl(self.mk_symbol("$rsc"), struct_ty);
            let quant_node_id = self.new_node(BOOL_TYPE.clone(), None);
            Some(
                ExpData::Quant(
                    quant_node_id,
                    kind,
                    vec![(resource_decl, resource_domain)],
                    vec![],
                    None,
                    body,
                )
                .into_exp(),
            )
        } else {
            None
        }
    }

    /// Makes a local variable declaration.
    fn mk_decl(&self, name: Symbol, ty: Type) -> Pattern {
        let node_id = self.new_node(ty, None);
        Pattern::Var(node_id, name)
    }

    /// Makes a symbol from a string.
    fn mk_symbol(&self, str: &str) -> Symbol {
        self.global_env().symbol_pool().make(str)
    }

    /// Makes a type domain expression.
    fn mk_type_domain(&self, ty: Type) -> Exp {
        let domain_ty = Type::TypeDomain(Box::new(ty.clone()));
        let node_id = self.new_node(domain_ty, Some(vec![ty]));
        ExpData::Call(node_id, Operation::TypeDomain, vec![]).into_exp()
    }

    /// Makes an expression which selects a field from a struct.
    fn mk_field_select(&self, field_env: &FieldEnv<'_>, targs: &[Type], exp: Exp) -> Exp {
        let ty = field_env.get_type().instantiate(targs);
        let node_id = self.new_node(ty, None);
        ExpData::Call(
            node_id,
            Operation::Select(
                field_env.struct_env.module_env.get_id(),
                field_env.struct_env.get_id(),
                field_env.get_id(),
            ),
            vec![exp],
        )
        .into_exp()
    }

    /// Makes an expression which updates a field in a struct.
    /// Returns `UpdateField(module_id, struct_id, field_id)(struct_exp, new_value)`
    fn mk_field_update(
        &self,
        field_env: &FieldEnv<'_>,
        targs: &[Type],
        struct_exp: Exp,
        new_value: Exp,
    ) -> Exp {
        let struct_ty = Type::Struct(
            field_env.struct_env.module_env.get_id(),
            field_env.struct_env.get_id(),
            targs.to_vec(),
        );
        let node_id = self.new_node(struct_ty, None);
        ExpData::Call(
            node_id,
            Operation::UpdateField(
                field_env.struct_env.module_env.get_id(),
                field_env.struct_env.get_id(),
                field_env.get_id(),
            ),
            vec![struct_exp, new_value],
        )
        .into_exp()
    }

    /// Makes an expression which tests a variant.
    fn mk_variant_test(&self, struct_env: &StructEnv, variant: Symbol, exp: Exp) -> Exp {
        let node_id = self.new_node(Type::Primitive(PrimitiveType::Bool), None);
        ExpData::Call(
            node_id,
            Operation::TestVariants(struct_env.module_env.get_id(), struct_env.get_id(), vec![
                variant,
            ]),
            vec![exp],
        )
        .into_exp()
    }

    /// Makes an expression which tests whether a value is one of several variants.
    fn mk_variant_tests(&self, struct_env: &StructEnv, variants: &[Symbol], exp: Exp) -> Exp {
        let node_id = self.new_node(Type::Primitive(PrimitiveType::Bool), None);
        ExpData::Call(
            node_id,
            Operation::TestVariants(
                struct_env.module_env.get_id(),
                struct_env.get_id(),
                variants.to_vec(),
            ),
            vec![exp],
        )
        .into_exp()
    }

    /// Makes a Pack expression for a struct (non-variant).
    fn mk_pack(
        &self,
        module_id: crate::model::ModuleId,
        struct_id: StructId,
        type_args: &[Type],
        fields: Vec<Exp>,
    ) -> Exp {
        let struct_ty = Type::Struct(module_id, struct_id, type_args.to_vec());
        let node_id = self.new_node(struct_ty, Some(type_args.to_vec()));
        ExpData::Call(node_id, Operation::Pack(module_id, struct_id, None), fields).into_exp()
    }

    /// Makes a Pack expression for an enum variant.
    fn mk_pack_variant(
        &self,
        module_id: crate::model::ModuleId,
        struct_id: StructId,
        variant: Symbol,
        type_args: &[Type],
        fields: Vec<Exp>,
    ) -> Exp {
        let struct_ty = Type::Struct(module_id, struct_id, type_args.to_vec());
        let node_id = self.new_node(struct_ty, Some(type_args.to_vec()));
        ExpData::Call(
            node_id,
            Operation::Pack(module_id, struct_id, Some(variant)),
            fields,
        )
        .into_exp()
    }

    /// Makes an expression for a temporary.
    fn mk_temporary(&self, temp: TempIndex) -> Exp {
        let ty = self.get_local_type(temp);
        let node_id = self.new_node(ty, None);
        ExpData::Temporary(node_id, temp).into_exp()
    }

    /// Makes a result expression for a function return value at the given index.
    fn mk_result(&self, idx: usize, ty: &Type) -> Exp {
        let node_id = self.new_node(ty.clone(), None);
        ExpData::Call(node_id, Operation::Result(idx), vec![]).into_exp()
    }

    /// Makes an expression for a named local.
    fn mk_local(&self, name: &str, ty: Type) -> Exp {
        let node_id = self.new_node(ty, None);
        let sym = self.mk_symbol(name);
        ExpData::LocalVar(node_id, sym).into_exp()
    }

    /// Makes an expression for a local with an existing symbol.
    fn mk_local_by_sym(&self, sym: Symbol, ty: Type) -> Exp {
        let node_id = self.new_node(ty, None);
        ExpData::LocalVar(node_id, sym).into_exp()
    }

    /// Get's the memory associated with a Call(Global,..) or Call(Exists, ..) node. Crashes
    /// if the node is not typed as expected.
    fn get_memory_of_node(&self, node_id: NodeId) -> QualifiedInstId<StructId> {
        // We do have a call `f<R<..>>` so extract the type from the function instantiation.
        let rty = &self.global_env().get_node_instantiation(node_id)[0];
        let (mid, sid, inst) = rty.require_struct();
        mid.qualified_inst(sid, inst.to_owned())
    }

    /// Create `exists<R>(@address)` expression.
    /// Returns a boolean expression checking if resource R exists at the given address.
    fn mk_exists(&self, struct_env: &StructEnv, type_args: &[Type], addr: Exp) -> Exp {
        self.mk_exists_with_label(struct_env, type_args, addr, None)
    }

    /// Create `exists<R, @label>(@address)` expression with optional memory label.
    /// The label distinguishes pre-state (`Some(label)`) from post-state (`None`).
    fn mk_exists_with_label(
        &self,
        struct_env: &StructEnv,
        type_args: &[Type],
        addr: Exp,
        label: Option<MemoryLabel>,
    ) -> Exp {
        let struct_type = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            type_args.to_vec(),
        );
        let node_id = self.new_node(BOOL_TYPE.clone(), Some(vec![struct_type]));
        ExpData::Call(node_id, Operation::Exists(label), vec![addr]).into_exp()
    }

    /// Create `global<R>(@address)` expression.
    /// Returns the resource value at the given address.
    fn mk_global(&self, struct_env: &StructEnv, type_args: &[Type], addr: Exp) -> Exp {
        self.mk_global_with_label(struct_env, type_args, addr, None)
    }

    /// Create `global<R, @label>(@address)` expression with optional memory label.
    /// The label distinguishes pre-state (`Some(label)`) from post-state (`None`).
    fn mk_global_with_label(
        &self,
        struct_env: &StructEnv,
        type_args: &[Type],
        addr: Exp,
        label: Option<MemoryLabel>,
    ) -> Exp {
        let struct_type = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            type_args.to_vec(),
        );
        let node_id = self.new_node(struct_type.clone(), Some(vec![struct_type]));
        ExpData::Call(node_id, Operation::Global(label), vec![addr]).into_exp()
    }

    // =================================================================================================
    // Numeric Expression Helpers

    /// Make a numeric constant expression with NUM_TYPE (spec arbitrary precision).
    fn mk_num_const(&self, value: BigInt) -> Exp {
        let node_id = self.new_node(NUM_TYPE.clone(), None);
        ExpData::Value(node_id, Value::Number(value)).into_exp()
    }

    /// Make a NUM_TYPE constant for the minimum value of a primitive integer type.
    /// Returns None for non-integer types or unbounded types (Num).
    fn mk_num_min(&self, ty: &PrimitiveType) -> Option<Exp> {
        let value = ty.get_min_value()?;
        Some(self.mk_num_const(value))
    }

    /// Make a NUM_TYPE constant for the maximum value of a primitive integer type.
    /// Returns None for non-integer types or unbounded types (Num).
    fn mk_num_max(&self, ty: &PrimitiveType) -> Option<Exp> {
        let value = ty.get_max_value()?;
        Some(self.mk_num_const(value))
    }

    /// Check if a value is out of range for the given primitive type.
    /// Dispatches on `kind` to emit only the relevant bound check:
    /// - `Overflow` → `value > MAX`
    /// - `Underflow` → `value < MIN`
    /// - `Both` → `value < MIN || value > MAX`
    /// Returns None for non-integer types or unbounded types.
    fn mk_range_check(&self, ty: &PrimitiveType, kind: RangeCheckKind, value: Exp) -> Option<Exp> {
        match kind {
            RangeCheckKind::Overflow => {
                let max = self.mk_num_max(ty)?;
                Some(self.mk_bool_call(Operation::Gt, vec![value, max]))
            },
            RangeCheckKind::Underflow => {
                let min = self.mk_num_min(ty)?;
                Some(self.mk_bool_call(Operation::Lt, vec![value, min]))
            },
            RangeCheckKind::Both => {
                let min = self.mk_num_min(ty)?;
                let max = self.mk_num_max(ty)?;
                let lt_min = self.mk_bool_call(Operation::Lt, vec![value.clone(), min]);
                let gt_max = self.mk_bool_call(Operation::Gt, vec![value, max]);
                Some(self.mk_or(lt_min, gt_max))
            },
        }
    }

    /// Make an addition expression using NUM_TYPE (spec arbitrary precision).
    fn mk_num_add(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Add, vec![e1, e2])
    }

    /// Make a subtraction expression using NUM_TYPE (spec arbitrary precision).
    fn mk_num_sub(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Sub, vec![e1, e2])
    }

    /// Make a multiplication expression using NUM_TYPE (spec arbitrary precision).
    fn mk_num_mul(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Mul, vec![e1, e2])
    }

    /// Make a division expression using NUM_TYPE (spec arbitrary precision).
    fn mk_num_div(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Div, vec![e1, e2])
    }

    /// Make a modulo expression using NUM_TYPE (spec arbitrary precision).
    fn mk_num_mod(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Mod, vec![e1, e2])
    }

    /// Make a bitwise OR expression using NUM_TYPE.
    fn mk_bit_or(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::BitOr, vec![e1, e2])
    }

    /// Make a bitwise AND expression using NUM_TYPE.
    fn mk_bit_and(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::BitAnd, vec![e1, e2])
    }

    /// Make a bitwise XOR expression using NUM_TYPE.
    fn mk_xor(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Xor, vec![e1, e2])
    }

    /// Make a left shift expression using NUM_TYPE.
    fn mk_shl(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Shl, vec![e1, e2])
    }

    /// Make a right shift expression using NUM_TYPE.
    fn mk_shr(&self, e1: Exp, e2: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Shr, vec![e1, e2])
    }

    /// Make a negation expression using NUM_TYPE.
    fn mk_negate(&self, e: Exp) -> Exp {
        self.mk_call(&NUM_TYPE, Operation::Sub, vec![
            self.mk_num_const(BigInt::zero()),
            e,
        ])
    }

    /// Make a cast expression to the target type.
    fn mk_cast(&self, target_ty: Type, e: Exp) -> Exp {
        self.mk_call(&target_ty, Operation::Cast, vec![e])
    }

    // =================================================================================================
    // Function Values

    /// Create a closure expression for a function reference.
    /// Returns the closure expression and the function's result type.
    fn mk_closure(&self, module_id: ModuleId, fun_id: FunId, type_inst: &[Type]) -> (Exp, Type) {
        let fun_env = self
            .global_env()
            .get_module(module_id)
            .into_function(fun_id);
        let param_types: Vec<Type> = fun_env
            .get_parameters()
            .iter()
            .map(|p| p.1.instantiate(type_inst))
            .collect();
        let result_type = fun_env.get_result_type().instantiate(type_inst);
        let fun_type = Type::Fun(
            Box::new(Type::tuple(param_types)),
            Box::new(result_type.clone()),
            AbilitySet::EMPTY,
        );
        let node_id = self.new_node(fun_type, Some(type_inst.to_vec()));
        let exp = ExpData::Call(
            node_id,
            Operation::Closure(module_id, fun_id, ClosureMask::empty()),
            vec![],
        )
        .into_exp();
        (exp, result_type)
    }

    /// Convert a signer expression to an address expression by wrapping it in
    /// `signer::address_of(exp)`. Returns the expression unchanged if it is not
    /// a signer type.
    fn signer_to_address(&self, exp: Exp) -> Exp {
        let env = self.global_env();
        let ty = env.get_node_type(exp.node_id());
        if !ty.skip_reference().is_signer() {
            return exp;
        }
        // Look up `signer::address_of` in the stdlib.
        let signer_sym = env.symbol_pool().make("signer");
        let Some(signer_module) = env.find_module_by_name(signer_sym) else {
            env.error(
                &env.get_node_loc(exp.node_id()),
                "bug: signer module must be available for signer-to-address conversion",
            );
            return exp;
        };
        let address_of_sym = env.symbol_pool().make("address_of");
        let Some(address_of_fun) = signer_module.find_function(address_of_sym) else {
            env.error(
                &env.get_node_loc(exp.node_id()),
                "bug: signer::address_of must be available",
            );
            return exp;
        };
        let mid = signer_module.get_id();
        let fid = address_of_fun.get_id();
        let addr_ty = Type::Primitive(PrimitiveType::Address);
        self.mk_call(&addr_ty, Operation::MoveFunction(mid, fid), vec![exp])
    }

    // =================================================================================================
    // Function Call Behavioral Predicates

    /// Create a result expression for a function call using behavioral predicates.
    /// Create `result_of<f>(args)` expression without state label.
    /// Returns the full result type which may be a tuple for multiple returns.
    fn mk_result_of(&self, fun_exp: Exp, args: Vec<Exp>, result_type: &Type) -> Exp {
        self.mk_result_of_with_label(fun_exp, args, result_type, None)
    }

    /// Create `result_of<f, @label>(args)` expression with optional memory label.
    /// The label specifies which memory state the function's behavior is evaluated at.
    fn mk_result_of_with_label(
        &self,
        fun_exp: Exp,
        args: Vec<Exp>,
        result_type: &Type,
        label: Option<MemoryLabel>,
    ) -> Exp {
        let mut all_args = vec![fun_exp];
        all_args.extend(args);
        let state = BehaviorState::new(None, label);
        self.mk_call(
            result_type,
            Operation::Behavior(BehaviorKind::ResultOf, state),
            all_args,
        )
    }

    /// Create a result expression for a function call, extracting the nth component.
    /// Calls `mk_result_of` and indexes into the tuple if there are multiple returns.
    fn mk_result_of_at(
        &self,
        fun_exp: Exp,
        args: Vec<Exp>,
        result_type: &Type,
        result_index: usize,
        num_results: usize,
    ) -> Exp {
        self.mk_result_of_at_with_label(fun_exp, args, result_type, result_index, num_results, None)
    }

    /// Create a result expression with state label, extracting the nth component.
    fn mk_result_of_at_with_label(
        &self,
        fun_exp: Exp,
        args: Vec<Exp>,
        result_type: &Type,
        result_index: usize,
        num_results: usize,
        label: Option<MemoryLabel>,
    ) -> Exp {
        let result_exp = self.mk_result_of_with_label(fun_exp, args, result_type, label);

        // For multiple returns, extract the nth component
        if num_results > 1 {
            let component_type = if let Type::Tuple(types) = result_type {
                types.get(result_index).cloned().unwrap_or(Type::Error)
            } else {
                result_type.clone()
            };
            let index_exp = self.mk_num_const(BigInt::from(result_index));
            let select_node = self.new_node(component_type, None);
            ExpData::Call(select_node, Operation::Index, vec![result_exp, index_exp]).into_exp()
        } else {
            result_exp
        }
    }

    /// Build aborts_of<f>(args) predicate without state label.
    fn mk_aborts_of(&self, fun_exp: Exp, args: Vec<Exp>) -> Exp {
        self.mk_aborts_of_with_label(fun_exp, args, None)
    }

    /// Build aborts_of<f, @label>(args) predicate with optional memory label.
    /// The label specifies which memory state the abort condition is evaluated at.
    fn mk_aborts_of_with_label(
        &self,
        fun_exp: Exp,
        args: Vec<Exp>,
        label: Option<MemoryLabel>,
    ) -> Exp {
        let mut all_args = vec![fun_exp];
        all_args.extend(args);
        let state = BehaviorState::new(None, label);
        self.mk_bool_call(Operation::Behavior(BehaviorKind::AbortsOf, state), all_args)
    }

    /// Create `result_of<f>(args)` expression with full state (pre and post labels).
    fn mk_result_of_with_state(
        &self,
        fun_exp: Exp,
        args: Vec<Exp>,
        result_type: &Type,
        state: BehaviorState,
    ) -> Exp {
        let mut all_args = vec![fun_exp];
        all_args.extend(args);
        self.mk_call(
            result_type,
            Operation::Behavior(BehaviorKind::ResultOf, state),
            all_args,
        )
    }

    /// Create a result expression with full state, extracting the nth component.
    /// For multiple results, generates `{ let (_t0, _t1, ...) = result_of<f>(args); _ti }`
    /// using let-binding deconstruction since tuple index select is not available in specs.
    fn mk_result_of_at_with_state(
        &self,
        fun_exp: Exp,
        args: Vec<Exp>,
        result_type: &Type,
        result_index: usize,
        num_results: usize,
        state: BehaviorState,
    ) -> Exp {
        let result_exp = self.mk_result_of_with_state(fun_exp, args, result_type, state);

        // For multiple returns, extract the nth component via let-binding deconstruction
        if num_results > 1 {
            let elem_types: Vec<Type> = if let Type::Tuple(types) = result_type {
                types.clone()
            } else {
                vec![result_type.clone(); num_results]
            };
            // Build pattern elements and find the symbol for the desired index
            let mut pat_elems = Vec::with_capacity(num_results);
            let mut selected_sym = None;
            let mut selected_ty = Type::Error;
            for (i, ty) in elem_types.iter().enumerate() {
                let sym = self.mk_symbol(&format!("_t{}", i));
                let pat = self.mk_decl(sym, ty.clone());
                pat_elems.push(pat);
                if i == result_index {
                    selected_sym = Some(sym);
                    selected_ty = ty.clone();
                }
            }
            let Some(sym) = selected_sym else {
                // Keep translation recoverable if result metadata is inconsistent.
                let index_exp = self.mk_num_const(BigInt::from(result_index));
                let select_node = self.new_node(Type::Error, None);
                return ExpData::Call(select_node, Operation::Index, vec![result_exp, index_exp])
                    .into_exp();
            };
            let tuple_pat_node = self.new_node(result_type.clone(), None);
            let pattern = Pattern::Tuple(tuple_pat_node, pat_elems);
            let body = self.mk_local_by_sym(sym, selected_ty.clone());
            let block_node = self.new_node(selected_ty, None);
            ExpData::Block(block_node, pattern, Some(result_exp), body).into_exp()
        } else {
            result_exp
        }
    }

    /// Build aborts_of<f>(args) predicate with full state (pre and post labels).
    fn mk_aborts_of_with_state(&self, fun_exp: Exp, args: Vec<Exp>, state: BehaviorState) -> Exp {
        let mut all_args = vec![fun_exp];
        all_args.extend(args);
        self.mk_bool_call(Operation::Behavior(BehaviorKind::AbortsOf, state), all_args)
    }

    /// Build ensures_of<f>(args) predicate with full state (pre and post labels).
    /// Used for void-returning calls to capture the callee's post-conditions
    /// and define the post-label for state chaining.
    fn mk_ensures_of_with_state(&self, fun_exp: Exp, args: Vec<Exp>, state: BehaviorState) -> Exp {
        let mut all_args = vec![fun_exp];
        all_args.extend(args);
        self.mk_bool_call(
            Operation::Behavior(BehaviorKind::EnsuresOf, state),
            all_args,
        )
    }
}

// =================================================================================================
// FunExpGenerator

/// Lightweight `ExpGenerator` wrapping a `FunctionEnv` and a `Loc`.
/// Use when you need an `ExpGenerator` but don't have a `FunctionDataBuilder`.
pub struct FunExpGenerator<'env> {
    fun_env: FunctionEnv<'env>,
    loc: Loc,
}

impl<'env> FunExpGenerator<'env> {
    pub fn new(fun_env: FunctionEnv<'env>, loc: Loc) -> Self {
        Self { fun_env, loc }
    }
}

impl<'env> ExpGenerator<'env> for FunExpGenerator<'env> {
    fn function_env(&self) -> &FunctionEnv<'env> {
        &self.fun_env
    }

    fn get_current_loc(&self) -> Loc {
        self.loc.clone()
    }

    fn set_loc(&mut self, loc: Loc) {
        self.loc = loc;
    }

    fn add_local(&mut self, _: Type) -> TempIndex {
        panic!("FunExpGenerator does not support add_local")
    }

    fn get_local_type(&self, temp: TempIndex) -> Type {
        self.fun_env
            .get_local_type(temp)
            .unwrap_or_else(|| panic!("undefined temp {}", temp))
    }
}
