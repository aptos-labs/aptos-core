// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    expansion::translate::is_valid_struct_constant_or_schema_name,
    parser::ast::{
        self as P, Ability, Ability_, BinOp, CallKind, ConstantName, Field, FunctionName, Label,
        LambdaCaptureKind, ModuleName, QuantKind, SpecApplyPattern, StructName, UnaryOp, UseDecl,
        Var, VariantName, ENTRY_MODIFIER,
    },
    shared::{
        ast_debug::*,
        known_attributes::{AttributeKind, KnownAttribute},
        unique_map::UniqueMap,
        unique_set::UniqueSet,
        *,
    },
};
use move_binary_format::file_format;
use move_ir_types::location::*;
use move_symbol_pool::Symbol;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
    hash::Hash,
};
//**************************************************************************************************
// Program
//**************************************************************************************************

#[derive(Debug, Clone)]
pub struct Program {
    // Map of declared named addresses, and their values if specified
    pub modules: UniqueMap<ModuleIdent, ModuleDefinition>,
    pub scripts: BTreeMap<Symbol, Script>,
}

//**************************************************************************************************
// Attributes
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum AttributeValue_ {
    Value(Value),
    Module(ModuleIdent),
    ModuleAccess(ModuleAccess),
}
pub type AttributeValue = Spanned<AttributeValue_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attribute_ {
    Name(Name),
    Assigned(Name, Box<AttributeValue>),
    Parameterized(Name, Attributes),
}
pub type Attribute = Spanned<Attribute_>;

impl Attribute_ {
    pub fn attribute_name(&self) -> &Name {
        match self {
            Attribute_::Name(nm)
            | Attribute_::Assigned(nm, _)
            | Attribute_::Parameterized(nm, _) => nm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AttributeName_ {
    Unknown(Symbol),
    Known(KnownAttribute),
}
pub type AttributeName = Spanned<AttributeName_>;

pub type Attributes = UniqueMap<AttributeName, Attribute>;

//**************************************************************************************************
// Scripts
//**************************************************************************************************

#[derive(Debug, Clone)]
pub struct Script {
    // package name metadata from compiler arguments, not used for any language rules
    pub package_name: Option<Symbol>,
    pub attributes: Attributes,
    pub loc: Loc,
    pub immediate_neighbors: UniqueMap<ModuleIdent, Neighbor>,
    pub used_addresses: BTreeSet<Address>,
    pub constants: UniqueMap<ConstantName, Constant>,
    pub function_name: FunctionName,
    pub function: Function,
    pub specs: Vec<SpecBlock>,
    pub use_decls: Vec<UseDecl>,
}

//**************************************************************************************************
// Modules
//**************************************************************************************************

#[derive(Clone, Copy)]
pub enum Address {
    Numerical(Option<Name>, Spanned<NumericalAddress>),
    NamedUnassigned(Name),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleIdent_ {
    pub address: Address,
    pub module: ModuleName,
}
pub type ModuleIdent = Spanned<ModuleIdent_>;

#[derive(Debug, Clone)]
pub struct ModuleDefinition {
    // package name metadata from compiler arguments, not used for any language rules
    pub package_name: Option<Symbol>,
    pub attributes: Attributes,
    pub loc: Loc,
    pub is_source_module: bool,
    /// `dependency_order` is the topological order/rank in the dependency graph.
    /// `dependency_order` is initialized at `0` and set in the uses pass
    pub dependency_order: usize,
    pub immediate_neighbors: UniqueMap<ModuleIdent, Neighbor>,
    pub used_addresses: BTreeSet<Address>,
    pub friends: UniqueMap<ModuleIdent, Friend>,
    pub structs: UniqueMap<StructName, StructDefinition>,
    pub functions: UniqueMap<FunctionName, Function>,
    pub constants: UniqueMap<ConstantName, Constant>,
    pub specs: Vec<SpecBlock>,
    pub use_decls: Vec<UseDecl>,
}

//**************************************************************************************************
// Friend
//**************************************************************************************************

#[derive(Debug, Clone)]
pub struct Friend {
    pub attributes: Attributes,
    pub loc: Loc,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Neighbor {
    Dependency,
    Friend,
}

//**************************************************************************************************
// Structs
//**************************************************************************************************

pub type Fields<T> = UniqueMap<Field, (usize, T)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructTypeParameter {
    pub is_phantom: bool,
    pub name: Name,
    pub constraints: AbilitySet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub attributes: Attributes,
    pub loc: Loc,
    pub abilities: AbilitySet,
    pub type_parameters: Vec<StructTypeParameter>,
    pub layout: StructLayout,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructLayout {
    // the second field is true iff the struct has positional fields
    Singleton(Fields<Type>, bool),
    Variants(Vec<StructVariant>),
    Native(Loc),
}

#[derive(Debug, PartialEq, Clone)]
pub struct StructVariant {
    pub attributes: Attributes,
    pub loc: Loc,
    pub name: VariantName,
    pub fields: Fields<Type>,
    pub is_positional: bool,
}

//**************************************************************************************************
// Functions
//**************************************************************************************************

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Visibility {
    Public(Loc),
    Friend(Loc),
    Package(Loc),
    Internal,
}

#[derive(PartialEq, Clone, Debug)]
pub struct FunctionSignature {
    pub type_parameters: Vec<(Name, AbilitySet)>,
    pub parameters: Vec<(Var, Type)>,
    pub return_type: Type,
}

#[derive(PartialEq, Clone, Debug)]
pub enum FunctionBody_ {
    Defined(Sequence),
    Native,
}
pub type FunctionBody = Spanned<FunctionBody_>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpecId(usize);

#[derive(PartialEq, Clone, Debug)]
pub struct Function {
    pub attributes: Attributes,
    pub loc: Loc,
    pub inline: bool,
    pub visibility: Visibility,
    pub entry: Option<Loc>,
    pub signature: FunctionSignature,
    pub acquires: Vec<ModuleAccess>,
    // Only v2 compiler
    pub access_specifiers: Option<Vec<AccessSpecifier>>,
    pub body: FunctionBody,
    pub specs: BTreeMap<SpecId, SpecBlock>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct AccessSpecifier_ {
    pub kind: file_format::AccessKind,
    pub negated: bool,
    pub module_address: Option<Address>,
    pub module_name: Option<ModuleName>,
    pub resource_name: Option<Name>,
    pub type_args: Option<Vec<Type>>,
    pub address: AddressSpecifier,
}

pub type AccessSpecifier = Spanned<AccessSpecifier_>;

#[derive(PartialEq, Clone, Debug)]
pub enum AddressSpecifier_ {
    Any,
    Empty,
    Literal(NumericalAddress),
    Name(Name),
    Call(ModuleAccess, Option<Vec<Type>>, Name),
}

pub type AddressSpecifier = Spanned<AddressSpecifier_>;

//**************************************************************************************************
// Constants
//**************************************************************************************************

#[derive(PartialEq, Clone, Debug)]
pub struct Constant {
    pub attributes: Attributes,
    pub loc: Loc,
    pub signature: Type,
    pub value: Exp,
}

//**************************************************************************************************
// Specification Blocks
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq)]
pub struct SpecBlock_ {
    pub attributes: Attributes,
    pub target: SpecBlockTarget,
    pub members: Vec<SpecBlockMember>,
}
pub type SpecBlock = Spanned<SpecBlock_>;

#[derive(Debug, Clone, PartialEq)]
pub enum SpecBlockTarget_ {
    Code,
    Module,
    Member(Name, Option<Box<FunctionSignature>>),
    Schema(Name, Vec<(Name, AbilitySet)>),
}

pub type SpecBlockTarget = Spanned<SpecBlockTarget_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum SpecBlockMember_ {
    Condition {
        kind: SpecConditionKind,
        properties: Vec<PragmaProperty>,
        exp: Exp,
        additional_exps: Vec<Exp>,
    },
    Function {
        uninterpreted: bool,
        name: FunctionName,
        signature: FunctionSignature,
        body: FunctionBody,
    },
    Variable {
        is_global: bool,
        name: Name,
        type_parameters: Vec<(Name, AbilitySet)>,
        type_: Type,
        init: Option<Exp>,
    },
    Update {
        lhs: Exp,
        rhs: Exp,
    },
    Let {
        name: Name,
        post_state: bool,
        def: Exp,
    },
    Include {
        properties: Vec<PragmaProperty>,
        exp: Exp,
    },
    Apply {
        exp: Exp,
        patterns: Vec<SpecApplyPattern>,
        exclusion_patterns: Vec<SpecApplyPattern>,
    },
    Pragma {
        properties: Vec<PragmaProperty>,
    },
}
pub type SpecBlockMember = Spanned<SpecBlockMember_>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SpecConditionKind_ {
    Assert,
    Assume,
    Decreases,
    AbortsIf,
    AbortsWith,
    SucceedsIf,
    Modifies,
    Emits,
    Ensures,
    Requires,
    Invariant(Vec<(Name, AbilitySet)>),
    InvariantUpdate(Vec<(Name, AbilitySet)>),
    Axiom(Vec<(Name, AbilitySet)>),
}
pub type SpecConditionKind = Spanned<SpecConditionKind_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PragmaProperty_ {
    pub name: Name,
    pub value: Option<PragmaValue>,
}
pub type PragmaProperty = Spanned<PragmaProperty_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PragmaValue {
    Literal(Value),
    Ident(ModuleAccess),
}

//**************************************************************************************************
// Types
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbilitySet(UniqueSet<Ability>);

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum ModuleAccess_ {
    Name(Name),
    // ModuleAccess(module_ident, member_ident, optional_variant_ident)
    ModuleAccess(ModuleIdent, Name, Option<Name>),
}

impl ModuleAccess_ {
    pub fn get_name(&self) -> &Name {
        match self {
            ModuleAccess_::Name(n) | ModuleAccess_::ModuleAccess(_, n, _) => n,
        }
    }

    pub fn is_valid_struct_constant_or_schema_name(&self) -> bool {
        is_valid_struct_constant_or_schema_name(self.get_name().value.as_str())
    }
}

pub type ModuleAccess = Spanned<ModuleAccess_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Type_ {
    Unit,
    Multiple(Vec<Type>),
    Apply(ModuleAccess, Vec<Type>),
    Ref(bool, Box<Type>),
    Fun(Vec<Type>, Box<Type>, AbilitySet),
    UnresolvedError,
}
pub type Type = Spanned<Type_>;

//**************************************************************************************************
// Expressions
//**************************************************************************************************

#[derive(Debug, Clone, PartialEq)]
pub enum LValue_ {
    Var(ModuleAccess, Option<Vec<Type>>),
    Unpack(
        ModuleAccess,
        Option<Vec<Type>>,
        Fields<LValue>,
        Option<DotDot>,
    ),
    PositionalUnpack(ModuleAccess, Option<Vec<Type>>, LValueOrDotDotList),
}
pub type LValue = Spanned<LValue_>;
pub type LValueList_ = Vec<LValue>;
pub type LValueList = Spanned<LValueList_>;

/// These represent LValues with user-specified explicit types.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedLValue_(pub LValue, pub Option<Type>);
pub type TypedLValue = Spanned<TypedLValue_>;
pub type TypedLValueList_ = Vec<TypedLValue>;
pub type TypedLValueList = Spanned<TypedLValueList_>;

pub fn wild_card(loc: Loc) -> LValue {
    let wildcard = sp(loc, Symbol::from("_"));
    let lvalue_ = LValue_::Var(sp(loc, ModuleAccess_::Name(wildcard)), None);
    sp(loc, lvalue_)
}

#[derive(Debug, Clone, PartialEq)]
pub struct DotDot_;
pub type DotDot = Spanned<DotDot_>;

#[derive(Debug, Clone, PartialEq)]
pub enum LValueOrDotDot_ {
    LValue(LValue),
    DotDot,
}
pub type LValueOrDotDot = Spanned<LValueOrDotDot_>;
pub type LValueOrDotDotList_ = Vec<LValueOrDotDot>;
pub type LValueOrDotDotList = Spanned<LValueOrDotDotList_>;

pub type LValueWithRange_ = (LValue, Exp);
pub type LValueWithRange = Spanned<LValueWithRange_>;
pub type LValueWithRangeList_ = Vec<LValueWithRange>;
pub type LValueWithRangeList = Spanned<LValueWithRangeList_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum ExpDotted_ {
    Exp(Exp),
    Dot(Box<ExpDotted>, Name),
}
pub type ExpDotted = Spanned<ExpDotted_>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value_ {
    // 0x<hex representation up to 64 digits with padding 0s>
    Address(Address),
    // <num>
    InferredNum(move_core_types::u256::U256),
    // <num>u8
    U8(u8),
    // <num>u16
    U16(u16),
    // <num>u32
    U32(u32),
    // <num>u64
    U64(u64),
    // <num>u128
    U128(u128),
    // <num>u256
    U256(move_core_types::u256::U256),
    // true
    // false
    Bool(bool),
    Bytearray(Vec<u8>),
}
pub type Value = Spanned<Value_>;

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Exp_ {
    Value(Value),
    Move(Var),
    Copy(Var),

    Name(ModuleAccess, Option<Vec<Type>>),
    Call(ModuleAccess, CallKind, Option<Vec<Type>>, Spanned<Vec<Exp>>),
    ExpCall(Box<Exp>, Spanned<Vec<Exp>>),
    Pack(ModuleAccess, Option<Vec<Type>>, Fields<Exp>),
    Vector(Loc, Option<Vec<Type>>, Spanned<Vec<Exp>>),

    IfElse(Box<Exp>, Box<Exp>, Box<Exp>),
    Match(Box<Exp>, Vec<Spanned<(LValueList, Option<Exp>, Exp)>>),
    While(Option<Label>, Box<Exp>, Box<Exp>),
    Loop(Option<Label>, Box<Exp>),
    Block(Sequence),
    Lambda(TypedLValueList, Box<Exp>, LambdaCaptureKind, AbilitySet),
    Quant(
        QuantKind,
        LValueWithRangeList,
        Vec<Vec<Exp>>,
        Option<Box<Exp>>,
        Box<Exp>,
    ), // spec only

    Assign(LValueList, Box<Exp>),
    FieldMutate(Box<ExpDotted>, Box<Exp>),
    Mutate(Box<Exp>, Box<Exp>),

    Return(Box<Exp>),
    Abort(Box<Exp>),
    Break(Option<Label>),
    Continue(Option<Label>),

    Dereference(Box<Exp>),
    UnaryExp(UnaryOp, Box<Exp>),
    BinopExp(Box<Exp>, BinOp, Box<Exp>),

    ExpList(Vec<Exp>),
    Unit {
        trailing: bool,
    },

    Borrow(bool, Box<Exp>),
    ExpDotted(Box<ExpDotted>),
    Index(Box<Exp>, Box<Exp>), // spec only unless language >= v2

    Cast(Box<Exp>, Type),
    Annotate(Box<Exp>, Type),
    Test(Box<Exp>, Vec<Type>),

    Spec(SpecId, BTreeSet<Name>, BTreeSet<Name>),

    UnresolvedError,
}
pub type Exp = Spanned<Exp_>;

pub type Sequence = VecDeque<SequenceItem>;
#[derive(Debug, Clone, PartialEq)]
pub enum SequenceItem_ {
    Seq(Exp),
    Declare(LValueList, Option<Type>),
    Bind(LValueList, Exp),
}
pub type SequenceItem = Spanned<SequenceItem_>;

//**************************************************************************************************
// Traits
//**************************************************************************************************

impl TName for ModuleIdent {
    type Key = ModuleIdent_;
    type Loc = Loc;

    fn drop_loc(self) -> (Loc, ModuleIdent_) {
        (self.loc, self.value)
    }

    fn add_loc(loc: Loc, value: ModuleIdent_) -> ModuleIdent {
        sp(loc, value)
    }

    fn borrow(&self) -> (&Loc, &ModuleIdent_) {
        (&self.loc, &self.value)
    }
}

impl TName for AttributeName {
    type Key = AttributeName_;
    type Loc = Loc;

    fn drop_loc(self) -> (Self::Loc, Self::Key) {
        let sp!(loc, n_) = self;
        (loc, n_)
    }

    fn add_loc(loc: Self::Loc, name_: Self::Key) -> Self {
        sp(loc, name_)
    }

    fn borrow(&self) -> (&Self::Loc, &Self::Key) {
        let sp!(loc, n_) = self;
        (loc, n_)
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Numerical(_, l), Self::Numerical(_, r)) => l == r,
            (Self::NamedUnassigned(l), Self::NamedUnassigned(r)) => l == r,
            _ => false,
        }
    }
}

impl Eq for Address {}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Address {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self, other) {
            (Self::Numerical(_, _), Self::NamedUnassigned(_)) => Ordering::Less,
            (Self::NamedUnassigned(_), Self::Numerical(_, _)) => Ordering::Greater,

            (Self::Numerical(_, l), Self::Numerical(_, r)) => l.cmp(r),
            (Self::NamedUnassigned(l), Self::NamedUnassigned(r)) => l.cmp(r),
        }
    }
}

impl Hash for Address {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Numerical(_, sp!(_, bytes)) => bytes.hash(state),
            Self::NamedUnassigned(name) => name.hash(state),
        }
    }
}

//**************************************************************************************************
// impls
//**************************************************************************************************

impl Address {
    pub const fn anonymous(loc: Loc, address: NumericalAddress) -> Self {
        Self::Numerical(None, sp(loc, address))
    }

    pub fn into_addr_bytes(self) -> NumericalAddress {
        match self {
            Self::Numerical(_, sp!(_, bytes)) => bytes,
            Self::NamedUnassigned(_) => NumericalAddress::DEFAULT_ERROR_ADDRESS,
        }
    }
}

impl ModuleIdent_ {
    pub fn new(address: Address, module: ModuleName) -> Self {
        Self { address, module }
    }
}

impl SpecId {
    pub fn new(u: usize) -> Self {
        SpecId(u)
    }

    pub fn inner(self) -> usize {
        self.0
    }
}

impl AbilitySet {
    /// All abilities
    pub const ALL: [Ability_; 4] = [
        Ability_::Copy,
        Ability_::Drop,
        Ability_::Store,
        Ability_::Key,
    ];
    /// Abilities for vector<_>, note they are predicated on the type argument
    pub const COLLECTION: [Ability_; 3] = [Ability_::Copy, Ability_::Drop, Ability_::Store];
    /// Abilities for bool, u8, u16, u32, u64, u128, u256 and address
    pub const PRIMITIVES: [Ability_; 3] = [Ability_::Copy, Ability_::Drop, Ability_::Store];
    /// Abilities for &_ and &mut _
    pub const REFERENCES: [Ability_; 2] = [Ability_::Copy, Ability_::Drop];
    /// Abilities for signer
    pub const SIGNER: [Ability_; 1] = [Ability_::Drop];

    pub fn empty() -> Self {
        AbilitySet(UniqueSet::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn add(&mut self, a: Ability) -> Result<(), Loc> {
        self.0.add(a).map_err(|(_a, loc)| loc)
    }

    pub fn has_ability(&self, a: &Ability) -> bool {
        self.0.contains(a)
    }

    pub fn has_ability_(&self, a: Ability_) -> bool {
        self.0.contains_(&a)
    }

    // intersection of two sets. Keeps the loc of the first set
    pub fn intersect(&self, other: &Self) -> Self {
        Self(self.0.intersect(&other.0))
    }

    // union of two sets. Prefers the loc of the first set
    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.union(&other.0))
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        self.0.is_subset(&other.0)
    }

    pub fn iter(&self) -> AbilitySetIter {
        self.into_iter()
    }

    pub fn from_abilities(
        iter: impl IntoIterator<Item = Ability>,
    ) -> Result<Self, (Ability_, Loc, Loc)> {
        Ok(Self(UniqueSet::from_elements(iter)?))
    }

    pub fn from_abilities_(
        loc: Loc,
        iter: impl IntoIterator<Item = Ability_>,
    ) -> Result<Self, (Ability_, Loc, Loc)> {
        Ok(Self(UniqueSet::from_elements_(loc, iter)?))
    }

    pub fn all(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::ALL.to_vec()).unwrap()
    }

    pub fn primitives(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::PRIMITIVES.to_vec()).unwrap()
    }

    pub fn references(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::REFERENCES.to_vec()).unwrap()
    }

    pub fn signer(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::SIGNER.to_vec()).unwrap()
    }

    pub fn collection(loc: Loc) -> Self {
        Self::from_abilities_(loc, Self::COLLECTION.to_vec()).unwrap()
    }

    pub fn functions() -> Self {
        Self::empty()
    }
}

impl Visibility {
    pub const FRIEND: &'static str = P::Visibility::FRIEND;
    pub const INTERNAL: &'static str = P::Visibility::INTERNAL;
    pub const PACKAGE: &'static str = P::Visibility::PACKAGE;
    pub const PUBLIC: &'static str = P::Visibility::PUBLIC;

    pub fn loc(&self) -> Option<Loc> {
        match self {
            Visibility::Public(loc) | Visibility::Friend(loc) | Visibility::Package(loc) => {
                Some(*loc)
            },
            Visibility::Internal => None,
        }
    }
}

//**************************************************************************************************
// Iter
//**************************************************************************************************

pub struct AbilitySetIter<'a>(unique_set::Iter<'a, Ability>);

impl<'a> Iterator for AbilitySetIter<'a> {
    type Item = Ability;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(loc, a_)| sp(loc, *a_))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> IntoIterator for &'a AbilitySet {
    type IntoIter = AbilitySetIter<'a>;
    type Item = Ability;

    fn into_iter(self) -> Self::IntoIter {
        AbilitySetIter(self.0.iter())
    }
}

pub struct AbilitySetIntoIter(unique_set::IntoIter<Ability>);

impl Iterator for AbilitySetIntoIter {
    type Item = Ability;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl IntoIterator for AbilitySet {
    type IntoIter = AbilitySetIntoIter;
    type Item = Ability;

    fn into_iter(self) -> Self::IntoIter {
        AbilitySetIntoIter(self.0.into_iter())
    }
}

//**************************************************************************************************
// Display
//**************************************************************************************************

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Numerical(None, sp!(_, bytes)) => write!(f, "{}", bytes),
            Self::Numerical(Some(name), sp!(_, bytes)) => write!(f, "({}={})", name, bytes),
            Self::NamedUnassigned(name) => write!(f, "{}", name),
        }
    }
}

impl fmt::Display for Neighbor {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match self {
            Neighbor::Dependency => write!(f, "neighbor#dependency"),
            Neighbor::Friend => write!(f, "neighbor#friend"),
        }
    }
}

impl fmt::Display for AttributeName_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        match self {
            AttributeName_::Unknown(sym) => write!(f, "{}", sym),
            AttributeName_::Known(known) => write!(f, "{}", known.name()),
        }
    }
}

impl fmt::Display for ModuleIdent_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}::{}", self.address, &self.module)
    }
}

impl fmt::Display for ModuleAccess_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        use ModuleAccess_::*;
        match self {
            Name(n) => write!(f, "{}", n),
            ModuleAccess(m, n, opt_v) => write!(
                f,
                "{}::{}{}",
                m,
                n,
                if let Some(v) = opt_v {
                    format!("::{}", v)
                } else {
                    "".to_string()
                }
            ),
        }
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", match &self {
            Visibility::Public(_) => Visibility::PUBLIC,
            Visibility::Package(_) => Visibility::PACKAGE,
            Visibility::Friend(_) => Visibility::FRIEND,
            Visibility::Internal => Visibility::INTERNAL,
        })
    }
}

impl fmt::Display for AbilitySet {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        if !self.is_empty() {
            write!(f, ": ")?;
            write!(f, "{}", format_delim(self, "+"))
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for Type_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        use Type_::*;
        match self {
            UnresolvedError => write!(f, "_"),
            Apply(n, tys) => {
                write!(f, "{}", n)?;
                if !tys.is_empty() {
                    write!(f, "<{}>", format_comma(tys))
                } else {
                    Ok(())
                }
            },
            Ref(mut_, ty) => write!(f, "&{}{}", if *mut_ { "mut " } else { "" }, ty),
            Fun(args, result, abilities) => {
                write!(f, "({}):{}{}", format_comma(args), result, abilities)
            },
            Unit => write!(f, "()"),
            Multiple(tys) => {
                write!(f, "({})", format_comma(tys))
            },
        }
    }
}

impl fmt::Display for SpecId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

//**************************************************************************************************
// Debug
//**************************************************************************************************

impl AstDebug for Program {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Program { modules, scripts } = self;
        for (m, mdef) in modules.key_cloned_iter() {
            w.write(&format!("module {}", m));
            w.block(|w| mdef.ast_debug(w));
            w.new_line();
        }

        for (n, s) in scripts {
            w.write(&format!("script {}", n));
            w.block(|w| s.ast_debug(w));
            w.new_line()
        }
    }
}

impl AstDebug for AttributeValue_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            AttributeValue_::Value(v) => v.ast_debug(w),
            AttributeValue_::Module(m) => w.write(&format!("{}", m)),
            AttributeValue_::ModuleAccess(n) => n.ast_debug(w),
        }
    }
}

impl AstDebug for Attribute_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            Attribute_::Name(n) => w.write(&format!("{}", n)),
            Attribute_::Assigned(n, v) => {
                w.write(&format!("{}", n));
                w.write(" = ");
                v.ast_debug(w);
            },
            Attribute_::Parameterized(n, inners) => {
                w.write(&format!("{}", n));
                w.write("(");
                w.list(inners, ", ", |w, (_, _, inner)| {
                    inner.ast_debug(w);
                    false
                });
                w.write(")");
            },
        }
    }
}

impl AstDebug for Attributes {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write("#[");
        w.list(self, ", ", |w, (_, _, attr)| {
            attr.ast_debug(w);
            false
        });
        w.write("]");
    }
}

impl AstDebug for Script {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Script {
            package_name,
            attributes,
            loc: _loc,
            immediate_neighbors,
            used_addresses,
            constants,
            function_name,
            function,
            specs,
            use_decls: _,
        } = self;
        if let Some(n) = package_name {
            w.writeln(&format!("{}", n))
        }
        attributes.ast_debug(w);
        for (mident, neighbor) in immediate_neighbors.key_cloned_iter() {
            w.write(&format!("{} {};", neighbor, mident));
            w.new_line();
        }
        for addr in used_addresses {
            w.write(&format!("uses address {};", addr));
            w.new_line()
        }
        for cdef in constants.key_cloned_iter() {
            cdef.ast_debug(w);
            w.new_line();
        }
        (*function_name, function).ast_debug(w);
        for spec in specs {
            spec.ast_debug(w);
            w.new_line();
        }
    }
}

impl AstDebug for ModuleDefinition {
    fn ast_debug(&self, w: &mut AstWriter) {
        let ModuleDefinition {
            package_name,
            attributes,
            loc: _loc,
            is_source_module,
            dependency_order,
            immediate_neighbors,
            used_addresses,
            friends,
            structs,
            functions,
            constants,
            specs,
            use_decls: _,
        } = self;
        if let Some(n) = package_name {
            w.writeln(&format!("{}", n))
        }
        attributes.ast_debug(w);
        w.writeln(
            if *is_source_module {
                "source module"
            } else {
                "library module"
            },
        );
        w.writeln(&format!("dependency order #{}", dependency_order));
        for (mident, neighbor) in immediate_neighbors.key_cloned_iter() {
            w.write(&format!("{} {};", neighbor, mident));
            w.new_line();
        }
        for addr in used_addresses {
            w.write(&format!("uses address {};", addr));
            w.new_line()
        }
        for (mident, _loc) in friends.key_cloned_iter() {
            w.write(&format!("friend {};", mident));
            w.new_line();
        }
        for sdef in structs.key_cloned_iter() {
            sdef.ast_debug(w);
            w.new_line();
        }
        for cdef in constants.key_cloned_iter() {
            cdef.ast_debug(w);
            w.new_line();
        }
        for fdef in functions.key_cloned_iter() {
            fdef.ast_debug(w);
            w.new_line();
        }
        for spec in specs {
            spec.ast_debug(w);
            w.new_line();
        }
    }
}

pub fn ability_modifiers_ast_debug(w: &mut AstWriter, abilities: &AbilitySet) {
    if !abilities.is_empty() {
        w.write(" has ");
        w.list(abilities, " ", |w, ab| {
            ab.ast_debug(w);
            false
        });
    }
}

impl AstDebug for (StructName, &StructDefinition) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (
            name,
            StructDefinition {
                attributes,
                loc: _loc,
                abilities,
                type_parameters,
                layout: fields,
            },
        ) = self;

        attributes.ast_debug(w);

        if let StructLayout::Native(_) = fields {
            w.write("native ");
        }

        w.write(&format!("struct {}", name));
        type_parameters.ast_debug(w);
        ability_modifiers_ast_debug(w, abilities);
        if let StructLayout::Singleton(fields, _) = fields {
            w.block(|w| {
                w.list(fields, ",", |w, (_, f, idx_st)| {
                    let (idx, st) = idx_st;
                    w.write(&format!("{}#{}: ", idx, f));
                    st.ast_debug(w);
                    true
                });
            })
        }
    }
}

impl AstDebug for SpecBlock_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(" spec ");
        self.target.ast_debug(w);
        w.write("{");
        w.semicolon(&self.members, |w, m| m.ast_debug(w));
        w.write("}");
    }
}

impl AstDebug for SpecBlockTarget_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            SpecBlockTarget_::Code => {},
            SpecBlockTarget_::Module => w.write("module "),
            SpecBlockTarget_::Member(name, sign_opt) => {
                w.write(name.value);
                if let Some(sign) = sign_opt {
                    sign.ast_debug(w);
                }
            },
            SpecBlockTarget_::Schema(n, tys) => {
                w.write(&format!("schema {}", n.value));
                if !tys.is_empty() {
                    w.write("<");
                    w.list(tys, ", ", |w, ty| {
                        ty.ast_debug(w);
                        true
                    });
                    w.write(">");
                }
            },
        }
    }
}

impl AstDebug for SpecConditionKind_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use SpecConditionKind_::*;
        match self {
            Assert => w.write("assert "),
            Assume => w.write("assume "),
            Decreases => w.write("decreases "),
            AbortsIf => w.write("aborts_if "),
            AbortsWith => w.write("aborts_with "),
            SucceedsIf => w.write("succeeds_if "),
            Modifies => w.write("modifies "),
            Emits => w.write("emits "),
            Ensures => w.write("ensures "),
            Requires => w.write("requires "),
            Invariant(ty_params) => {
                w.write("invariant");
                ty_params.ast_debug(w);
                w.write(" ")
            },
            InvariantUpdate(ty_params) => {
                w.write("invariant");
                ty_params.ast_debug(w);
                w.write(" update ")
            },
            Axiom(ty_params) => {
                w.write("axiom");
                ty_params.ast_debug(w);
                w.write(" ")
            },
        }
    }
}

impl AstDebug for SpecBlockMember_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            SpecBlockMember_::Condition {
                kind,
                properties: _,
                exp,
                additional_exps,
            } => {
                kind.ast_debug(w);
                exp.ast_debug(w);
                w.list(additional_exps, ",", |w, e| {
                    e.ast_debug(w);
                    true
                });
            },
            SpecBlockMember_::Function {
                uninterpreted,
                signature,
                name,
                body,
            } => {
                if *uninterpreted {
                    w.write("uninterpreted ")
                } else if let FunctionBody_::Native = &body.value {
                    w.write("native ");
                }
                w.write(&format!("define {}", name));
                signature.ast_debug(w);
                match &body.value {
                    FunctionBody_::Defined(body) => w.block(|w| body.ast_debug(w)),
                    FunctionBody_::Native => w.writeln(";"),
                }
            },
            SpecBlockMember_::Variable {
                is_global,
                name,
                type_parameters,
                type_,
                init: _,
            } => {
                if *is_global {
                    w.write("global ");
                } else {
                    w.write("local");
                }
                w.write(&format!("{}", name));
                type_parameters.ast_debug(w);
                w.write(": ");
                type_.ast_debug(w);
            },
            SpecBlockMember_::Update { lhs, rhs } => {
                w.write("update ");
                lhs.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            },
            SpecBlockMember_::Let {
                name,
                post_state,
                def,
            } => {
                w.write(&format!(
                    "let {}{} = ",
                    if *post_state { "post " } else { "" },
                    name
                ));
                def.ast_debug(w);
            },
            SpecBlockMember_::Include { properties: _, exp } => {
                w.write("include ");
                exp.ast_debug(w);
            },
            SpecBlockMember_::Apply {
                exp,
                patterns,
                exclusion_patterns,
            } => {
                w.write("apply ");
                exp.ast_debug(w);
                w.write(" to ");
                w.list(patterns, ", ", |w, p| {
                    p.ast_debug(w);
                    true
                });
                if !exclusion_patterns.is_empty() {
                    w.write(" exclude ");
                    w.list(exclusion_patterns, ", ", |w, p| {
                        p.ast_debug(w);
                        true
                    });
                }
            },
            SpecBlockMember_::Pragma { properties } => {
                w.write("pragma ");
                w.list(properties, ", ", |w, p| {
                    p.ast_debug(w);
                    true
                });
            },
        }
    }
}

impl AstDebug for PragmaProperty_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(self.name.value);
        if let Some(value) = &self.value {
            w.write(" = ");
            match value {
                PragmaValue::Literal(l) => l.ast_debug(w),
                PragmaValue::Ident(i) => i.ast_debug(w),
            }
        }
    }
}

impl AstDebug for (FunctionName, &Function) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (
            name,
            Function {
                attributes,
                loc: _loc,
                inline,
                visibility,
                entry,
                signature,
                acquires,
                access_specifiers: _,
                body,
                specs: _specs,
            },
        ) = self;
        attributes.ast_debug(w);
        visibility.ast_debug(w);
        if entry.is_some() {
            w.write(&format!("{} ", ENTRY_MODIFIER));
        }
        if let FunctionBody_::Native = &body.value {
            w.write("native ");
        }
        if *inline {
            w.write(&format!("inline fun {}", name));
        } else {
            w.write(&format!("fun {}", name));
        }
        signature.ast_debug(w);
        if !acquires.is_empty() {
            w.write(" acquires ");
            w.comma(acquires, |w, m| m.ast_debug(w));
            w.write(" ");
        }
        match &body.value {
            FunctionBody_::Defined(body) => w.block(|w| body.ast_debug(w)),
            FunctionBody_::Native => w.writeln(";"),
        }
    }
}

impl AstDebug for Visibility {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(&format!("{} ", self))
    }
}

impl AstDebug for FunctionSignature {
    fn ast_debug(&self, w: &mut AstWriter) {
        let FunctionSignature {
            type_parameters,
            parameters,
            return_type,
        } = self;
        type_parameters.ast_debug(w);
        w.write("(");
        w.comma(parameters, |w, (v, st)| {
            w.write(&format!("{}: ", v));
            st.ast_debug(w);
        });
        w.write("): ");
        return_type.ast_debug(w)
    }
}

impl AstDebug for (ConstantName, &Constant) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (
            name,
            Constant {
                attributes,
                loc: _loc,
                signature,
                value,
            },
        ) = self;
        attributes.ast_debug(w);
        w.write(&format!("const {}:", name));
        signature.ast_debug(w);
        w.write(" = ");
        value.ast_debug(w);
        w.write(";");
    }
}

impl AstDebug for Type_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        match self {
            Type_::Unit => w.write("()"),
            Type_::Multiple(ss) => {
                w.write("(");
                ss.ast_debug(w);
                w.write(")")
            },
            Type_::Apply(m, ss) => {
                m.ast_debug(w);
                if !ss.is_empty() {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
            },
            Type_::Ref(mut_, s) => {
                w.write("&");
                if *mut_ {
                    w.write("mut ");
                }
                s.ast_debug(w)
            },
            Type_::Fun(args, result, abilities) => {
                w.write("|");
                w.comma(args, |w, ty| ty.ast_debug(w));
                w.write("|");
                result.ast_debug(w);
                ability_constraints_ast_debug(w, abilities);
            },
            Type_::UnresolvedError => w.write("_|_"),
        }
    }
}

impl AstDebug for Vec<(Name, AbilitySet)> {
    fn ast_debug(&self, w: &mut AstWriter) {
        if !self.is_empty() {
            w.write("<");
            w.comma(self, |w, tp| tp.ast_debug(w));
            w.write(">")
        }
    }
}

impl AstDebug for Vec<StructTypeParameter> {
    fn ast_debug(&self, w: &mut AstWriter) {
        if !self.is_empty() {
            w.write("<");
            w.comma(self, |w, tp| tp.ast_debug(w));
            w.write(">")
        }
    }
}

pub fn ability_constraints_ast_debug(w: &mut AstWriter, abilities: &AbilitySet) {
    if !abilities.is_empty() {
        w.write(": ");
        w.list(abilities, "+", |w, ab| {
            ab.ast_debug(w);
            false
        })
    }
}

impl AstDebug for (Name, AbilitySet) {
    fn ast_debug(&self, w: &mut AstWriter) {
        let (n, abilities) = self;
        w.write(n.value);
        ability_constraints_ast_debug(w, abilities)
    }
}

impl AstDebug for StructTypeParameter {
    fn ast_debug(&self, w: &mut AstWriter) {
        let Self {
            is_phantom,
            name,
            constraints,
        } = self;
        if *is_phantom {
            w.write("phantom ");
        }
        w.write(name.value);
        ability_constraints_ast_debug(w, constraints)
    }
}

impl AstDebug for Vec<Type> {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.comma(self, |w, s| s.ast_debug(w))
    }
}

impl AstDebug for ModuleAccess_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.write(&match self {
            ModuleAccess_::Name(n) => format!("{}", n),
            ModuleAccess_::ModuleAccess(m, n, opt_v) => format!(
                "{}::{}{}",
                m,
                n,
                if let Some(v) = opt_v {
                    format!("::{}", v)
                } else {
                    "".to_string()
                }
            ),
        })
    }
}

impl AstDebug for VecDeque<SequenceItem> {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.semicolon(self, |w, item| item.ast_debug(w))
    }
}

impl AstDebug for SequenceItem_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use SequenceItem_ as I;
        match self {
            I::Seq(e) => e.ast_debug(w),
            I::Declare(sp!(_, bs), ty_opt) => {
                w.write("let ");
                bs.ast_debug(w);
                if let Some(ty) = ty_opt {
                    ty.ast_debug(w)
                }
            },
            I::Bind(sp!(_, bs), e) => {
                w.write("let ");
                bs.ast_debug(w);
                w.write(" = ");
                e.ast_debug(w);
            },
        }
    }
}

impl AstDebug for Value_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use Value_ as V;
        match self {
            V::Address(addr) => w.write(&format!("@{}", addr)),
            V::InferredNum(u) => w.write(&format!("{}", u)),
            V::U8(u) => w.write(&format!("{}u8", u)),
            V::U16(u) => w.write(&format!("{}u16", u)),
            V::U32(u) => w.write(&format!("{}u32", u)),
            V::U64(u) => w.write(&format!("{}u64", u)),
            V::U128(u) => w.write(&format!("{}u128", u)),
            V::U256(u) => w.write(&format!("{}u256", u)),
            V::Bool(b) => w.write(&format!("{}", b)),
            V::Bytearray(v) => w.write(&format!("{:?}", v)),
        }
    }
}

impl AstDebug for Exp_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use Exp_ as E;
        match self {
            E::Unit { trailing } if !trailing => w.write("()"),
            E::Unit {
                trailing: _trailing,
            } => w.write("/*()*/"),
            E::Value(v) => v.ast_debug(w),
            E::Move(v) => w.write(&format!("move {}", v)),
            E::Copy(v) => w.write(&format!("copy {}", v)),
            E::Name(ma, tys_opt) => {
                ma.ast_debug(w);
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
            },
            E::Call(ma, kind, tys_opt, sp!(_, rhs)) => {
                ma.ast_debug(w);
                w.write(kind.to_string());
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("(");
                w.comma(rhs, |w, e| e.ast_debug(w));
                w.write(")");
            },
            E::ExpCall(fexp, sp!(_, rhs)) => {
                fexp.ast_debug(w);
                w.write("(");
                w.comma(rhs, |w, e| e.ast_debug(w));
                w.write(")");
            },
            E::Pack(ma, tys_opt, fields) => {
                ma.ast_debug(w);
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("{");
                w.comma(fields, |w, (_, f, idx_e)| {
                    let (idx, e) = idx_e;
                    w.write(&format!("{}#{}: ", idx, f));
                    e.ast_debug(w);
                });
                w.write("}");
            },
            E::Vector(_loc, tys_opt, sp!(_, elems)) => {
                w.write("vector");
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("[");
                w.comma(elems, |w, e| e.ast_debug(w));
                w.write("]");
            },
            E::IfElse(b, t, f) => {
                w.write("if (");
                b.ast_debug(w);
                w.write(") ");
                t.ast_debug(w);
                w.write(" else ");
                f.ast_debug(w);
            },
            E::While(l, b, e) => {
                if let Some(l) = l {
                    w.write(&format!("{}: ", l.value().as_str()))
                }
                w.write("while (");
                b.ast_debug(w);
                w.write(")");
                e.ast_debug(w);
            },
            E::Loop(l, e) => {
                if let Some(l) = l {
                    w.write(&format!("{}: ", l.value().as_str()))
                }
                w.write("loop ");
                e.ast_debug(w);
            },
            E::Match(e, arms) => {
                w.write("match (");
                e.ast_debug(w);
                w.write(") {");
                for arm in arms {
                    arm.value.0.ast_debug(w);
                    if let Some(cond) = &arm.value.1 {
                        w.write(" if ");
                        cond.ast_debug(w);
                    }
                    w.write(" => ");
                    arm.value.2.ast_debug(w)
                }
            },
            E::Block(seq) => w.block(|w| seq.ast_debug(w)),
            E::Lambda(sp!(_, bs), e, capture_kind, abilities) => {
                if *capture_kind != LambdaCaptureKind::Default {
                    w.write(format!(" {}", capture_kind));
                }
                w.write("|");
                bs.ast_debug(w);
                w.write("|");
                e.ast_debug(w);
                ability_constraints_ast_debug(w, abilities)
            },
            E::Quant(kind, sp!(_, rs), trs, c_opt, e) => {
                kind.ast_debug(w);
                w.write(" ");
                rs.ast_debug(w);
                trs.ast_debug(w);
                if let Some(c) = c_opt {
                    w.write(" where ");
                    c.ast_debug(w);
                }
                w.write(" : ");
                e.ast_debug(w);
            },
            E::ExpList(es) => {
                w.write("(");
                w.comma(es, |w, e| e.ast_debug(w));
                w.write(")");
            },

            E::Assign(sp!(_, lvalues), rhs) => {
                lvalues.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            },
            E::FieldMutate(ed, rhs) => {
                ed.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            },
            E::Mutate(lhs, rhs) => {
                if !matches!(lhs.value, E::Index(_, _)) {
                    w.write("*");
                }
                lhs.ast_debug(w);
                w.write(" = ");
                rhs.ast_debug(w);
            },

            E::Return(e) => {
                w.write("return ");
                e.ast_debug(w);
            },
            E::Abort(e) => {
                w.write("abort ");
                e.ast_debug(w);
            },
            E::Break(l) => {
                w.write("break");
                if let Some(l) = l {
                    w.write(format!(" {}", l.value().as_str()));
                }
            },
            E::Continue(l) => {
                w.write("continue");
                if let Some(l) = l {
                    w.write(format!(" {}", l.value().as_str()));
                }
            },
            E::Dereference(e) => {
                w.write("*");
                e.ast_debug(w)
            },
            E::UnaryExp(op, e) => {
                op.ast_debug(w);
                w.write(" ");
                e.ast_debug(w);
            },
            E::BinopExp(l, op, r) => {
                l.ast_debug(w);
                w.write(" ");
                op.ast_debug(w);
                w.write(" ");
                r.ast_debug(w)
            },
            E::Borrow(mut_, e) => {
                w.write("&");
                if *mut_ {
                    w.write("mut ");
                }
                e.ast_debug(w);
            },
            E::ExpDotted(ed) => ed.ast_debug(w),
            E::Cast(e, ty) => {
                w.write("(");
                e.ast_debug(w);
                w.write(" as ");
                ty.ast_debug(w);
                w.write(")");
            },
            E::Test(e, tys) => {
                w.write("(");
                e.ast_debug(w);
                w.write(" is ");
                w.list(tys, "|", |w, item| {
                    item.ast_debug(w);
                    false
                });
                w.write(")");
            },
            E::Index(oper, index) => {
                oper.ast_debug(w);
                w.write("[");
                index.ast_debug(w);
                w.write("]");
            },
            E::Annotate(e, ty) => {
                w.write("(");
                e.ast_debug(w);
                w.write(": ");
                ty.ast_debug(w);
                w.write(")");
            },
            E::Spec(u, unbound_vars, unbound_func_ptrs) => {
                w.write(&format!("spec #{}", u));
                if !unbound_vars.is_empty() {
                    w.write(" uses [");
                    w.comma(unbound_vars, |w, n| w.write(&format!("{}", n)));
                    w.write("]");
                }
                if !unbound_func_ptrs.is_empty() {
                    w.write(" applies [");
                    w.comma(unbound_func_ptrs, |w, n| w.write(&format!("{}", n)));
                    w.write("]");
                }
            },
            E::UnresolvedError => w.write("_|_"),
        }
    }
}

impl AstDebug for ExpDotted_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use ExpDotted_ as D;
        match self {
            D::Exp(e) => e.ast_debug(w),
            D::Dot(e, n) => {
                e.ast_debug(w);
                w.write(&format!(".{}", n))
            },
        }
    }
}

impl AstDebug for Vec<TypedLValue> {
    fn ast_debug(&self, w: &mut AstWriter) {
        w.comma(self, |w, b| b.ast_debug(w));
    }
}

impl AstDebug for TypedLValue_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        let TypedLValue_(lv, opt_ty) = self;
        lv.ast_debug(w);
        if let Some(ty) = opt_ty {
            w.write(":");
            ty.ast_debug(w);
        }
    }
}

impl AstDebug for Vec<LValue> {
    fn ast_debug(&self, w: &mut AstWriter) {
        let parens = self.len() != 1;
        if parens {
            w.write("(");
        }
        w.comma(self, |w, b| b.ast_debug(w));
        if parens {
            w.write(")");
        }
    }
}

impl AstDebug for LValue_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use LValue_ as L;
        match self {
            L::Var(v, tys_opt) => {
                w.write(&format!("{}", v));
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
            },
            L::Unpack(ma, tys_opt, fields, dotdot) => {
                ma.ast_debug(w);
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("{");
                w.comma(fields, |w, (_, f, idx_b)| {
                    let (idx, b) = idx_b;
                    w.write(&format!("{}#{}: ", idx, f));
                    b.ast_debug(w);
                });
                if !fields.is_empty() {
                    w.write(", ");
                }
                if dotdot.is_some() {
                    w.write("..");
                }
                w.write("}");
            },
            L::PositionalUnpack(ma, tys_opt, args) => {
                ma.ast_debug(w);
                if let Some(ss) = tys_opt {
                    w.write("<");
                    ss.ast_debug(w);
                    w.write(">");
                }
                w.write("(");
                w.comma(&args.value, |w, b| b.ast_debug(w));
                w.write(")");
            },
        }
    }
}

impl AstDebug for LValueOrDotDot_ {
    fn ast_debug(&self, w: &mut AstWriter) {
        use LValueOrDotDot_::*;
        match self {
            LValue(l) => l.ast_debug(w),
            DotDot => w.write(".."),
        }
    }
}

impl AstDebug for Vec<LValueWithRange> {
    fn ast_debug(&self, w: &mut AstWriter) {
        let parens = self.len() != 1;
        if parens {
            w.write("(");
        }
        w.comma(self, |w, b| b.ast_debug(w));
        if parens {
            w.write(")");
        }
    }
}

impl AstDebug for (LValue, Exp) {
    fn ast_debug(&self, w: &mut AstWriter) {
        self.0.ast_debug(w);
        w.write(" in ");
        self.1.ast_debug(w);
    }
}

impl AstDebug for Vec<Vec<Exp>> {
    fn ast_debug(&self, w: &mut AstWriter) {
        for trigger in self {
            w.write("{");
            w.comma(trigger, |w, b| b.ast_debug(w));
            w.write("}");
        }
    }
}
