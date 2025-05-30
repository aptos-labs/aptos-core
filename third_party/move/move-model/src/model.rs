// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Provides a model for a set of Move modules (and scripts, which
//! are handled like modules). The model allows to access many different aspects of the Move
//! code: all declared functions and types, their associated bytecode, their source location,
//! their source text, and the specification fragments.
//!
//! The environment is nested into a hierarchy:
//!
//! - A `GlobalEnv` which gives access to all modules plus other information on global level,
//!   and is the owner of all related data.
//! - A `ModuleEnv` which is a reference to the data of some module in the environment.
//! - A `StructEnv` which is a reference to the data of some struct in a module.
//! - A `FunctionEnv` which is a reference to the data of some function in a module.

use crate::{
    ast::{
        AccessSpecifier, AccessSpecifierKind, Address, AddressSpecifier, Attribute, ConditionKind,
        Exp, ExpData, FriendDecl, GlobalInvariant, ModuleName, PropertyBag, PropertyValue,
        ResourceSpecifier, Spec, SpecBlockInfo, SpecBlockTarget, SpecFunDecl, SpecVarDecl, UseDecl,
        Value,
    },
    code_writer::CodeWriter,
    emit, emitln,
    intrinsics::IntrinsicsAnnotation,
    metadata::LanguageVersion,
    pragmas::{
        DELEGATE_INVARIANTS_TO_CALLER_PRAGMA, DISABLE_INVARIANTS_IN_BODY_PRAGMA, FRIEND_PRAGMA,
        INTRINSIC_PRAGMA, OPAQUE_PRAGMA, VERIFY_PRAGMA,
    },
    symbol::{Symbol, SymbolPool},
    ty::{
        AbilityInference, AbilityInferer, NoUnificationContext, Type, TypeDisplayContext, Variance,
    },
    ty_invariant_analysis::TypeUnificationAdapter,
    well_known,
};
use anyhow::bail;
use codespan::{ByteIndex, ByteOffset, ColumnOffset, FileId, Files, LineOffset, Location, Span};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle, Severity},
    term::{emit, termcolor::WriteColor, Config},
};
use itertools::Itertools;
use legacy_move_compiler::command_line as cli;
#[allow(unused_imports)]
use log::{debug, info, warn};
pub use move_binary_format::file_format::Visibility;
#[allow(deprecated)]
use move_binary_format::normalized::Type as MType;
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    file_format::{
        Bytecode, CodeOffset, Constant as VMConstant, ConstantPoolIndex, FunctionDefinitionIndex,
        FunctionHandleIndex, MemberCount, SignatureIndex, SignatureToken, StructDefinitionIndex,
        VariantIndex,
    },
    views::{FunctionDefinitionView, FunctionHandleView, StructHandleView},
    CompiledModule,
};
use move_bytecode_source_map::{mapping::SourceMapping, source_map::SourceMap};
use move_command_line_common::{
    address::NumericalAddress, env::read_bool_env_var, files::FileHash,
};
pub use move_core_types::ability::AbilitySet;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage,
    value::MoveValue,
};
use move_disassembler::disassembler::{Disassembler, DisassemblerOptions};
use num::ToPrimitive;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    backtrace::{Backtrace, BacktraceStatus},
    cell::{Ref, RefCell, RefMut},
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, VecDeque},
    ffi::OsStr,
    fmt::{self, Formatter, Write},
    rc::Rc,
};

static DEBUG_TRACE: bool = true;

// =================================================================================================
/// # Constants

/// A name we use to represent a script as a module.
pub const SCRIPT_MODULE_NAME: &str = "<SELF>";

/// Names used in the bytecode/AST to represent the main function of a script
pub const SCRIPT_BYTECODE_FUN_NAME: &str = "<SELF>";

/// A prefix used for structs which are backing specification ("ghost") memory.
pub const GHOST_MEMORY_PREFIX: &str = "Ghost$";

// =================================================================================================
/// # Locations

/// A location, consisting of a FileId and a span in this file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct Loc {
    file_id: FileId,
    span: Span,
    inlined_from_loc: Option<Box<Loc>>,
}

impl AsRef<Loc> for Loc {
    fn as_ref(&self) -> &Loc {
        self
    }
}

impl Loc {
    pub fn new(file_id: FileId, span: Span) -> Loc {
        Loc {
            file_id,
            span,
            inlined_from_loc: None,
        }
    }

    pub fn inlined_from(&self, inlined_from: &Loc) -> Loc {
        Loc {
            file_id: self.file_id,
            span: self.span,
            inlined_from_loc: Some(Box::new(match &self.inlined_from_loc {
                None => inlined_from.clone(),
                Some(locbox) => (*locbox.clone()).inlined_from(inlined_from),
            })),
        }
    }

    /// Checks if `self` is an inlined location.
    pub fn is_inlined(&self) -> bool {
        self.inlined_from_loc.is_some()
    }

    // If `self` is an inlined `Loc`, then add the same
    // inlining info to the parameter `loc`.
    fn inline_if_needed(&self, loc: Loc) -> Loc {
        if let Some(locbox) = &self.inlined_from_loc {
            let source_loc = locbox.as_ref();
            loc.inlined_from(source_loc)
        } else {
            loc
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    // Delivers a location pointing to the end of this one.
    pub fn at_end(&self) -> Loc {
        if self.span.end() > ByteIndex(0) {
            self.inline_if_needed(Loc::new(
                self.file_id,
                Span::new(self.span.end() - ByteOffset(1), self.span.end()),
            ))
        } else {
            self.clone()
        }
    }

    // Delivers a location pointing to the start of this one.
    pub fn at_start(&self) -> Loc {
        self.inline_if_needed(Loc::new(
            self.file_id,
            Span::new(self.span.start(), self.span.start() + ByteOffset(1)),
        ))
    }

    /// Creates a location which encloses all the locations in the provided slice,
    /// which must not be empty. All locations are expected to be in the same file.
    pub fn enclosing(locs: &[Loc]) -> Loc {
        assert!(!locs.is_empty());
        let loc = &locs[0];
        let mut start = loc.span.start();
        let mut end = loc.span.end();
        for l in locs.iter().skip(1) {
            if l.file_id() == loc.file_id() {
                start = std::cmp::min(start, l.span().start());
                end = std::cmp::max(end, l.span().end());
            }
        }
        loc.inline_if_needed(Loc::new(loc.file_id(), Span::new(start, end)))
    }

    /// Returns true if the other location is enclosed by this location.
    pub fn is_enclosing(&self, other: &Loc) -> bool {
        self.file_id == other.file_id
            && self.inlined_from_loc == other.inlined_from_loc
            && GlobalEnv::enclosing_span(self.span, other.span)
    }

    /// Returns true if this location is the default one.
    pub fn is_default(&self) -> bool {
        *self == Loc::default()
    }
}

impl Default for Loc {
    fn default() -> Self {
        static DEFAULT: Lazy<Loc> = Lazy::new(|| {
            let mut files = Files::new();
            let dummy_id = files.add(String::new(), String::new());
            Loc::new(dummy_id, Span::default())
        });
        DEFAULT.clone()
    }
}

/// Alias for the Loc variant of MoveIR.
pub type MoveIrLoc = move_ir_types::location::Loc;
pub type MoveIrByteIndex = move_ir_types::location::ByteIndex;

// =================================================================================================
/// # Identifiers
///
/// Identifiers are opaque values used to reference entities in the environment.
///
/// We have two kinds of ids: those based on an index, and those based on a symbol. We use
/// the symbol based ids where we do not have control of the definition index order in bytecode
/// (i.e. we do not know in which order move-compiler enters functions and structs into file format),
/// and index based ids where we do have control (for modules, SpecFun and SpecVar).
///
/// In any case, ids are opaque in the sense that if someone has a StructId or similar in hand,
/// it is known to be defined in the environment, as it has been obtained also from the environment.

/// Raw index type used in ids. 16 bits are sufficient currently.
pub type RawIndex = u16;

/// Identifier for a module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct ModuleId(RawIndex);

/// Identifier for a named constant, relative to module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct NamedConstantId(Symbol);

/// Identifier for a structure/resource, relative to module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct StructId(Symbol);

/// Identifier for a field of a structure, relative to struct.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct FieldId(Symbol);

/// Identifier for a Move function, relative to module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct FunId(Symbol);

/// Identifier for a schema.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct SchemaId(Symbol);

/// Identifier for a specification function, relative to module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct SpecFunId(RawIndex);

/// Identifier for a specification variable, relative to module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct SpecVarId(RawIndex);

/// Identifier for a node in the AST, relative to a module. This is used to associate attributes
/// with the node, like source location and type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct NodeId(usize);

/// A global id. Instances of this type represent unique identifiers relative to `GlobalEnv`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct GlobalId(usize);

/// Identifier for an intrinsic declaration, relative globally in `GlobalEnv`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct IntrinsicId(usize);

/// Some identifier qualified by a module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct QualifiedId<Id> {
    pub module_id: ModuleId,
    pub id: Id,
}

/// Some identifier qualified by a module and a type instantiation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct QualifiedInstId<Id> {
    pub module_id: ModuleId,
    pub inst: Vec<Type>,
    pub id: Id,
}

impl NamedConstantId {
    pub fn new(sym: Symbol) -> Self {
        Self(sym)
    }

    pub fn symbol(self) -> Symbol {
        self.0
    }
}

impl FunId {
    pub fn new(sym: Symbol) -> Self {
        Self(sym)
    }

    pub fn symbol(self) -> Symbol {
        self.0
    }
}

impl SchemaId {
    pub fn new(sym: Symbol) -> Self {
        Self(sym)
    }

    pub fn symbol(self) -> Symbol {
        self.0
    }
}

impl StructId {
    pub fn new(sym: Symbol) -> Self {
        Self(sym)
    }

    pub fn symbol(self) -> Symbol {
        self.0
    }
}

impl FieldId {
    pub fn new(sym: Symbol) -> Self {
        Self(sym)
    }

    pub fn symbol(self) -> Symbol {
        self.0
    }

    /// Makes a variant field name unique to a given struct.
    /// TODO: Consider making FieldId containing two symbols, but this can be a breaking change
    ///   to public APIs.
    pub fn make_variant_field_id_str(
        variant_name: impl AsRef<str>,
        field_name: impl AsRef<str>,
    ) -> String {
        format!("{}.{}", variant_name.as_ref(), field_name.as_ref())
    }
}

impl SpecFunId {
    pub fn new(idx: usize) -> Self {
        Self(idx as RawIndex)
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl SpecVarId {
    pub fn new(idx: usize) -> Self {
        Self(idx as RawIndex)
    }

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl NodeId {
    pub fn new(idx: usize) -> Self {
        Self(idx)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl AsRef<NodeId> for NodeId {
    fn as_ref(&self) -> &NodeId {
        self
    }
}

impl ModuleId {
    pub fn new(idx: usize) -> Self {
        Self(idx as RawIndex)
    }

    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
}

impl ModuleId {
    pub fn qualified<Id>(self, id: Id) -> QualifiedId<Id> {
        QualifiedId {
            module_id: self,
            id,
        }
    }

    pub fn qualified_inst<Id>(self, id: Id, inst: Vec<Type>) -> QualifiedInstId<Id> {
        QualifiedInstId {
            module_id: self,
            inst,
            id,
        }
    }
}

impl GlobalId {
    pub fn new(idx: usize) -> Self {
        Self(idx)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl IntrinsicId {
    pub fn new(idx: usize) -> Self {
        Self(idx)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl<Id: Clone> QualifiedId<Id> {
    pub fn instantiate(self, inst: Vec<Type>) -> QualifiedInstId<Id> {
        let QualifiedId { module_id, id } = self;
        QualifiedInstId {
            module_id,
            inst,
            id,
        }
    }
}

impl<Id: Clone> QualifiedInstId<Id> {
    pub fn instantiate(self, params: &[Type]) -> Self {
        if params.is_empty() {
            self
        } else {
            let Self {
                module_id,
                inst,
                id,
            } = self;
            Self {
                module_id,
                inst: Type::instantiate_vec(inst, params),
                id,
            }
        }
    }

    pub fn instantiate_ref(&self, params: &[Type]) -> Self {
        let res = self.clone();
        res.instantiate(params)
    }

    pub fn to_qualified_id(&self) -> QualifiedId<Id> {
        let Self { module_id, id, .. } = self;
        module_id.qualified(id.to_owned())
    }
}

impl QualifiedInstId<StructId> {
    pub fn to_type(&self) -> Type {
        Type::Struct(self.module_id, self.id, self.inst.to_owned())
    }
}

// =================================================================================================
/// # Verification Scope

/// Defines what functions to verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationScope {
    /// Verify only public functions.
    Public,
    /// Verify all functions.
    All,
    /// Verify only one function.
    Only(String),
    /// Verify only functions from the given module.
    OnlyModule(String),
    /// Verify no functions
    None,
}

impl Default for VerificationScope {
    fn default() -> Self {
        Self::Public
    }
}

impl VerificationScope {
    /// Whether verification is exclusive to only one function or module. If set, this overrides
    /// all implicitly included verification targets via invariants and friends.
    pub fn is_exclusive(&self) -> bool {
        matches!(
            self,
            VerificationScope::Only(_) | VerificationScope::OnlyModule(_)
        )
    }

    /// Returns the target function if verification is exclusive to one function.
    pub fn get_exclusive_verify_function_name(&self) -> Option<&String> {
        match self {
            VerificationScope::Only(s) => Some(s),
            _ => None,
        }
    }
}

// =================================================================================================
/// # Global Environment

/// Global environment for a set of modules.
#[derive(Debug)]
pub struct GlobalEnv {
    /// The language version for which this model is build.
    pub(crate) language_version: LanguageVersion,
    /// A Files database for the codespan crate which supports diagnostics.
    pub(crate) source_files: Files<String>,
    /// A map of FileId in the Files database to information about documentation comments in a file.
    /// The comments are represented as map from ByteIndex into string, where the index is the
    /// start position of the associated language item in the source.
    pub(crate) doc_comments: BTreeMap<FileId, BTreeMap<ByteIndex, String>>,
    /// A mapping from file hash to file name and associated FileId.
    pub(crate) file_hash_map: BTreeMap<FileHash, (String, FileId)>,
    /// Reverse of the above mapping, mapping FileId to hash.
    pub(crate) reverse_file_hash_map: BTreeMap<FileId, FileHash>,
    /// Bijective mapping between FileId and a plain int. FileId's are themselves wrappers around
    /// ints, but the inner representation is opaque and cannot be accessed. This is used so we
    /// can emit FileId's to generated code and read them back.
    pub(crate) file_id_to_idx: BTreeMap<FileId, u16>,
    pub(crate) file_idx_to_id: BTreeMap<u16, FileId>,
    /// A set indicating whether a file id is a compilation target.
    pub(crate) file_id_is_target: BTreeSet<FileId>,
    /// A set indicating whether a file id is a test/docgen/warning/prover target.
    pub(crate) file_id_is_primary_target: BTreeSet<FileId>,
    /// A special constant location representing an unknown location.
    /// This uses a pseudo entry in `source_files` to be safely represented.
    pub(crate) unknown_loc: Loc,
    /// An equivalent of the MoveIrLoc to the above location. Used to map back and force between
    /// them.
    pub(crate) unknown_move_ir_loc: MoveIrLoc,
    /// A special constant location representing an opaque location.
    /// In difference to an `unknown_loc`, this is a well-known but undisclosed location.
    pub(crate) internal_loc: Loc,
    /// Accumulated diagnosis. In a RefCell so we can add to it without needing a mutable GlobalEnv.
    /// The boolean indicates whether the diag was reported.
    pub(crate) diags: RefCell<Vec<(Diagnostic<FileId>, bool)>>,
    /// Pool of symbols -- internalized strings.
    pub(crate) symbol_pool: SymbolPool,
    /// A counter for allocating node ids.
    pub(crate) next_free_node_id: RefCell<usize>,
    /// A map from node id to associated information of the expression.
    pub(crate) exp_info: RefCell<BTreeMap<NodeId, ExpInfo>>,
    /// List of loaded modules, in order they have been provided using `add`.
    pub module_data: Vec<ModuleData>,
    /// A counter for issuing global ids.
    pub(crate) global_id_counter: RefCell<usize>,
    /// A map of global invariants.
    pub(crate) global_invariants: BTreeMap<GlobalId, GlobalInvariant>,
    /// A map from global memories to global invariants which refer to them.
    pub(crate) global_invariants_for_memory:
        BTreeMap<QualifiedInstId<StructId>, BTreeSet<GlobalId>>,
    /// A set containing spec functions which are called/used in specs. Note that these
    /// are represented without type instantiation because we assume the backend can handle
    /// generics in the expression language.
    pub(crate) used_spec_funs: BTreeSet<QualifiedId<SpecFunId>>,
    /// An annotation of all intrinsic declarations
    pub(crate) intrinsics: IntrinsicsAnnotation,
    /// A type-indexed container for storing extension data in the environment.
    pub(crate) extensions: RefCell<BTreeMap<TypeId, Box<dyn Any>>>,
    /// The address of the standard and extension libraries.
    pub(crate) stdlib_address: Option<Address>,
    pub(crate) extlib_address: Option<Address>,
    /// Address alias map
    pub(crate) address_alias_map: BTreeMap<Symbol, AccountAddress>,
    /// A flag which allows to indicate that the whole program including
    /// dependencies should be built.
    pub(crate) everything_is_target: RefCell<bool>,
    /// Whether the v2 compiler has generated this model.
    /// TODO: replace with a proper version number once we have this in file format
    pub(crate) generated_by_v2: bool,
}

/// A helper type for implementing fmt::Display depending on GlobalEnv
pub struct EnvDisplay<'a, T> {
    pub env: &'a GlobalEnv,
    pub val: &'a T,
}

impl GlobalEnv {
    /// Creates a new environment.
    pub fn new() -> Self {
        let mut source_files = Files::new();
        let mut file_hash_map = BTreeMap::new();
        let mut reverse_file_hash_map = BTreeMap::new();
        let mut file_id_to_idx = BTreeMap::new();
        let mut file_idx_to_id = BTreeMap::new();
        let mut fake_loc = |content: &str| {
            let file_id = source_files.add(content, content.to_string());
            let file_hash = FileHash::new(content);
            file_hash_map.insert(file_hash, (content.to_string(), file_id));
            reverse_file_hash_map.insert(file_id, file_hash);
            let file_idx = file_id_to_idx.len() as u16;
            file_id_to_idx.insert(file_id, file_idx);
            file_idx_to_id.insert(file_idx, file_id);
            Loc::new(
                file_id,
                Span::from(ByteIndex(0_u32)..ByteIndex(content.len() as u32)),
            )
        };
        let unknown_loc = fake_loc("<unknown>");
        let unknown_move_ir_loc = MoveIrLoc::new(FileHash::new("<unknown>"), 0, 0);
        let internal_loc = fake_loc("<internal>");
        GlobalEnv {
            language_version: LanguageVersion::default(),
            source_files,
            doc_comments: Default::default(),
            unknown_loc,
            unknown_move_ir_loc,
            internal_loc,
            file_hash_map,
            reverse_file_hash_map,
            file_id_to_idx,
            file_idx_to_id,
            file_id_is_target: BTreeSet::new(),
            file_id_is_primary_target: BTreeSet::new(),
            diags: RefCell::new(vec![]),
            symbol_pool: SymbolPool::new(),
            next_free_node_id: Default::default(),
            exp_info: Default::default(),
            module_data: vec![],
            global_id_counter: RefCell::new(0),
            global_invariants: Default::default(),
            global_invariants_for_memory: Default::default(),
            used_spec_funs: BTreeSet::new(),
            intrinsics: Default::default(),
            extensions: Default::default(),
            stdlib_address: None,
            extlib_address: None,
            address_alias_map: Default::default(),
            everything_is_target: Default::default(),
            generated_by_v2: false,
        }
    }

    /// Sets whether this is generated by the v2 compiler.
    pub fn set_compiler_v2(&mut self, yes: bool) {
        self.generated_by_v2 = yes
    }

    /// Returns true if this is generated by v2.
    pub fn generated_by_v2(&self) -> bool {
        self.generated_by_v2
    }

    /// Sets the language version.
    pub fn set_language_version(&mut self, version: LanguageVersion) {
        self.language_version = version
    }

    /// Returns the language version
    pub fn language_version(&self) -> LanguageVersion {
        self.language_version
    }

    /// Creates a display container for the given value. There must be an implementation
    /// of fmt::Display for an instance to work in formatting.
    pub fn display<'a, T>(&'a self, val: &'a T) -> EnvDisplay<'a, T> {
        EnvDisplay { env: self, val }
    }

    /// Sets the global address alias map
    pub fn set_address_alias_map(&mut self, map: BTreeMap<Symbol, AccountAddress>) {
        self.address_alias_map = map
    }

    /// Gets the global address alias map
    pub fn get_address_alias_map(&self) -> &BTreeMap<Symbol, AccountAddress> {
        &self.address_alias_map
    }

    /// Indicates that all modules in the environment should be treated as
    /// target modules, i.e. `module.is_target()` returns true. This can be
    /// used to temporarily override the default which distinguishes
    /// between dependencies and target modules, and is used for tools like
    /// the prover which need to compile all code, while still maintaining
    /// the difference between targets and dependencies for verification.
    /// Those tools can temporarily set this to true.
    pub fn treat_everything_as_target(&self, on: bool) {
        *self.everything_is_target.borrow_mut() = on
    }

    /// Attempts to resolve address alias.
    pub fn resolve_address_alias(&self, alias: Symbol) -> Option<AccountAddress> {
        self.address_alias_map.get(&alias).cloned()
    }

    /// Stores extension data in the environment. This can be arbitrary data which is
    /// indexed by type. Used by tools which want to store their own data in the environment,
    /// like a set of tool dependent options/flags. This can also be used to update
    /// extension data.
    pub fn set_extension<T: Any>(&self, x: T) {
        let id = TypeId::of::<T>();
        self.extensions
            .borrow_mut()
            .insert(id, Box::new(Rc::new(x)));
    }

    /// Retrieves extension data from the environment. Use as in `env.get_extension::<T>()`.
    /// An `Rc<T>` is returned because extension data is stored in a RefCell and we can't use
    /// lifetimes (`&'a T`) to control borrowing.
    pub fn get_extension<T: Any>(&self) -> Option<Rc<T>> {
        let id = TypeId::of::<T>();
        self.extensions
            .borrow()
            .get(&id)
            .and_then(|d| d.downcast_ref::<Rc<T>>().cloned())
    }

    /// Retrieves a clone of the extension data from the environment. Use as in `env.get_cloned_extension::<T>()`.
    pub fn get_cloned_extension<T: Any + Clone>(&self) -> T {
        let id = TypeId::of::<T>();
        let d = self
            .extensions
            .borrow_mut()
            .remove(&id)
            .expect("extension defined")
            .downcast_ref::<Rc<T>>()
            .cloned()
            .unwrap();
        Rc::try_unwrap(d).unwrap_or_else(|d| d.as_ref().clone())
    }

    /// Updates extension data. If they are no outstanding references to this extension it
    /// is updated in place, otherwise it will be cloned before the update.
    pub fn update_extension<T: Any + Clone>(&self, f: impl FnOnce(&mut T)) {
        let id = TypeId::of::<T>();
        let d = self
            .extensions
            .borrow_mut()
            .remove(&id)
            .expect("extension defined")
            .downcast_ref::<Rc<T>>()
            .cloned()
            .unwrap();
        let mut curr = Rc::try_unwrap(d).unwrap_or_else(|d| d.as_ref().clone());
        f(&mut curr);
        self.set_extension(curr);
    }

    /// Checks whether there is an extension of type `T`.
    pub fn has_extension<T: Any>(&self) -> bool {
        let id = TypeId::of::<T>();
        self.extensions.borrow().contains_key(&id)
    }

    /// Clear extension data from the environment (return the data if it is previously set).
    /// Use as in `env.clear_extension::<T>()` and an `Rc<T>` is returned if the extension data is
    /// previously stored in the environment.
    pub fn clear_extension<T: Any>(&self) -> Option<Rc<T>> {
        let id = TypeId::of::<T>();
        self.extensions
            .borrow_mut()
            .remove(&id)
            .and_then(|d| d.downcast::<Rc<T>>().ok())
            .map(|boxed| *boxed)
    }

    /// Create a new global id unique to this environment.
    pub fn new_global_id(&self) -> GlobalId {
        let mut counter = self.global_id_counter.borrow_mut();
        let id = GlobalId::new(*counter);
        *counter += 1;
        id
    }

    /// Returns a reference to the symbol pool owned by this environment.
    pub fn symbol_pool(&self) -> &SymbolPool {
        &self.symbol_pool
    }

    /// Adds a source to this environment, returning a FileId for it.
    pub fn add_source(
        &mut self,
        file_hash: FileHash,
        address_aliases: Rc<BTreeMap<Symbol, NumericalAddress>>,
        file_name: &str,
        source: &str,
        is_target: bool,
        is_primary_target: bool,
    ) -> FileId {
        // Check for address alias conflicts.
        self.stdlib_address =
            self.resolve_std_address_alias(self.stdlib_address.clone(), "std", &address_aliases);
        self.extlib_address = self.resolve_std_address_alias(
            self.extlib_address.clone(),
            "Extensions",
            &address_aliases,
        );
        if let Some((_filename, file_id)) = self.file_hash_map.get(&file_hash) {
            // This is a duplicate source, make sure it is marked as a target
            // and/or a primary_target if any instance marks it as such.
            if is_target && !self.file_id_is_target.contains(file_id) {
                self.file_id_is_target.insert(*file_id);
            }
            if is_primary_target && !self.file_id_is_primary_target.contains(file_id) {
                self.file_id_is_primary_target.insert(*file_id);
            }
            *file_id
        } else {
            // Record new source file and properties
            let file_id = self.source_files.add(file_name, source.to_string());
            self.file_hash_map
                .insert(file_hash, (file_name.to_string(), file_id));
            self.reverse_file_hash_map.insert(file_id, file_hash);
            let file_idx = self.file_id_to_idx.len() as u16;
            self.file_id_to_idx.insert(file_id, file_idx);
            self.file_idx_to_id.insert(file_idx, file_id);
            if is_target {
                self.file_id_is_target.insert(file_id);
            }
            if is_primary_target {
                self.file_id_is_primary_target.insert(file_id);
            }
            file_id
        }
    }

    fn resolve_std_address_alias(
        &self,
        def: Option<Address>,
        name: &str,
        aliases: &BTreeMap<Symbol, NumericalAddress>,
    ) -> Option<Address> {
        let name_sym = self.symbol_pool().make(name);
        if let Some(a) = aliases.get(&name_sym) {
            let addr = Address::Numerical(a.into_inner());
            if matches!(&def, Some(other_addr) if &addr != other_addr) {
                self.error(
                    &self.unknown_loc,
                    &format!(
                        "Ambiguous definition of standard address alias `{}` (`0x{} != 0x{}`).\
                                 This alias currently must be unique across all packages.",
                        name,
                        self.display(&addr),
                        self.display(&def.unwrap())
                    ),
                );
            }
            Some(addr)
        } else {
            def
        }
    }

    /// Find all target modules and return in a vector
    pub fn get_target_modules(&self) -> Vec<ModuleEnv> {
        let mut target_modules: Vec<ModuleEnv> = vec![];
        for module_env in self.get_modules() {
            if module_env.is_target() {
                target_modules.push(module_env);
            }
        }
        target_modules
    }

    /// Find all target modules and their transitive closures and return in a vector
    pub fn get_target_modules_transitive_closure(&self) -> Vec<ModuleEnv> {
        let mut target_and_transitive_modules: BTreeSet<ModuleId> = BTreeSet::new();
        let mut todo_modules: BTreeSet<ModuleId> = BTreeSet::new();
        for module_env in self.get_modules() {
            if module_env.is_target() {
                todo_modules.insert(module_env.get_id());
            }
        }
        while let Some(module_id) = todo_modules.pop_first() {
            if !target_and_transitive_modules.contains(&module_id) {
                target_and_transitive_modules.insert(module_id);
            }
            let module_env = self.get_module(module_id);
            for func_env in module_env.get_functions() {
                let used_functions = func_env
                    .get_used_functions()
                    .expect("used functions available");
                for used_function in used_functions {
                    if !target_and_transitive_modules.contains(&used_function.module_id) {
                        todo_modules.insert(used_function.module_id);
                    }
                }
            }
        }
        target_and_transitive_modules
            .iter()
            .map(|id| self.get_module(*id))
            .collect_vec()
    }

    /// Find all primary target modules and return in a vector
    pub fn get_primary_target_modules(&self) -> Vec<ModuleEnv> {
        let mut target_modules: Vec<ModuleEnv> = vec![];
        for module_env in self.get_modules() {
            if module_env.is_primary_target() {
                target_modules.push(module_env);
            }
        }
        target_modules
    }

    fn add_backtrace(msg: &str, _is_bug: bool) -> String {
        // Note that you need both MOVE_COMPILER_BACKTRACE=1 and RUST_BACKTRACE=1 for this to
        // actually generate a backtrace.
        static DUMP_BACKTRACE: Lazy<bool> = Lazy::new(|| {
            read_bool_env_var(cli::MOVE_COMPILER_BACKTRACE_ENV_VAR)
                | read_bool_env_var(cli::MVC_BACKTRACE_ENV_VAR)
        });
        if *DUMP_BACKTRACE {
            let bt = Backtrace::capture();
            let msg_out = if BacktraceStatus::Captured == bt.status() {
                format!("{}\nBacktrace: {:#?}", msg, bt)
            } else {
                msg.to_owned()
            };
            if DEBUG_TRACE {
                debug!("{}", msg_out);
            }
            msg_out
        } else {
            msg.to_owned()
        }
    }

    /// Adds documentation for a file.
    pub fn add_documentation(&mut self, file_id: FileId, docs: BTreeMap<ByteIndex, String>) {
        self.doc_comments.insert(file_id, docs);
    }

    /// Adds diagnostic to the environment.
    pub fn add_diag(&self, diag: Diagnostic<FileId>) {
        self.diags.borrow_mut().push((diag, false));
    }

    /// Adds an error to this environment, without notes.
    pub fn error(&self, loc: &Loc, msg: &str) {
        self.diag(Severity::Error, loc, msg)
    }

    /// Adds a warning to this environment, without notes.
    pub fn warning(&self, loc: &Loc, msg: &str) {
        self.diag(Severity::Warning, loc, msg)
    }

    /// Adds an error to this environment, with notes.
    pub fn error_with_notes(&self, loc: &Loc, msg: &str, notes: Vec<String>) {
        self.diag_with_notes(Severity::Error, loc, msg, notes)
    }

    /// Adds an error to this environment, with notes.
    pub fn error_with_labels(&self, loc: &Loc, msg: &str, labels: Vec<(Loc, String)>) {
        self.diag_with_labels(Severity::Error, loc, msg, labels)
    }

    /// Add a label to `labels` to specify "inlined from loc" for the `loc` in `inlined_from`,
    /// and, if that is inlined from someplace, repeat as needed, etc.
    fn add_inlined_from_labels(labels: &mut Vec<Label<FileId>>, inlined_from: &Option<Box<Loc>>) {
        let mut inlined_from = inlined_from;
        while let Some(boxed_loc) = inlined_from {
            let loc = boxed_loc.as_ref();
            let new_label = Label::secondary(loc.file_id, loc.span)
                .with_message("from a call inlined at this callsite");
            labels.push(new_label);
            inlined_from = &loc.inlined_from_loc;
        }
    }

    /// Adds a diagnostic of given severity to this environment.
    pub fn diag(&self, severity: Severity, loc: &Loc, msg: &str) {
        self.diag_with_primary_notes_and_labels(severity, loc, msg, "", vec![], vec![])
    }

    /// Add a lint warning to this environment, with the `msg` and `notes`.
    pub fn lint_diag_with_notes(&self, loc: &Loc, msg: &str, notes: Vec<String>) {
        let lint_msg = format!("[lint] {}", msg);
        self.diag_with_notes(Severity::Warning, loc, &lint_msg, notes)
    }

    /// Adds a diagnostic of given severity to this environment, with notes.
    pub fn diag_with_notes(&self, severity: Severity, loc: &Loc, msg: &str, notes: Vec<String>) {
        self.diag_with_primary_notes_and_labels(severity, loc, msg, "", notes, vec![])
    }

    /// Adds a diagnostic of given severity to this environment, with labels.
    pub fn diag_with_labels(
        &self,
        severity: Severity,
        loc: &Loc,
        msg: &str,
        labels: Vec<(Loc, String)>,
    ) {
        self.diag_with_primary_notes_and_labels(severity, loc, msg, "", vec![], labels)
    }

    /// Adds a diagnostic of given severity to this environment, with primary and primary labels.
    pub fn diag_with_primary_and_labels(
        &self,
        severity: Severity,
        loc: &Loc,
        msg: &str,
        primary: &str,
        labels: Vec<(Loc, String)>,
    ) {
        self.diag_with_primary_notes_and_labels(severity, loc, msg, primary, vec![], labels)
    }

    /// Adds a diagnostic of given severity to this environment, with notes and labels.
    pub fn diag_with_primary_notes_and_labels(
        &self,
        severity: Severity,
        loc: &Loc,
        msg: &str,
        primary: &str,
        notes: Vec<String>,
        labels: Vec<(Loc, String)>,
    ) {
        let new_msg = Self::add_backtrace(msg, severity == Severity::Bug);

        let mut primary_label = Label::primary(loc.file_id, loc.span);
        if !primary.is_empty() {
            primary_label = primary_label.with_message(primary)
        }
        let mut primary_labels = vec![primary_label];
        GlobalEnv::add_inlined_from_labels(&mut primary_labels, &loc.inlined_from_loc);

        // add "inlined from" qualifiers to secondary labels as needed
        let labels = labels
            .into_iter()
            .map(|(loc, msg)| {
                let mut expanded_labels =
                    vec![Label::secondary(loc.file_id, loc.span).with_message(msg)];
                GlobalEnv::add_inlined_from_labels(&mut expanded_labels, &loc.inlined_from_loc);
                expanded_labels
            })
            .concat();

        self.add_diag(
            Diagnostic::new(severity)
                .with_message(new_msg)
                .with_labels(primary_labels)
                .with_labels(labels)
                .with_notes(notes),
        )
    }

    /// Checks whether any of the diagnostics contains string.
    pub fn has_diag(&self, pattern: &str) -> bool {
        self.diags
            .borrow()
            .iter()
            .any(|(d, _)| d.message.contains(pattern))
    }

    /// Clear all accumulated diagnosis.
    pub fn clear_diag(&self) {
        self.diags.borrow_mut().clear();
    }

    /// Returns the unknown location.
    pub fn unknown_loc(&self) -> Loc {
        self.unknown_loc.clone()
    }

    /// Returns a Move IR version of the unknown location which is guaranteed to map to the
    /// regular unknown location via `to_loc`.
    pub fn unknown_move_ir_loc(&self) -> MoveIrLoc {
        self.unknown_move_ir_loc
    }

    /// Returns the internal location.
    pub fn internal_loc(&self) -> Loc {
        self.internal_loc.clone()
    }

    /// Converts a Loc as used by the move compiler to the one we are using here.
    pub fn to_loc(&self, loc: &MoveIrLoc) -> Loc {
        if let Some(file_id) = self.get_file_id(loc.file_hash()) {
            // Note that move-compiler doesn't use "inlined from"
            Loc::new(file_id, Span::new(loc.start(), loc.end()))
        } else {
            // Cannot map this location, return unknown loc
            self.unknown_loc.clone()
        }
    }

    /// Converts a location back into a MoveIrLoc. If the location is not convertible, unknown
    /// location will be returned.
    pub fn to_ir_loc(&self, loc: &Loc) -> MoveIrLoc {
        if let Some(file_hash) = self.get_file_hash(loc.file_id()) {
            MoveIrLoc::new(file_hash, loc.span().start().0, loc.span().end().0)
        } else {
            self.unknown_move_ir_loc()
        }
    }

    /// Returns the file id for a file hash, if defined.
    pub fn get_file_id(&self, fhash: FileHash) -> Option<FileId> {
        self.file_hash_map.get(&fhash).map(|(_, id)| id).cloned()
    }

    /// Returns the file hash for the file id, if defined.
    pub fn get_file_hash(&self, file_id: FileId) -> Option<FileHash> {
        self.reverse_file_hash_map.get(&file_id).cloned()
    }

    /// Maps a FileId to an index which can be mapped back to a FileId.
    pub fn file_id_to_idx(&self, file_id: FileId) -> u16 {
        *self
            .file_id_to_idx
            .get(&file_id)
            .expect("file_id undefined")
    }

    /// Maps a an index which was obtained by `file_id_to_idx` back to a FileId.
    pub fn file_idx_to_id(&self, file_idx: u16) -> FileId {
        *self
            .file_idx_to_id
            .get(&file_idx)
            .expect("file_idx undefined")
    }

    /// Returns file name and line/column position for a location, if available.
    pub fn get_file_and_location(&self, loc: &Loc) -> Option<(String, Location)> {
        self.get_location(loc).map(|line_column| {
            (
                self.source_files
                    .name(loc.file_id())
                    .to_string_lossy()
                    .to_string(),
                line_column,
            )
        })
    }

    /// Returns line/column position for a location, if available.
    pub fn get_location(&self, loc: &Loc) -> Option<Location> {
        self.source_files
            .location(loc.file_id(), loc.span().start())
            .ok()
    }

    /// Return the source text for the given location.
    pub fn get_source(&self, loc: &Loc) -> Result<&str, codespan_reporting::files::Error> {
        self.source_files.source_slice(loc.file_id, loc.span)
    }

    /// Return the source text for the given file.
    pub fn get_file_source(&self, id: FileId) -> &str {
        self.source_files.source(id)
    }

    /// Return the source file name for `file_id`
    pub fn get_file(&self, file_id: FileId) -> &OsStr {
        self.source_files.name(file_id)
    }

    /// Return the source file names.
    pub fn get_source_file_names(&self) -> Vec<String> {
        self.file_hash_map
            .iter()
            .filter_map(|(_, (k, _))| {
                if k.eq("<internal>") || k.eq("<unknown>") {
                    None
                } else {
                    Some(k.clone())
                }
            })
            .collect()
    }

    /// Return the source file ids.
    pub fn get_source_file_ids(&self) -> Vec<FileId> {
        self.file_hash_map
            .iter()
            .filter_map(|(_, (k, id))| {
                if k.eq("<internal>") || k.eq("<unknown>") {
                    None
                } else {
                    Some(*id)
                }
            })
            .collect()
    }

    // Gets the number of source files in this environment.
    pub fn get_file_count(&self) -> usize {
        self.file_hash_map.len()
    }

    /// Returns true if diagnostics have error severity or worse.
    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    /// Returns the number of diagnostics.
    pub fn diag_count(&self, min_severity: Severity) -> usize {
        self.diags
            .borrow()
            .iter()
            .filter(|(d, _)| d.severity >= min_severity)
            .count()
    }

    /// Returns the number of errors.
    pub fn error_count(&self) -> usize {
        self.diag_count(Severity::Error)
    }

    /// Returns true if diagnostics have warning severity or worse.
    pub fn has_warnings(&self) -> bool {
        self.diags
            .borrow()
            .iter()
            .any(|(d, _)| d.severity >= Severity::Warning)
    }

    /// Writes accumulated diagnostics of given or higher severity.
    pub fn report_diag<W: WriteColor>(&self, writer: &mut W, severity: Severity) {
        self.report_diag_with_filter(
            |files, diag| {
                emit(writer, &Config::default(), files, diag).expect("emit must not fail")
            },
            |d| d.severity >= severity,
        );
    }

    /// Helper function to report diagnostics, check for errors, and fail with a message on
    /// errors. This function is idempotent and will not report the same diagnostics again.
    pub fn check_diag<W>(
        &self,
        error_writer: &mut W,
        report_severity: Severity,
        msg: &str,
    ) -> anyhow::Result<()>
    where
        W: WriteColor + std::io::Write,
    {
        self.report_diag(error_writer, report_severity);
        if self.has_errors() {
            bail!("exiting with {}", msg);
        } else {
            Ok(())
        }
    }

    // Comparison of Diagnostic values that tries to match program ordering so we
    // can display them to the user in a more natural order.
    fn cmp_diagnostic(diag1: &Diagnostic<FileId>, diag2: &Diagnostic<FileId>) -> Ordering {
        let labels_ordering = GlobalEnv::cmp_labels(&diag1.labels, &diag2.labels);
        if Ordering::Equal == labels_ordering {
            let sev_ordering = diag1
                .severity
                .partial_cmp(&diag2.severity)
                .expect("Severity provides a total ordering for valid severity enum values");
            if Ordering::Equal == sev_ordering {
                let message_ordering = diag1.message.cmp(&diag2.message);
                if Ordering::Equal == message_ordering {
                    diag1.code.cmp(&diag2.code)
                } else {
                    message_ordering
                }
            } else {
                sev_ordering
            }
        } else {
            labels_ordering
        }
    }

    // Label comparison that tries to match program ordering.  `FileId` is already set in visitation
    // order, so we honor that.  Within the same file, we order by labelled code ranges.  For labels
    // marking nested regions, we want the innermost region, so we order first by end of labelled
    // code region, then in reverse by start of region.
    fn cmp_label(label1: &Label<FileId>, label2: &Label<FileId>) -> Ordering {
        let file_ordering = label1.file_id.cmp(&label2.file_id);
        if Ordering::Equal == file_ordering {
            // First order by end of region.
            let end1 = label1.range.end;
            let end2 = label2.range.end;
            let end_ordering = end1.cmp(&end2);
            if Ordering::Equal == end_ordering {
                let start1 = label1.range.start;
                let start2 = label2.range.start;

                // For nested regions with same end, show inner-most region first.
                // Swap 1 and 2 in comparing starts.
                start2.cmp(&start1)
            } else {
                end_ordering
            }
        } else {
            file_ordering
        }
    }

    // Label comparison within a list of labels for a given diagnostic, which orders by priority
    // first, then files and line numbers.
    fn cmp_label_priority(label1: &Label<FileId>, label2: &Label<FileId>) -> Ordering {
        use LabelStyle::*;
        match (label1.style, label2.style) {
            (Primary, Secondary) => Ordering::Less,
            (Secondary, Primary) => Ordering::Greater,
            (_, _) => GlobalEnv::cmp_label(label1, label2),
        }
    }

    // Comparison for sets of labels that orders them based on program ordering, using
    // the earliest label found.  If a `Primary` label is found then `Secondary` labels
    // are ignored, but if all are `Secondary` then the earliest of those is used in
    // the ordering.
    fn cmp_labels(labels1: &[Label<FileId>], labels2: &[Label<FileId>]) -> Ordering {
        let mut sorted_labels1 = labels1.iter().collect_vec();
        sorted_labels1.sort_by(|l1, l2| GlobalEnv::cmp_label_priority(l1, l2));
        let mut sorted_labels2 = labels2.iter().collect_vec();
        sorted_labels2.sort_by(|l1, l2| GlobalEnv::cmp_label_priority(l1, l2));
        std::iter::zip(sorted_labels1, sorted_labels2)
            .map(|(l1, l2)| GlobalEnv::cmp_label(l1, l2))
            .find(|r| Ordering::Equal != *r)
            .unwrap_or(Ordering::Equal)
    }

    /// Writes accumulated diagnostics that pass through `filter`
    pub fn report_diag_with_filter<E, F>(&self, mut emitter: E, mut filter: F)
    where
        E: FnMut(&Files<String>, &Diagnostic<FileId>),
        F: FnMut(&Diagnostic<FileId>) -> bool,
    {
        let mut shown = BTreeSet::new();
        self.diags.borrow_mut().sort_by(|a, b| {
            let reported_ordering = a.1.cmp(&b.1);
            if Ordering::Equal == reported_ordering {
                GlobalEnv::cmp_diagnostic(&a.0, &b.0)
            } else {
                reported_ordering
            }
        });
        for (diag, reported) in self.diags.borrow_mut().iter_mut().filter(|(d, reported)| {
            !reported
                && filter(d)
                && (d.severity >= Severity::Error
                    || d.labels
                        .iter()
                        .any(|label| self.file_id_is_primary_target.contains(&label.file_id)))
        }) {
            if !*reported {
                // Avoid showing the same message twice. This can happen e.g. because of
                // duplication of expressions via schema inclusion.
                if shown.insert(format!("{:?}", diag)) {
                    emitter(&self.source_files, diag);
                }
                *reported = true;
            }
        }
    }

    /// Adds a global invariant to this environment.
    pub fn add_global_invariant(&mut self, inv: GlobalInvariant) {
        let id = inv.id;
        for memory in &inv.mem_usage {
            self.global_invariants_for_memory
                .entry(memory.clone())
                .or_default()
                .insert(id);
        }
        self.global_invariants.insert(id, inv);
    }

    /// Get global invariant by id.
    pub fn get_global_invariant(&self, id: GlobalId) -> Option<&GlobalInvariant> {
        self.global_invariants.get(&id)
    }

    /// Return the global invariants which refer to the given memory.
    pub fn get_global_invariants_for_memory(
        &self,
        memory: &QualifiedInstId<StructId>,
    ) -> BTreeSet<GlobalId> {
        let mut inv_ids = BTreeSet::new();
        for (key, val) in &self.global_invariants_for_memory {
            if key.module_id != memory.module_id || key.id != memory.id {
                continue;
            }
            assert_eq!(key.inst.len(), memory.inst.len());
            let adapter = TypeUnificationAdapter::new_vec(&memory.inst, &key.inst, true, true);
            let rel = adapter.unify(&mut NoUnificationContext, Variance::SpecVariance, true);
            if rel.is_some() {
                inv_ids.extend(val.clone());
            }
        }
        inv_ids
    }

    pub fn get_global_invariants_for_module(&self, module_id: ModuleId) -> Vec<&GlobalInvariant> {
        self.global_invariants
            .iter()
            .filter(|(_, inv)| inv.declaring_module == module_id)
            .map(|(_, inv)| inv)
            .collect()
    }

    pub fn get_global_invariants_by_module(&self, module_id: ModuleId) -> BTreeSet<GlobalId> {
        self.global_invariants
            .iter()
            .filter(|(_, inv)| inv.declaring_module == module_id)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns true if a spec fun is used in specs.
    pub fn is_spec_fun_used(&self, id: QualifiedId<SpecFunId>) -> bool {
        self.used_spec_funs.contains(&id)
    }

    /// Marks a spec fun to be used
    pub fn add_used_spec_fun(&mut self, id: QualifiedId<SpecFunId>) {
        self.used_spec_funs.insert(id);
    }

    /// Determines whether the given spec fun is recursive.
    pub fn is_spec_fun_recursive(&self, id: QualifiedId<SpecFunId>) -> bool {
        fn is_caller(
            env: &GlobalEnv,
            visited: &mut BTreeSet<QualifiedId<SpecFunId>>,
            caller: QualifiedId<SpecFunId>,
            fun: QualifiedId<SpecFunId>,
        ) -> bool {
            if !visited.insert(caller) {
                return false;
            }
            let module = env.get_module(caller.module_id);
            let decl = module.get_spec_fun(caller.id);
            decl.callees.iter().any(|c| c.to_qualified_id() == fun)
                || decl.callees.iter().any(|trans_caller| {
                    is_caller(env, visited, trans_caller.to_qualified_id(), fun)
                })
        }
        let module = self.get_module(id.module_id);
        let is_recursive = *module.get_spec_fun(id.id).is_recursive.borrow();
        if let Some(b) = is_recursive {
            b
        } else {
            let b = is_caller(self, &mut BTreeSet::new(), id, id);
            *module.get_spec_fun(id.id).is_recursive.borrow_mut() = Some(b);
            b
        }
    }

    /// Determines whether the given spec fun uses type reflection over type parameters.
    pub fn spec_fun_uses_generic_type_reflection(
        &self,
        fun_id: &QualifiedInstId<SpecFunId>,
    ) -> bool {
        fn uses_generic_type_reflection(
            env: &GlobalEnv,
            visited: &mut BTreeSet<QualifiedInstId<SpecFunId>>,
            fun: &QualifiedInstId<SpecFunId>,
        ) -> bool {
            if !visited.insert(fun.clone()) {
                return false;
            }
            let module = env.get_module(fun.module_id);
            let decl = module.get_spec_fun(fun.id);
            if let Some(def) = &decl.body {
                // Check called spec funs
                def.called_spec_funs(env).into_iter().any(|qid| {
                    let qid_inst = qid.instantiate(&fun.inst);
                    is_generic_type_reflection(env, &qid_inst)
                        || uses_generic_type_reflection(env, visited, &qid_inst)
                })
            } else {
                false
            }
        }
        fn is_generic_type_reflection(
            env: &GlobalEnv,
            fun_id: &QualifiedInstId<SpecFunId>,
        ) -> bool {
            static REFLECTION_FUNS: Lazy<BTreeSet<String>> = Lazy::new(|| {
                [
                    well_known::TYPE_INFO_SPEC.to_owned(),
                    well_known::TYPE_NAME_SPEC.to_owned(),
                    well_known::TYPE_NAME_GET_SPEC.to_owned(),
                ]
                .into_iter()
                .collect()
            });
            // The function must be at `extlib` or `stdlib`.
            let module = env.get_module(fun_id.module_id);
            let addr = module.get_name().addr();
            if addr == &env.get_extlib_address() || addr == &env.get_stdlib_address() {
                let fun = module.get_spec_fun(fun_id.id);
                let name = format!(
                    "{}::{}",
                    module.get_name().name().display(module.symbol_pool()),
                    fun.name.display(module.symbol_pool())
                );
                REFLECTION_FUNS.contains(&name)
                    && fun_id.inst.iter().any(|ty| ty.is_type_parameter())
            } else {
                false
            }
        }
        let module = self.get_module(fun_id.module_id);
        let decl = module.get_spec_fun(fun_id.id);
        let uses = decl
            .insts_using_generic_type_reflection
            .borrow()
            .get(&fun_id.inst)
            .cloned();
        if let Some(b) = uses {
            b
        } else {
            let b = uses_generic_type_reflection(self, &mut BTreeSet::new(), fun_id);
            module
                .get_spec_fun(fun_id.id)
                .insts_using_generic_type_reflection
                .borrow_mut()
                .insert(fun_id.inst.clone(), b);
            b
        }
    }

    /// Returns true if the type represents the well-known event handle type.
    pub fn is_wellknown_event_handle_type(&self, ty: &Type) -> bool {
        if let Type::Struct(mid, sid, _) = ty {
            let module_env = self.get_module(*mid);
            let struct_env = module_env.get_struct(*sid);
            let module_name = module_env.get_name();
            module_name.addr() == &Address::Numerical(AccountAddress::ONE)
                && &*self.symbol_pool.string(module_name.name()) == "event"
                && &*self.symbol_pool.string(struct_env.get_name()) == "EventHandle"
        } else {
            false
        }
    }

    /// Computes the abilities associated with the given type.
    pub fn type_abilities(&self, ty: &Type, ty_params: &[TypeParameter]) -> AbilitySet {
        AbilityInferer::new(self, ty_params).infer_abilities(ty).1
    }

    /// Returns associated intrinsics.
    pub fn get_intrinsics(&self) -> &IntrinsicsAnnotation {
        &self.intrinsics
    }

    /// Adds a new module to the environment.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        loc: Loc,
        name: ModuleName,
        attributes: Vec<Attribute>,
        use_decls: Vec<UseDecl>,
        friend_decls: Vec<FriendDecl>,
        named_constants: BTreeMap<NamedConstantId, NamedConstantData>,
        mut struct_data: BTreeMap<StructId, StructData>,
        function_data: BTreeMap<FunId, FunctionData>,
        spec_vars: Vec<SpecVarDecl>,
        spec_funs: Vec<SpecFunDecl>,
        module_spec: Spec,
        spec_block_infos: Vec<SpecBlockInfo>,
    ) -> ModuleId {
        let spec_vars: BTreeMap<SpecVarId, SpecVarDecl> = spec_vars
            .into_iter()
            .enumerate()
            .map(|(i, v)| (SpecVarId::new(i), v))
            .collect();
        // Generate ghost memory struct declarations for spec vars.
        for (svar_id, svar) in &spec_vars {
            let data = self.create_ghost_struct_data(
                svar.loc.clone(),
                svar.name,
                *svar_id,
                svar.type_params.clone(),
                svar.type_.clone(),
            );
            struct_data.insert(StructId::new(data.name), data);
        }
        let spec_funs: BTreeMap<SpecFunId, SpecFunDecl> = spec_funs
            .into_iter()
            .enumerate()
            .map(|(i, v)| (SpecFunId::new(i), v))
            .collect();

        let id = ModuleId(self.module_data.len() as RawIndex);
        let used_modules = use_decls.iter().filter_map(|ud| ud.module_id).collect();
        self.module_data.push(ModuleData {
            name,
            id,
            compiled_module: None,
            source_map: None,
            named_constants,
            struct_data,
            struct_idx_to_id: Default::default(),
            function_data,
            function_idx_to_id: Default::default(),
            spec_vars,
            spec_funs,
            module_spec: RefCell::new(module_spec),
            loc,
            attributes,
            use_decls,
            friend_decls,
            spec_block_infos,
            used_modules,
            used_modules_including_specs: Default::default(),
            friend_modules: Default::default(),
        });
        id
    }

    /// Attaches a bytecode module to the module in the environment. This functions expects
    /// the `self.module_data[module_id]` to be already initialized using the `self.add`
    /// function.
    pub fn attach_compiled_module(
        &mut self,
        module_id: ModuleId,
        module: CompiledModule,
        source_map: SourceMap,
    ) {
        {
            let mod_data = &mut self.module_data[module_id.0 as usize];
            mod_data.struct_idx_to_id.clear();
            mod_data.function_idx_to_id.clear();
        }

        // Attach indices pointing into the compiled module to function and struct data
        for idx in 0..module.struct_defs.len() {
            let def_idx = StructDefinitionIndex(idx as u16);
            let handle_idx = module.struct_def_at(def_idx).struct_handle;
            let handle = module.struct_handle_at(handle_idx);
            let view = StructHandleView::new(&module, handle);
            let struct_id = StructId(self.symbol_pool.make(view.name().as_str()));
            let mod_data = &mut self.module_data[module_id.0 as usize];
            if let Some(struct_data) = mod_data.struct_data.get_mut(&struct_id) {
                struct_data.def_idx = Some(def_idx);
                mod_data.struct_idx_to_id.insert(def_idx, struct_id);
            } else {
                panic!("attaching mismatching bytecode module")
            }
        }
        for idx in 0..module.function_defs.len() {
            let def_idx = FunctionDefinitionIndex(idx as u16);
            let handle_idx = module.function_def_at(def_idx).function;
            let handle = module.function_handle_at(handle_idx);
            let view = FunctionHandleView::new(&module, handle);
            let name_str = view.name().as_str();
            let fun_id = if name_str.starts_with(SCRIPT_BYTECODE_FUN_NAME) {
                // This is a pseudo script module, which has exactly one function. Determine
                // the name of this function.
                let mod_data = &self.module_data[module_id.0 as usize];
                *mod_data
                    .function_data
                    .iter()
                    .next()
                    .expect("script has function")
                    .0
            } else {
                FunId(self.symbol_pool.make(name_str))
            };

            // While releasing any mutation, compute the used/called functions if needed.
            let fun_data = &self.module_data[module_id.to_usize()]
                .function_data
                .get(&fun_id)
                .unwrap();
            let used_funs = if fun_data.used_funs.is_none() {
                Some(self.get_used_funs_from_bytecode(&module, def_idx))
            } else {
                None
            };
            let called_funs = if fun_data.called_funs.is_none() {
                Some(self.get_called_funs_from_bytecode(&module, def_idx))
            } else {
                None
            };

            let mod_data = &mut self.module_data[module_id.0 as usize];
            if let Some(fun_data) = mod_data.function_data.get_mut(&fun_id) {
                fun_data.def_idx = Some(def_idx);
                fun_data.handle_idx = Some(handle_idx);
                mod_data.function_idx_to_id.insert(def_idx, fun_id);
                if let Some(used_funs) = used_funs {
                    fun_data.used_funs = Some(used_funs);
                }
                if let Some(called_funs) = called_funs {
                    fun_data.called_funs = Some(called_funs);
                }
            } else {
                panic!("attaching mismatching bytecode module")
            }
        }
        let used_modules = self.get_used_modules_from_bytecode(&module);
        let friend_modules = self.get_friend_modules_from_bytecode(&module);
        let mod_data = &mut self.module_data[module_id.0 as usize];
        mod_data.used_modules = used_modules;
        mod_data.friend_modules = friend_modules;
        mod_data.compiled_module = Some(module);
        mod_data.source_map = Some(source_map);
    }

    fn get_used_funs_from_bytecode(
        &self,
        module: &CompiledModule,
        def_idx: FunctionDefinitionIndex,
    ) -> BTreeSet<QualifiedId<FunId>> {
        let function_definition = module.function_def_at(def_idx);
        let function_definition_view = FunctionDefinitionView::new(module, function_definition);
        let used_funs: BTreeSet<QualifiedId<FunId>> = match function_definition_view.code() {
            Some(unit) => unit
                .code
                .iter()
                .filter_map(|c| {
                    let handle_idx = match c {
                        Bytecode::Call(i) | Bytecode::PackClosure(i, ..) => Some(*i),
                        Bytecode::CallGeneric(i) | Bytecode::PackClosureGeneric(i, ..) => {
                            Some(module.function_instantiation_at(*i).handle)
                        },
                        _ => None,
                    };
                    handle_idx.map(|idx| {
                        ModuleEnv::get_used_function_from_compiled_module(self, idx, module)
                            .get_qualified_id()
                    })
                })
                .collect(),
            None => BTreeSet::default(),
        };
        used_funs
    }

    fn get_called_funs_from_bytecode(
        &self,
        module: &CompiledModule,
        def_idx: FunctionDefinitionIndex,
    ) -> BTreeSet<QualifiedId<FunId>> {
        let function_definition = module.function_def_at(def_idx);
        let function_definition_view = FunctionDefinitionView::new(module, function_definition);
        let called_funs: BTreeSet<QualifiedId<FunId>> = match function_definition_view.code() {
            Some(unit) => unit
                .code
                .iter()
                .filter_map(|c| {
                    let handle_idx = match c {
                        Bytecode::Call(i) => Some(*i),
                        Bytecode::CallGeneric(i) => {
                            Some(module.function_instantiation_at(*i).handle)
                        },
                        _ => None,
                    };
                    handle_idx.map(|idx| {
                        ModuleEnv::get_used_function_from_compiled_module(self, idx, module)
                            .get_qualified_id()
                    })
                })
                .collect(),
            None => BTreeSet::default(),
        };
        called_funs
    }

    #[allow(unused)]
    fn get_used_modules_from_bytecode(
        &self,
        compiled_module: &CompiledModule,
    ) -> BTreeSet<ModuleId> {
        compiled_module
            .immediate_dependencies()
            .into_iter()
            .map(|storage_id| self.to_module_name(&storage_id))
            .filter_map(|name| self.find_module(&name))
            .map(|module_env| module_env.get_id())
            .collect()
    }

    #[allow(unused)]
    fn get_friend_modules_from_bytecode(
        &self,
        compiled_module: &CompiledModule,
    ) -> BTreeSet<ModuleId> {
        compiled_module
            .immediate_friends()
            .into_iter()
            .map(|storage_id| self.to_module_name(&storage_id))
            .flat_map(|name| self.find_module(&name))
            .map(|module_env| module_env.get_id())
            .collect()
    }

    /// Return the name of the ghost memory associated with spec var.
    pub fn ghost_memory_name(&self, spec_var_name: Symbol) -> Symbol {
        self.symbol_pool.make(&format!(
            "{}{}",
            GHOST_MEMORY_PREFIX,
            self.symbol_pool.string(spec_var_name)
        ))
    }

    /// Create a ghost memory struct declaration.
    fn create_ghost_struct_data(
        &self,
        loc: Loc,
        var_name: Symbol,
        var_id: SpecVarId,
        type_params: Vec<TypeParameter>,
        ty: Type,
    ) -> StructData {
        let field_name = self.symbol_pool.make("v");
        let mut field_data = BTreeMap::new();
        let field_id = FieldId::new(field_name);
        field_data.insert(field_id, FieldData {
            name: field_name,
            loc: loc.clone(),
            offset: 0,
            variant: None,
            ty,
        });
        StructData {
            name: self.ghost_memory_name(var_name),
            loc,
            def_idx: None,
            attributes: Default::default(),
            type_params,
            abilities: AbilitySet::ALL,
            spec_var_opt: Some(var_id),
            field_data,
            variants: None,
            spec: RefCell::new(Spec::default()),
            is_native: false,
        }
    }

    /// Finds a module by name and returns an environment for it.
    pub fn find_module(&self, name: &ModuleName) -> Option<ModuleEnv<'_>> {
        for module_data in &self.module_data {
            let module_env = ModuleEnv {
                env: self,
                data: module_data,
            };
            if module_env.get_name() == name {
                return Some(module_env);
            }
        }
        None
    }

    /// Finds a module by simple name and returns an environment for it.
    /// TODO: we may need to disallow this to support modules of the same simple name but with
    ///    different addresses in one verification session.
    pub fn find_module_by_name(&self, simple_name: Symbol) -> Option<ModuleEnv<'_>> {
        self.get_modules()
            .find(|m| m.get_name().name() == simple_name)
    }

    /// Find a module by its bytecode format ID
    pub fn find_module_by_language_storage_id(
        &self,
        id: &language_storage::ModuleId,
    ) -> Option<ModuleEnv<'_>> {
        self.find_module(&self.to_module_name(id))
    }

    /// Find a function by its bytecode format name and ID
    pub fn find_function_by_language_storage_id_name(
        &self,
        id: &language_storage::ModuleId,
        name: &IdentStr,
    ) -> Option<FunctionEnv<'_>> {
        self.find_module_by_language_storage_id(id)
            .and_then(|menv| menv.find_function(menv.symbol_pool().make(name.as_str())))
    }

    /// Gets a StructEnv in this module by its `StructTag`
    pub fn find_struct_by_tag(
        &self,
        tag: &language_storage::StructTag,
    ) -> Option<QualifiedId<StructId>> {
        self.find_module(&self.to_module_name(&tag.module_id()))
            .and_then(|menv| {
                menv.find_struct_by_identifier(tag.name.clone())
                    .map(|sid| menv.get_id().qualified(sid))
            })
    }

    /// Return the module enclosing this location.
    pub fn get_enclosing_module(&self, loc: &Loc) -> Option<ModuleEnv<'_>> {
        for data in &self.module_data {
            if data.loc.file_id() == loc.file_id()
                && Self::enclosing_span(data.loc.span(), loc.span())
            {
                return Some(ModuleEnv { env: self, data });
            }
        }
        None
    }

    /// Returns the function enclosing this location.
    pub fn get_enclosing_function(&self, loc: &Loc) -> Option<FunctionEnv<'_>> {
        // Currently we do a brute-force linear search, may need to speed this up if it appears
        // to be a bottleneck.
        let module_env = self.get_enclosing_module(loc)?;
        for func_env in module_env.into_functions() {
            if Self::enclosing_span(func_env.get_loc().span(), loc.span())
                || Self::enclosing_span(
                    func_env
                        .get_spec()
                        .loc
                        .clone()
                        .unwrap_or_else(|| self.unknown_loc.clone())
                        .span(),
                    loc.span(),
                )
            {
                return Some(func_env.clone());
            }
        }
        None
    }

    /// Returns the struct enclosing this location.
    pub fn get_enclosing_struct(&self, loc: &Loc) -> Option<StructEnv<'_>> {
        let module_env = self.get_enclosing_module(loc)?;
        module_env
            .into_structs()
            .find(|struct_env| Self::enclosing_span(struct_env.get_loc().span(), loc.span()))
    }

    fn enclosing_span(outer: Span, inner: Span) -> bool {
        inner.start() >= outer.start() && inner.end() <= outer.end()
    }

    /// Return the `FunctionEnv` for `fun`
    pub fn get_function(&self, fun: QualifiedId<FunId>) -> FunctionEnv<'_> {
        self.get_module(fun.module_id).into_function(fun.id)
    }

    pub fn get_function_opt(&self, fun: QualifiedId<FunId>) -> Option<FunctionEnv<'_>> {
        self.get_module_opt(fun.module_id)
            .map(|module| module.into_function(fun.id))
    }

    /// Sets the AST based definition of the function.
    pub fn set_function_def(&mut self, fun: QualifiedId<FunId>, def: Exp) {
        let data = self
            .module_data
            .get_mut(fun.module_id.to_usize())
            .unwrap()
            .function_data
            .get_mut(&fun.id)
            .unwrap();
        data.used_funs = Some(def.used_funs());
        data.called_funs = Some(def.called_funs());
        data.def = Some(def);
    }

    /// Sets the inferred acquired structs of this function.
    pub fn set_acquired_structs(&mut self, fun: QualifiedId<FunId>, acquires: BTreeSet<StructId>) {
        let data = self
            .module_data
            .get_mut(fun.module_id.to_usize())
            .unwrap()
            .function_data
            .get_mut(&fun.id)
            .unwrap();
        data.acquired_structs = Some(acquires)
    }

    /// Adds a new function definition.
    pub fn add_function_def(
        &mut self,
        module_id: ModuleId,
        name: Symbol,
        loc: Loc,
        visibility: Visibility,
        has_package_visibility: bool,
        type_params: Vec<TypeParameter>,
        params: Vec<Parameter>,
        result_type: Type,
        def: Exp,
        spec_opt: Option<Spec>,
    ) {
        let used_funs = def.used_funs();
        let called_funs = def.called_funs();
        let data = FunctionData {
            name,
            loc: FunctionLoc {
                full: loc.clone(),
                id_loc: loc.clone(),
                result_type_loc: loc,
            },
            def_idx: None,
            handle_idx: None,
            visibility,
            has_package_visibility,
            is_native: false,
            kind: FunctionKind::Regular,
            attributes: vec![],
            type_params,
            params,
            result_type,
            access_specifiers: None,
            acquired_structs: None,
            spec: RefCell::new(spec_opt.unwrap_or_default()),
            def: Some(def),
            called_funs: Some(called_funs),
            calling_funs: RefCell::new(None),
            transitive_closure_of_called_funs: RefCell::new(None),
            used_funs: Some(used_funs),
            using_funs: RefCell::new(None),
            transitive_closure_of_used_funs: RefCell::new(None),
            used_functions_with_transitive_inline: RefCell::new(None),
        };
        assert!(self
            .module_data
            .get_mut(module_id.to_usize())
            .expect("module defined")
            .function_data
            .insert(FunId::new(name), data)
            .is_none())
    }

    /// Adds a new function definition from data
    pub fn add_function_def_from_data(&mut self, module_id: ModuleId, data: FunctionData) -> FunId {
        let new_id = FunId::new(data.name);
        assert!(self
            .module_data
            .get_mut(module_id.to_usize())
            .expect("module defined")
            .function_data
            .insert(FunId::new(data.name), data)
            .is_none());
        new_id
    }

    /// Constructs function data
    pub fn construct_function_data(
        &self,
        name: Symbol,
        loc: Loc,
        visibility: Visibility,
        has_package_visibility: bool,
        type_params: Vec<TypeParameter>,
        params: Vec<Parameter>,
        result_type: Type,
        def: Exp,
        spec_opt: Option<Spec>,
    ) -> FunctionData {
        let used_funs = def.used_funs();
        let called_funs = def.called_funs();
        FunctionData {
            name,
            loc: FunctionLoc {
                full: loc.clone(),
                id_loc: loc.clone(),
                result_type_loc: loc,
            },
            def_idx: None,
            handle_idx: None,
            visibility,
            has_package_visibility,
            is_native: false,
            kind: FunctionKind::Regular,
            attributes: vec![],
            type_params,
            params,
            result_type,
            access_specifiers: None,
            acquired_structs: None,
            spec: RefCell::new(spec_opt.unwrap_or_default()),
            def: Some(def),
            called_funs: Some(called_funs),
            calling_funs: RefCell::new(None),
            transitive_closure_of_called_funs: RefCell::new(None),
            used_funs: Some(used_funs),
            using_funs: RefCell::new(None),
            transitive_closure_of_used_funs: RefCell::new(None),
            used_functions_with_transitive_inline: RefCell::new(None),
        }
    }

    /// Returns a reference to the declaration of a spec fun.
    pub fn get_spec_fun(&self, fun: QualifiedId<SpecFunId>) -> &SpecFunDecl {
        self.module_data
            .get(fun.module_id.to_usize())
            .unwrap()
            .spec_funs
            .get(&fun.id)
            .unwrap()
    }

    /// Returns a mutable reference to the declaration of a spec fun.
    pub fn get_spec_fun_mut(&mut self, fun: QualifiedId<SpecFunId>) -> &mut SpecFunDecl {
        self.module_data
            .get_mut(fun.module_id.to_usize())
            .unwrap()
            .spec_funs
            .get_mut(&fun.id)
            .unwrap()
    }

    /// Adds a new specification function and returns id of it.
    pub fn add_spec_function_def(
        &mut self,
        module_id: ModuleId,
        decl: SpecFunDecl,
    ) -> QualifiedId<SpecFunId> {
        let spec_funs = &mut self
            .module_data
            .get_mut(module_id.to_usize())
            .unwrap()
            .spec_funs;
        let id = SpecFunId::new(spec_funs.len());
        assert!(spec_funs.insert(id, decl).is_none());
        module_id.qualified(id)
    }

    /// Gets the spec block associated with the spec block target. Only
    /// module, struct, and function specs are supported.
    pub fn get_spec_block(&self, target: &SpecBlockTarget) -> Ref<Spec> {
        use SpecBlockTarget::*;
        match target {
            Module(mid) => self.module_data[mid.to_usize()].module_spec.borrow(),
            Struct(mid, sid) => self.module_data[mid.to_usize()]
                .struct_data
                .get(sid)
                .unwrap()
                .spec
                .borrow(),
            Function(mid, fid) => self.module_data[mid.to_usize()]
                .function_data
                .get(fid)
                .unwrap()
                .spec
                .borrow(),
            SpecFunction(mid, fid) => self.get_spec_fun(mid.qualified(*fid)).spec.borrow(),
            FunctionCode(..) | Schema(_, _, _) | Inline => {
                // Schemas are expanded, inline spec blocks are part of the AST,
                // and function code is nested inside of a function spec block
                panic!("spec not available for schema or inline blocks")
            },
        }
    }

    /// Gets the spec block associated with the spec block target. Only
    /// module, struct, and function specs are supported.
    pub fn get_spec_block_mut(&self, target: &SpecBlockTarget) -> RefMut<Spec> {
        use SpecBlockTarget::*;
        match target {
            Module(mid) => self.module_data[mid.to_usize()].module_spec.borrow_mut(),
            Struct(mid, sid) => self.module_data[mid.to_usize()]
                .struct_data
                .get(sid)
                .unwrap()
                .spec
                .borrow_mut(),
            Function(mid, fid) => self.module_data[mid.to_usize()]
                .function_data
                .get(fid)
                .unwrap()
                .spec
                .borrow_mut(),
            SpecFunction(mid, fid) => self.get_spec_fun(mid.qualified(*fid)).spec.borrow_mut(),
            FunctionCode(..) | Schema(_, _, _) | Inline => {
                // Schemas are expanded, inline spec blocks are part of the AST,
                // and function code is nested inside of a function spec block
                panic!("spec not available for schema or inline blocks")
            },
        }
    }

    /// Return the `StructEnv` for `str`
    pub fn get_struct(&self, str: QualifiedId<StructId>) -> StructEnv {
        self.get_module(str.module_id).into_struct(str.id)
    }

    // Gets the number of modules in this environment.
    pub fn get_module_count(&self) -> usize {
        self.module_data.len()
    }

    /// Gets a module by id.
    pub fn get_module(&self, id: ModuleId) -> ModuleEnv<'_> {
        let module_data = &self.module_data[id.0 as usize];
        ModuleEnv {
            env: self,
            data: module_data,
        }
    }

    /// Gets a module by id.
    pub fn get_module_opt(&self, id: ModuleId) -> Option<ModuleEnv<'_>> {
        let module_data = self.module_data.get(id.0 as usize);
        module_data.map(|module_data| ModuleEnv {
            env: self,
            data: module_data,
        })
    }

    pub(crate) fn get_module_data_mut(&mut self, id: ModuleId) -> &mut ModuleData {
        &mut self.module_data[id.0 as usize]
    }

    /// Gets a struct by qualified id.
    pub fn get_struct_qid(&self, qid: QualifiedId<StructId>) -> StructEnv<'_> {
        self.get_module(qid.module_id).into_struct(qid.id)
    }

    /// Gets a function by qualified id.
    pub fn get_function_qid(&self, qid: QualifiedId<FunId>) -> FunctionEnv<'_> {
        self.get_module(qid.module_id).into_function(qid.id)
    }

    /// Returns an iterator for all modules in the environment.
    pub fn get_modules(&self) -> impl Iterator<Item = ModuleEnv<'_>> {
        self.module_data.iter().map(move |module_data| ModuleEnv {
            env: self,
            data: module_data,
        })
    }

    /// Returns all structs in all modules which carry invariants.
    pub fn get_all_structs_with_conditions(&self) -> Vec<Type> {
        let mut res = vec![];
        for module_env in self.get_modules() {
            for struct_env in module_env.get_structs() {
                if struct_env.has_conditions() {
                    let formals = struct_env
                        .get_type_parameters()
                        .iter()
                        .enumerate()
                        .map(|(idx, _)| Type::new_param(idx))
                        .collect_vec();
                    res.push(Type::Struct(
                        module_env.get_id(),
                        struct_env.get_id(),
                        formals,
                    ));
                }
            }
        }
        res
    }

    /// Converts a storage module id into an AST module name.
    pub fn to_module_name(&self, storage_id: &language_storage::ModuleId) -> ModuleName {
        ModuleName::from_str(
            &storage_id.address().to_hex(),
            self.symbol_pool.make(storage_id.name().as_str()),
        )
    }

    /// Get documentation associated with an item at Loc.
    pub fn get_doc(&self, loc: &Loc) -> &str {
        self.doc_comments
            .get(&loc.file_id)
            .and_then(|comments| comments.get(&loc.span.start()).map(|s| s.as_str()))
            .unwrap_or("")
    }

    /// Returns true if the boolean property is true.
    pub fn is_property_true(&self, properties: &PropertyBag, name: &str) -> Option<bool> {
        let sym = &self.symbol_pool().make(name);
        if let Some(PropertyValue::Value(Value::Bool(b))) = properties.get(sym) {
            return Some(*b);
        }
        None
    }

    /// Returns the value of a number property.
    pub fn get_num_property(&self, properties: &PropertyBag, name: &str) -> Option<usize> {
        let sym = &self.symbol_pool().make(name);
        if let Some(PropertyValue::Value(Value::Number(n))) = properties.get(sym) {
            return n.to_usize();
        }
        None
    }

    /// Attempt to compute a struct tag for (`mid`, `sid`, `ts`). Returns `Some` if all types in
    /// `ts` are closed, `None` otherwise
    pub fn get_struct_tag(
        &self,
        mid: ModuleId,
        sid: StructId,
        ts: &[Type],
    ) -> Option<language_storage::StructTag> {
        self.get_struct_type(mid, sid, ts)?.into_struct_tag()
    }

    /// Attempt to compute a struct type for (`mid`, `sid`, `ts`).
    #[allow(deprecated)]
    pub fn get_struct_type(&self, mid: ModuleId, sid: StructId, ts: &[Type]) -> Option<MType> {
        let menv = self.get_module(mid);
        Some(MType::Struct {
            address: (if let Address::Numerical(addr) = *menv.self_address() {
                Some(addr)
            } else {
                None
            })?,
            module: menv.get_identifier()?,
            name: menv.get_struct(sid).get_identifier()?,
            type_arguments: ts
                .iter()
                .map(|t| t.clone().into_normalized_type(self).unwrap())
                .collect(),
        })
    }

    /// Gets the location of the given node.
    pub fn get_node_loc(&self, node_id: NodeId) -> Loc {
        self.exp_info
            .borrow()
            .get(&node_id)
            .map_or_else(|| self.unknown_loc(), |info| info.loc.clone())
    }

    /// Gets the type of the given node.
    pub fn get_node_type(&self, node_id: NodeId) -> Type {
        self.get_node_type_opt(node_id).expect("node type defined")
    }

    /// Gets the type of the given node, if available.
    pub fn get_node_type_opt(&self, node_id: NodeId) -> Option<Type> {
        self.exp_info
            .borrow()
            .get(&node_id)
            .map(|info| info.ty.clone())
    }

    /// Converts an index into a node id.
    pub fn index_to_node_id(&self, index: usize) -> Option<NodeId> {
        let id = NodeId::new(index);
        if self.exp_info.borrow().get(&id).is_some() {
            Some(id)
        } else {
            None
        }
    }

    /// Returns the next free node number.
    pub fn next_free_node_number(&self) -> usize {
        *self.next_free_node_id.borrow()
    }

    /// Allocates a new node id.
    pub fn new_node_id(&self) -> NodeId {
        let id = NodeId::new(*self.next_free_node_id.borrow());
        let mut r = self.next_free_node_id.borrow_mut();
        *r = r.checked_add(1).expect("NodeId overflow");
        id
    }

    /// Allocates a new node id and assigns location and type to it.
    pub fn new_node(&self, loc: Loc, ty: Type) -> NodeId {
        let id = self.new_node_id();
        self.exp_info.borrow_mut().insert(id, ExpInfo::new(loc, ty));
        id
    }

    /// Allocates a new node id with same info as original.
    pub fn clone_node(&self, node_id: NodeId) -> NodeId {
        let id = self.new_node_id();
        let opt_info = self.exp_info.borrow().get(&node_id).cloned();
        if let Some(info) = opt_info {
            self.exp_info.borrow_mut().insert(id, info.clone());
        }
        id
    }

    /// Updates type for the given node id. Must have been set before.
    pub fn update_node_type(&self, node_id: NodeId, ty: Type) {
        let mut mods = self.exp_info.borrow_mut();
        let info = mods.get_mut(&node_id).expect("node exist");
        info.ty = ty;
    }

    /// Sets instantiation for the given node id. Must not have been set before.
    pub fn set_node_instantiation(&self, node_id: NodeId, instantiation: Vec<Type>) {
        let mut mods = self.exp_info.borrow_mut();
        let info = mods.get_mut(&node_id).expect("node exist");
        assert!(info.instantiation.is_none());
        info.instantiation = Some(instantiation);
    }

    /// Updates instantiation for the given node id. Must have been set before.
    pub fn update_node_instantiation(&self, node_id: NodeId, instantiation: Vec<Type>) {
        let mut mods = self.exp_info.borrow_mut();
        let info = mods.get_mut(&node_id).expect("node exist");
        assert!(info.instantiation.is_some());
        info.instantiation = Some(instantiation);
    }

    /// Gets the type parameter instantiation associated with the given node.
    pub fn get_node_instantiation(&self, node_id: NodeId) -> Vec<Type> {
        self.get_node_instantiation_opt(node_id).unwrap_or_default()
    }

    /// Gets the type parameter instantiation associated with the given node, if it is available.
    pub fn get_node_instantiation_opt(&self, node_id: NodeId) -> Option<Vec<Type>> {
        self.exp_info
            .borrow()
            .get(&node_id)
            .and_then(|info| info.instantiation.clone())
    }

    /// Gets the type parameter instantiation associated with the given node, if it is available.
    pub fn get_nodes(&self) -> Vec<NodeId> {
        (*self.exp_info.borrow()).clone().into_keys().collect_vec()
    }

    /// Return the total number of declared functions in the modules of `self`
    pub fn get_declared_function_count(&self) -> usize {
        let mut total = 0;
        for m in &self.module_data {
            total += m.function_data.len();
        }
        total
    }

    /// Return the total number of declared structs in the modules of `self`
    pub fn get_declared_struct_count(&self) -> usize {
        let mut total = 0;
        for m in &self.module_data {
            // This includes both spec and program structs.
            total += m.struct_data.len();
        }
        total
    }

    /// Return the total number of Move modules that contain specs
    pub fn get_modules_with_specs_count(&self) -> usize {
        let mut total = 0;
        for m in self.get_modules() {
            if m.has_specs() {
                total += 1
            }
        }
        total
    }

    /// Override the specification for a given module
    pub fn override_module_spec(&mut self, mid: ModuleId, spec: Spec) {
        let module_data = self
            .module_data
            .iter_mut()
            .filter(|m| m.id == mid)
            .exactly_one()
            .unwrap_or_else(|_| {
                panic!("Expect one and only one module for {:?}", mid);
            });
        *module_data.module_spec.borrow_mut() = spec;
    }

    /// Override the specification for a given function
    pub fn override_function_spec(&mut self, fid: QualifiedId<FunId>, spec: Spec) {
        let func_data = self
            .module_data
            .iter_mut()
            .filter(|m| m.id == fid.module_id)
            .flat_map(|m| {
                m.function_data
                    .iter_mut()
                    .filter(|(k, _)| **k == fid.id)
                    .map(|(_, v)| v)
            })
            .exactly_one()
            .unwrap_or_else(|_| {
                panic!("Expect one and only one function for {:?}", fid);
            });
        func_data.spec = spec.into();
    }

    /// Override the specification for a given code location
    pub fn override_inline_spec(
        &mut self,
        fid: QualifiedId<FunId>,
        code_offset: CodeOffset,
        spec: Spec,
    ) {
        let func_data = self
            .module_data
            .iter_mut()
            .filter(|m| m.id == fid.module_id)
            .flat_map(|m| {
                m.function_data
                    .iter_mut()
                    .filter(|(k, _)| **k == fid.id)
                    .map(|(_, v)| v)
            })
            .exactly_one()
            .unwrap_or_else(|_| {
                panic!("Expect one and only one function for {:?}", fid);
            });
        func_data
            .spec
            .borrow_mut()
            .on_impl
            .insert(code_offset, spec);
    }

    /// Produce a TypeDisplayContext to print types within the scope of this env
    pub fn get_type_display_ctx(&self) -> TypeDisplayContext {
        TypeDisplayContext::new(self)
    }

    /// Returns the address where the standard lib is defined.
    pub fn get_stdlib_address(&self) -> Address {
        self.stdlib_address
            .clone()
            .unwrap_or(Address::Numerical(AccountAddress::ONE))
    }

    /// Returns the address where the extensions libs are defined.
    pub fn get_extlib_address(&self) -> Address {
        self.extlib_address
            .clone()
            .unwrap_or(Address::Numerical(AccountAddress::TWO))
    }

    // Removes all functions not matching the predicate from
    //   module_data fields function_data and function_idx_to_id
    //   remaining function_data fields used_funs and using_funs
    pub fn filter_functions<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&QualifiedId<FunId>) -> bool,
    {
        for module_data in self.module_data.iter_mut() {
            let module_id = module_data.id;
            module_data
                .function_data
                .retain(|fun_id, _| predicate(&module_id.qualified(*fun_id)));
            module_data
                .function_idx_to_id
                .retain(|_, fun_id| predicate(&module_id.qualified(*fun_id)));
            module_data.function_data.values_mut().for_each(|fun_data| {
                if let Some(used_funs) = fun_data.used_funs.as_mut() {
                    used_funs.retain(|qfun_id| predicate(qfun_id))
                }
                if let Some(using_funs) = &mut *fun_data.using_funs.borrow_mut() {
                    using_funs.retain(|qfun_id| predicate(qfun_id))
                }
            });
        }
    }
}

impl Default for GlobalEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalEnv {
    pub fn dump_env(&self) -> String {
        self.internal_dump_env(false)
    }

    pub fn dump_env_all(&self) -> String {
        self.internal_dump_env(true)
    }

    pub fn internal_dump_env(&self, all: bool) -> String {
        let spool = self.symbol_pool();
        let writer = CodeWriter::new(self.internal_loc());
        for module in self.get_modules() {
            if !all && !module.is_target() {
                continue;
            }
            emitln!(writer, "module {} {{", module.get_full_name_str());
            writer.indent();
            let add_alias = |s: String, a_opt: Option<Symbol>| {
                format!(
                    "{}{}",
                    s,
                    if let Some(a) = a_opt {
                        format!(" as {}", a.display(spool))
                    } else {
                        "".to_owned()
                    }
                )
            };
            for use_decl in module.get_use_decls() {
                emitln!(
                    writer,
                    "use {}{};{}",
                    add_alias(
                        use_decl.module_name.display_full(self).to_string(),
                        use_decl.alias
                    ),
                    if !use_decl.members.is_empty() {
                        format!(
                            "::{{{}}}",
                            use_decl
                                .members
                                .iter()
                                .map(|(_, n, a)| add_alias(n.display(spool).to_string(), *a))
                                .join(", ")
                        )
                    } else {
                        "".to_owned()
                    },
                    if let Some(mid) = use_decl.module_id {
                        format!(
                            " // resolved as: {}",
                            self.get_module(mid).get_full_name_str()
                        )
                    } else {
                        "".to_owned()
                    },
                )
            }
            let module_spec = module.get_spec();
            if !module_spec.is_empty() {
                emitln!(writer, "{}", self.display(&*module_spec));
            }
            for str in module.get_structs() {
                let tctx = str.get_type_display_ctx();
                let type_params = str.get_type_parameters();
                let type_params_str = if !type_params.is_empty() {
                    format!(
                        "<{}>",
                        type_params
                            .iter()
                            .map(|p| p.0.display(spool).to_string())
                            .join(",")
                    )
                } else {
                    "".to_owned()
                };
                if str.has_variants() {
                    emitln!(
                        writer,
                        "enum {}{} {{",
                        str.get_name().display(spool),
                        type_params_str
                    );
                    writer.indent();
                    for variant in str.get_variants() {
                        emit!(writer, "{}", variant.display(spool));
                        let fields = str.get_fields_of_variant(variant).collect_vec();
                        if !fields.is_empty() {
                            emitln!(writer, " {");
                            writer.indent();
                            for fld in fields {
                                emitln!(writer, "{},", self.dump_field(&tctx, &fld))
                            }
                            writer.unindent();
                            emitln!(writer, "}")
                        } else {
                            emitln!(writer, ",")
                        }
                    }
                } else {
                    emitln!(
                        writer,
                        "struct {}{} {{",
                        str.get_name().display(spool),
                        type_params_str
                    );
                    writer.indent();
                    for fld in str.get_fields() {
                        emitln!(writer, "{},", self.dump_field(&tctx, &fld))
                    }
                }
                writer.unindent();
                emitln!(writer, "}");
                let spec = str.get_spec();
                if !spec.is_empty() {
                    emitln!(writer, "{}", self.display(&*spec))
                }
            }
            for fun in module.get_functions() {
                let tctx = fun.get_type_display_ctx();
                self.dump_fun_internal(&writer, &tctx, &fun);
            }
            for (_, fun) in module.get_spec_funs() {
                emit!(
                    writer,
                    "spec fun {}{}",
                    fun.name.display(spool),
                    self.get_fun_signature_string(
                        &module.get_type_display_ctx(),
                        &fun.type_params,
                        &fun.params,
                        &fun.result_type
                    )
                );
                if let Some(exp) = &fun.body {
                    emitln!(writer, " {");
                    writer.indent();
                    emitln!(writer, "{}", exp.display(self));
                    writer.unindent();
                    emitln!(writer, "}");
                } else {
                    emitln!(writer, ";");
                }
            }
            if module.get_verified_module().is_some() {
                emitln!(writer, "// bytecode attached");
            }
            writer.unindent();
            emitln!(writer, "}} // end {}", module.get_full_name_str())
        }
        writer.extract_result()
    }

    fn dump_field(&self, tctx: &TypeDisplayContext, fld: &FieldEnv) -> String {
        format!(
            "{}: {}",
            fld.get_name().display(tctx.env.symbol_pool()),
            fld.get_type().display(tctx)
        )
    }

    pub fn dump_fun(&self, fun: &FunctionEnv) -> String {
        let tctx = &fun.get_type_display_ctx();
        let writer = CodeWriter::new(self.internal_loc());
        self.dump_fun_internal(&writer, tctx, fun);
        writer.extract_result()
    }

    fn dump_fun_internal(&self, writer: &CodeWriter, tctx: &TypeDisplayContext, fun: &FunctionEnv) {
        emit!(writer, "{}", fun.get_header_string());
        if let Some(specs) = fun.get_access_specifiers() {
            emitln!(writer);
            writer.indent();
            for spec in specs {
                if spec.negated {
                    emit!(writer, "!")
                }
                match &spec.kind {
                    AccessSpecifierKind::Reads => emit!(writer, "reads "),
                    AccessSpecifierKind::Writes => emit!(writer, "writes "),
                    AccessSpecifierKind::LegacyAcquires => emit!(writer, "acquires "),
                }
                match &spec.resource.1 {
                    ResourceSpecifier::Any => emit!(writer, "*"),
                    ResourceSpecifier::DeclaredAtAddress(addr) => {
                        emit!(
                            writer,
                            "0x{}::*",
                            addr.expect_numerical().short_str_lossless()
                        )
                    },
                    ResourceSpecifier::DeclaredInModule(mid) => {
                        emit!(writer, "{}::*", self.get_module(*mid).get_full_name_str())
                    },
                    ResourceSpecifier::Resource(sid) => {
                        emit!(writer, "{}", sid.to_type().display(tctx))
                    },
                }
                emit!(writer, "(");
                match &spec.address.1 {
                    AddressSpecifier::Any => emit!(writer, "*"),
                    AddressSpecifier::Address(addr) => {
                        emit!(writer, "0x{}", addr.expect_numerical().short_str_lossless())
                    },
                    AddressSpecifier::Parameter(sym) => {
                        emit!(writer, "{}", sym.display(self.symbol_pool()))
                    },
                    AddressSpecifier::Call(fun, sym) => emit!(
                        writer,
                        "{}({})",
                        self.get_function(fun.to_qualified_id()).get_full_name_str(),
                        sym.display(self.symbol_pool())
                    ),
                }
                emitln!(writer, ")")
            }
            writer.unindent()
        }
        let fun_def = fun.get_def();
        if let Some(exp) = fun_def {
            emitln!(writer, " {");
            writer.indent();
            emitln!(writer, "{}", exp.display_for_fun(fun));
            writer.unindent();
            emitln!(writer, "}");
        } else {
            emitln!(writer, ";");
        }
        let spec = fun.get_spec();
        if !spec.is_empty() {
            emitln!(writer, "{}", self.display(&*spec))
        }
    }

    /// Helper to create a string for a function signature.
    fn get_fun_signature_string(
        &self,
        tctx: &TypeDisplayContext,
        type_params: &[TypeParameter],
        params: &[Parameter],
        result_type: &Type,
    ) -> String {
        let spool = self.symbol_pool();
        let type_params_str = if !type_params.is_empty() {
            format!(
                "<{}>",
                type_params
                    .iter()
                    .map(|p| p.0.display(spool).to_string())
                    .join(",")
            )
        } else {
            "".to_owned()
        };
        let params_str = params
            .iter()
            .map(|p| format!("{}: {}", p.0.display(spool), p.1.display(tctx)))
            .join(",");
        let result_str = if result_type.is_unit() {
            "".to_owned()
        } else {
            format!(": {}", result_type.display(tctx))
        };
        format!("{}({}){}", type_params_str, params_str, result_str)
    }
}

// =================================================================================================
/// # Module Environment

/// Represents data for a module.
#[derive(Debug)]
pub struct ModuleData {
    /// Module name.
    pub(crate) name: ModuleName,

    /// Id of this module in the global env.
    pub(crate) id: ModuleId,

    /// Attributes attached to this module.
    pub(crate) attributes: Vec<Attribute>,

    /// Use declarations
    pub(crate) use_decls: Vec<UseDecl>,

    /// Friend declarations
    pub(crate) friend_decls: Vec<FriendDecl>,

    /// Module byte code, if available.
    pub(crate) compiled_module: Option<CompiledModule>,

    /// Module source location information for bytecode, if a bytecode module is attached.
    pub(crate) source_map: Option<SourceMap>,

    /// Named constant data
    pub(crate) named_constants: BTreeMap<NamedConstantId, NamedConstantData>,

    /// Struct data.
    pub(crate) struct_data: BTreeMap<StructId, StructData>,

    /// Mapping from struct definition index to id in above map.
    pub(crate) struct_idx_to_id: BTreeMap<StructDefinitionIndex, StructId>,

    /// Function data.
    pub(crate) function_data: BTreeMap<FunId, FunctionData>,

    /// Mapping from function definition index to id in above map. This map is empty if not
    /// bytecode module is attached.
    pub(crate) function_idx_to_id: BTreeMap<FunctionDefinitionIndex, FunId>,

    /// Specification variables, in SpecVarId order.
    pub(crate) spec_vars: BTreeMap<SpecVarId, SpecVarDecl>,

    /// Specification functions, in SpecFunId order.
    pub(crate) spec_funs: BTreeMap<SpecFunId, SpecFunDecl>,

    /// Module level specification.
    pub(crate) module_spec: RefCell<Spec>,

    /// The location of this module.
    pub(crate) loc: Loc,

    /// A list of spec block infos, for documentation generation.
    pub(crate) spec_block_infos: Vec<SpecBlockInfo>,

    /// Holds the set of modules used by this one. This excludes usage from specifications.
    pub(crate) used_modules: BTreeSet<ModuleId>,

    /// Holds the set of modules used by this one, including specification constructs.
    /// Computed lazily.
    pub(crate) used_modules_including_specs: RefCell<Option<BTreeSet<ModuleId>>>,

    /// Holds the set of modules declared as friend.
    pub(crate) friend_modules: BTreeSet<ModuleId>,
}

impl ModuleData {
    pub fn new(name: ModuleName, id: ModuleId, loc: Loc) -> Self {
        Self {
            name,
            id,
            loc,
            attributes: vec![],
            use_decls: vec![],
            friend_decls: vec![],
            compiled_module: None,
            source_map: None,
            named_constants: Default::default(),
            struct_data: Default::default(),
            struct_idx_to_id: Default::default(),
            function_data: Default::default(),
            function_idx_to_id: Default::default(),
            spec_vars: Default::default(),
            spec_funs: Default::default(),
            module_spec: RefCell::new(Default::default()),
            spec_block_infos: vec![],
            used_modules: Default::default(),
            used_modules_including_specs: RefCell::new(None),
            friend_modules: Default::default(),
        }
    }
}

/// Represents a module environment.
#[derive(Debug, Clone)]
pub struct ModuleEnv<'env> {
    /// Reference to the outer env.
    pub env: &'env GlobalEnv,

    /// Reference to the data of the module.
    pub data: &'env ModuleData,
}

impl<'env> ModuleEnv<'env> {
    /// Returns the id of this module in the global env.
    pub fn get_id(&self) -> ModuleId {
        self.data.id
    }

    /// Returns the name of this module.
    pub fn get_name(&'env self) -> &'env ModuleName {
        &self.data.name
    }

    /// Returns true if either the full name or simple name of this module matches the given string
    pub fn matches_name(&self, name: &str) -> bool {
        self.get_full_name_str() == name || self.get_name().display(self.env).to_string() == name
    }

    /// Returns the location of this module.
    pub fn get_loc(&'env self) -> Loc {
        self.data.loc.clone()
    }

    /// Returns the attributes of this module.
    pub fn get_attributes(&self) -> &[Attribute] {
        &self.data.attributes
    }

    /// Checks whether the module has an attribute.
    pub fn has_attribute(&self, pred: impl Fn(&Attribute) -> bool) -> bool {
        Attribute::has(&self.data.attributes, pred)
    }

    /// Checks whether this item is only used in tests.
    pub fn is_test_only(&self) -> bool {
        self.has_attribute(|a| {
            let s = self.symbol_pool().string(a.name());
            well_known::is_test_only_attribute_name(s.as_str())
        })
    }

    /// Checks whether this item is only used in verification.
    pub fn is_verify_only(&self) -> bool {
        self.has_attribute(|a| {
            let s = self.symbol_pool().string(a.name());
            well_known::is_verify_only_attribute_name(s.as_str())
        })
    }

    /// Returns the use declarations of this module.
    pub fn get_use_decls(&self) -> &[UseDecl] {
        &self.data.use_decls
    }

    /// Returns the friend declarations of this module.
    pub fn get_friend_decls(&self) -> &[FriendDecl] {
        &self.data.friend_decls
    }

    /// Does this module declare `module_id` as a friend?
    pub fn has_friend(&self, module_id: &ModuleId) -> bool {
        self.data.friend_modules.contains(module_id)
    }

    /// Does this module have any friends?
    pub fn has_no_friends(&self) -> bool {
        self.data.friend_modules.is_empty()
    }

    /// Returns full name as a string.
    pub fn get_full_name_str(&self) -> String {
        self.get_name().display_full(self.env).to_string()
    }

    /// Returns the VM identifier for this module, if a compiled module is attached.
    pub fn get_identifier(&'env self) -> Option<Identifier> {
        self.data
            .compiled_module
            .as_ref()
            .map(|m| m.name().to_owned())
    }

    /// Returns true if this is a module representing a script.
    pub fn is_script_module(&self) -> bool {
        self.data.name.is_script()
    }

    /// Returns true if this module is target of compilation. A non-target module is
    /// a dependency only but not explicitly requested to process.
    pub fn is_target(&self) -> bool {
        let file_id = self.data.loc.file_id;
        *self.env.everything_is_target.borrow() || self.env.file_id_is_target.contains(&file_id)
    }

    /// Returns true of this module is a primary (test/docgen/warning/prover) target.
    pub fn is_primary_target(&self) -> bool {
        let file_id = self.data.loc.file_id;
        self.env.file_id_is_primary_target.contains(&file_id)
    }

    /// Returns the path to source file of this module.
    pub fn get_source_path(&self) -> &OsStr {
        let file_id = self.data.loc.file_id;
        self.env.source_files.name(file_id)
    }

    /// Returns the set of modules that use this one.
    pub fn get_using_modules(&self, include_specs: bool) -> BTreeSet<ModuleId> {
        self.env
            .get_modules()
            .filter_map(|module_env| {
                if module_env
                    .get_used_modules(include_specs)
                    .contains(&self.data.id)
                {
                    Some(module_env.data.id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the set of modules this one uses.
    pub fn get_used_modules(&self, include_specs: bool) -> BTreeSet<ModuleId> {
        if !include_specs {
            self.data.used_modules.clone()
        } else {
            if let Some(used) = self.data.used_modules_including_specs.borrow().as_ref() {
                return used.clone();
            }
            let mut usage = self.data.used_modules.clone();
            let add_usage_of_exp = |usage: &mut BTreeSet<ModuleId>, exp: &ExpData| {
                exp.module_usage(usage);
                for node_id in exp.node_ids() {
                    self.env.get_node_type(node_id).module_usage(usage);
                    for ty in self.env.get_node_instantiation(node_id) {
                        ty.module_usage(usage);
                    }
                }
            };
            let add_usage_of_spec = |usage: &mut BTreeSet<ModuleId>, spec: &Spec| {
                for cond in &spec.conditions {
                    add_usage_of_exp(usage, &cond.exp);
                }
            };
            add_usage_of_spec(&mut usage, &self.get_spec());
            for struct_env in self.get_structs() {
                add_usage_of_spec(&mut usage, &struct_env.get_spec())
            }
            for func_env in self.get_functions() {
                add_usage_of_spec(&mut usage, &func_env.get_spec())
            }
            for (_, decl) in self.get_spec_funs() {
                if let Some(def) = &decl.body {
                    add_usage_of_exp(&mut usage, def);
                }
            }
            *self.data.used_modules_including_specs.borrow_mut() = Some(usage.clone());
            usage
        }
    }

    /// Returns the set of modules this one declares as friends.
    pub fn get_friend_modules(&self) -> BTreeSet<ModuleId> {
        self.data.friend_modules.clone()
    }

    /// Returns the set of modules in the current package,
    /// whose public(package) functions are called or referenced in the current module.
    pub fn need_to_be_friended_by(&self) -> BTreeSet<ModuleId> {
        let mut deps = BTreeSet::new();
        if self.is_script_module() {
            return deps;
        }
        for fun_env in self.get_functions() {
            // We need to traverse transitive inline functions because they will be expanded during inlining.
            for used_fun in fun_env.get_used_functions_with_transitive_inline() {
                let used_mod_id = used_fun.module_id;
                if self.get_id() == used_mod_id {
                    // no need to friend self
                    continue;
                }
                let used_mod_env = self.env.get_module(used_mod_id);
                let used_fun_env = used_mod_env.get_function(used_fun.id);
                if used_fun_env.has_package_visibility()
                    && self.can_call_package_fun_in(&used_mod_env)
                {
                    deps.insert(used_mod_id);
                }
            }
        }
        deps
    }

    /// Returns true if functions in the current module can call a public(package) function in the given module.
    fn can_call_package_fun_in(&self, other: &Self) -> bool {
        !self.is_script_module()
            && !other.is_script_module()
            // TODO(#13745): fix this when we have a way to check if
            // two non-primary targets are in the same package
            && (!self.is_primary_target() || other.is_primary_target())
            && self.self_address() == other.self_address()
    }

    /// Returns true if the given module is a transitive dependency of this one. The
    /// transitive dependency set contains this module and all directly or indirectly used
    /// modules (without spec usage).
    pub fn is_transitive_dependency(&self, module_id: ModuleId) -> bool {
        if self.get_id() == module_id {
            true
        } else {
            for dep in self.get_used_modules(false) {
                if self.env.get_module(dep).is_transitive_dependency(module_id) {
                    return true;
                }
            }
            false
        }
    }

    /// Returns documentation associated with this module.
    pub fn get_doc(&self) -> &str {
        self.env.get_doc(&self.data.loc)
    }

    /// Returns spec block documentation infos.
    pub fn get_spec_block_infos(&self) -> &[SpecBlockInfo] {
        &self.data.spec_block_infos
    }

    /// Shortcut for accessing the symbol pool.
    pub fn symbol_pool(&self) -> &SymbolPool {
        &self.env.symbol_pool
    }

    /// Returns a context to display types for this module.
    pub fn get_type_display_ctx(&self) -> TypeDisplayContext {
        TypeDisplayContext {
            module_name: Some(self.get_name().clone()),
            used_modules: self.get_used_modules(false),
            ..TypeDisplayContext::new(self.env)
        }
    }

    /// Gets the underlying bytecode module, if one is attached.
    pub fn get_verified_module(&'env self) -> Option<&'env CompiledModule> {
        self.data.compiled_module.as_ref()
    }

    /// Gets the underlying source map, if one is attached.
    pub fn get_source_map(&'env self) -> Option<&'env SourceMap> {
        self.data.source_map.as_ref()
    }

    /// Gets a `NamedConstantEnv` in this module by name
    pub fn find_named_constant(&'env self, name: Symbol) -> Option<NamedConstantEnv<'env>> {
        let id = NamedConstantId(name);
        self.data
            .named_constants
            .get(&id)
            .map(|data| NamedConstantEnv {
                module_env: self.clone(),
                data,
            })
    }

    /// Gets a `NamedConstantEnv` in this module by the constant's id
    pub fn get_named_constant(&'env self, id: NamedConstantId) -> NamedConstantEnv<'env> {
        self.clone().into_named_constant(id)
    }

    /// Gets a `NamedConstantEnv` by id
    pub fn into_named_constant(self, id: NamedConstantId) -> NamedConstantEnv<'env> {
        let data = self
            .data
            .named_constants
            .get(&id)
            .expect("NamedConstantId undefined");
        NamedConstantEnv {
            module_env: self,
            data,
        }
    }

    /// Gets the number of named constants in this module.
    pub fn get_named_constant_count(&self) -> usize {
        self.data.named_constants.len()
    }

    /// Returns iterator over `NamedConstantEnv`s in this module.
    pub fn get_named_constants(&'env self) -> impl Iterator<Item = NamedConstantEnv<'env>> {
        self.clone().into_named_constants()
    }

    /// Returns an iterator over `NamedConstantEnv`s in this module.
    pub fn into_named_constants(self) -> impl Iterator<Item = NamedConstantEnv<'env>> {
        self.data
            .named_constants
            .values()
            .map(move |data| NamedConstantEnv {
                module_env: self.clone(),
                data,
            })
    }

    /// Gets a FunctionEnv in this module by name.
    pub fn find_function(&self, name: Symbol) -> Option<FunctionEnv<'env>> {
        let id = FunId(name);
        self.data.function_data.get(&id).map(|data| FunctionEnv {
            module_env: self.clone(),
            data,
        })
    }

    /// Gets a FunctionEnv by id.
    pub fn get_function(&'env self, id: FunId) -> FunctionEnv<'env> {
        self.clone().into_function(id)
    }

    /// Gets a FunctionEnv by id.
    pub fn into_function(self, id: FunId) -> FunctionEnv<'env> {
        let data = self.data.function_data.get(&id).unwrap_or_else(|| {
            panic!(
                "FunId {}::{} undefined",
                self.get_full_name_str(),
                id.0.display(self.symbol_pool())
            )
        });
        FunctionEnv {
            module_env: self,
            data,
        }
    }

    /// Gets the number of functions in this module.
    pub fn get_function_count(&self) -> usize {
        self.data.function_data.len()
    }

    /// Returns iterator over FunctionEnvs in this module.
    pub fn get_functions(&'env self) -> impl Iterator<Item = FunctionEnv<'env>> {
        self.clone().into_functions()
    }

    /// Returns iterator over FunctionEnvs in this module.
    pub fn into_functions(self) -> impl Iterator<Item = FunctionEnv<'env>> {
        self.data
            .function_data
            .values()
            .map(move |data| FunctionEnv {
                module_env: self.clone(),
                data,
            })
    }

    /// Gets FunctionEnv for a function used in an attached compiled module, via the
    /// FunctionHandleIndex. The returned function might be from this or another module.
    pub fn get_used_function(&self, idx: FunctionHandleIndex) -> Option<FunctionEnv<'_>> {
        let module = self.data.compiled_module.as_ref()?;
        Some(Self::get_used_function_from_compiled_module(
            self.env, idx, module,
        ))
    }

    fn get_used_function_from_compiled_module<'a>(
        env: &'a GlobalEnv,
        idx: FunctionHandleIndex,
        module: &CompiledModule,
    ) -> FunctionEnv<'a> {
        let view = FunctionHandleView::new(module, module.function_handle_at(idx));
        let module_name = env.to_module_name(&view.module_id());
        let module_env = env
            .find_module(&module_name)
            .expect("unexpected reference to module not found in global env");
        module_env.into_function(FunId::new(env.symbol_pool.make(view.name().as_str())))
    }

    /// Gets the function id from a definition index.
    pub fn try_get_function_id(&self, idx: FunctionDefinitionIndex) -> Option<FunId> {
        self.data.function_idx_to_id.get(&idx).cloned()
    }

    /// Gets the function definition index for the given function id, if a module is attached.
    pub fn get_function_def_idx(&self, fun_id: FunId) -> Option<FunctionDefinitionIndex> {
        self.data
            .function_data
            .get(&fun_id)
            .expect("function id defined")
            .def_idx
    }

    /// Gets a StructEnv in this module by name.
    pub fn find_struct(&self, name: Symbol) -> Option<StructEnv<'_>> {
        let id = StructId(name);
        self.data.struct_data.get(&id).map(|data| StructEnv {
            module_env: self.clone(),
            data,
        })
    }

    /// Gets a StructEnv in this module by identifier
    pub fn find_struct_by_identifier(&self, identifier: Identifier) -> Option<StructId> {
        let some_id = Some(identifier);
        for data in self.data.struct_data.values() {
            let senv = StructEnv {
                module_env: self.clone(),
                data,
            };
            if senv.get_identifier() == some_id {
                return Some(senv.get_id());
            }
        }
        None
    }

    /// Gets the struct id from a definition index which must be valid for this environment.
    pub fn get_struct_id(&self, idx: StructDefinitionIndex) -> StructId {
        *self
            .data
            .struct_idx_to_id
            .get(&idx)
            .unwrap_or_else(|| panic!("undefined struct definition index {:?}", idx))
    }

    /// Gets a StructEnv by id.
    pub fn get_struct(&self, id: StructId) -> StructEnv<'_> {
        let data = self.data.struct_data.get(&id).expect("StructId undefined");
        StructEnv {
            module_env: self.clone(),
            data,
        }
    }

    pub fn get_struct_by_def_idx(&self, idx: StructDefinitionIndex) -> StructEnv<'_> {
        self.get_struct(self.get_struct_id(idx))
    }

    /// Gets a StructEnv by id, consuming this module env.
    pub fn into_struct(self, id: StructId) -> StructEnv<'env> {
        let data = self.data.struct_data.get(&id).expect("StructId undefined");
        StructEnv {
            module_env: self,
            data,
        }
    }

    /// Gets the number of structs in this module.
    pub fn get_struct_count(&self) -> usize {
        self.data.struct_data.len()
    }

    /// Returns iterator over structs in this module.
    pub fn get_structs(&'env self) -> impl Iterator<Item = StructEnv<'env>> {
        self.clone().into_structs()
    }

    /// Returns iterator over structs in this module.
    pub fn into_structs(self) -> impl Iterator<Item = StructEnv<'env>> {
        self.data.struct_data.values().map(move |data| StructEnv {
            module_env: self.clone(),
            data,
        })
    }

    /// Globalizes a signature local to this module. This requires a compiled module to be
    /// attached.
    pub fn globalize_signature(&self, sig: &SignatureToken) -> Option<Type> {
        let struct_resolver = |module_name: ModuleName, struct_name: Symbol| {
            let declaring_module_env = self
                .env
                .find_module(&module_name)
                .expect("expected defined module name");
            let struct_env = declaring_module_env
                .find_struct(struct_name)
                .expect("expected defined struct name");
            declaring_module_env.get_id().qualified(struct_env.get_id())
        };
        Some(Type::from_signature_token(
            self.env,
            self.data.compiled_module.as_ref()?,
            &struct_resolver,
            sig,
        ))
    }

    /// Globalizes a list of signatures.
    pub fn globalize_signatures(&self, sigs: &[SignatureToken]) -> Option<Vec<Type>> {
        sigs.iter().map(|s| self.globalize_signature(s)).collect()
    }

    /// Gets a list of type actuals associated with the index in the bytecode, if
    /// a compiled module is attached.
    pub fn get_type_actuals(&self, idx: Option<SignatureIndex>) -> Option<Vec<Type>> {
        let module = self.data.compiled_module.as_ref()?;
        match idx {
            Some(idx) => {
                let actuals = &module.signature_at(idx).0;
                self.globalize_signatures(actuals)
            },
            None => Some(vec![]),
        }
    }

    /// Retrieve a constant from the pool of an attached compiled module.
    pub fn get_constant(&self, idx: ConstantPoolIndex) -> Option<&VMConstant> {
        self.data
            .compiled_module
            .as_ref()
            .map(|m| &m.constant_pool()[idx.0 as usize])
    }

    /// Converts a constant to the specified type. The type must correspond to the expected
    /// canonical representation as defined in `move_core_types::values`
    pub fn get_constant_value(&self, constant: &VMConstant) -> MoveValue {
        VMConstant::deserialize_constant(constant).unwrap()
    }

    /// Return the address of this module
    pub fn self_address(&self) -> &Address {
        self.data.name.addr()
    }

    /// Returns specification variables of this module.
    pub fn get_spec_vars(&'env self) -> impl Iterator<Item = (&'env SpecVarId, &'env SpecVarDecl)> {
        self.data.spec_vars.iter()
    }

    /// Gets spec var by id.
    pub fn get_spec_var(&self, id: SpecVarId) -> &SpecVarDecl {
        self.data.spec_vars.get(&id).expect("spec var id defined")
    }

    /// Find spec var by name.
    pub fn find_spec_var(&self, name: Symbol) -> Option<&SpecVarDecl> {
        self.data
            .spec_vars
            .iter()
            .find(|(_, svar)| svar.name == name)
            .map(|(_, svar)| svar)
    }

    /// Returns specification functions of this module.
    pub fn get_spec_funs(&'env self) -> impl Iterator<Item = (&'env SpecFunId, &'env SpecFunDecl)> {
        self.data.spec_funs.iter()
    }

    /// Gets spec fun by id.
    pub fn get_spec_fun(&self, id: SpecFunId) -> &SpecFunDecl {
        self.data.spec_funs.get(&id).expect("spec fun id defined")
    }

    /// Gets module specification.
    pub fn get_spec(&self) -> Ref<Spec> {
        self.data.module_spec.borrow()
    }

    /// Returns whether a spec fun is ever called/referenced or not.
    pub fn spec_fun_is_used(&self, spec_fun_id: SpecFunId) -> bool {
        self.env
            .used_spec_funs
            .contains(&self.get_id().qualified(spec_fun_id))
    }

    /// Get all spec fun overloads with the given name.
    pub fn get_spec_funs_of_name(
        &self,
        name: Symbol,
    ) -> impl Iterator<Item = (&'env SpecFunId, &'env SpecFunDecl)> {
        self.data
            .spec_funs
            .iter()
            .filter(move |(_, decl)| decl.name == name)
    }

    /// Disassemble the module bytecode, if it is available.
    pub fn disassemble(&self) -> Option<String> {
        let view = BinaryIndexedView::Module(self.get_verified_module()?);
        let smap = self.data.source_map.as_ref().expect("source map").clone();
        let disas = Disassembler::new(SourceMapping::new(smap, view), DisassemblerOptions {
            only_externally_visible: false,
            print_code: true,
            print_basic_blocks: true,
            print_locals: true,
            print_bytecode_stats: false,
        });
        Some(
            disas
                .disassemble()
                // Failure here is fatal and should not happen
                .expect("Failed to disassemble a verified module"),
        )
    }

    /// Return true if the module has any global, module, function, or struct specs
    pub fn has_specs(&self) -> bool {
        // module specs
        if self.get_spec().has_conditions() {
            return true;
        }
        // function specs
        for f in self.get_functions() {
            if f.get_spec().has_conditions() || !f.get_spec().on_impl.is_empty() {
                return true;
            }
        }
        // struct specs
        for s in self.get_structs() {
            if s.get_spec().has_conditions() {
                return true;
            }
        }
        // global specs
        let global_invariants = self.env.get_global_invariants_by_module(self.get_id());
        if !global_invariants.is_empty() {
            return true;
        }
        false
    }

    fn match_module_name(&self, module_name: &str) -> bool {
        self.get_name()
            .name()
            .display(self.env.symbol_pool())
            .to_string()
            == module_name
    }

    fn is_module_in_std(&self, module_name: &str) -> bool {
        let addr = self.get_name().addr();
        *addr == self.env.get_stdlib_address() && self.match_module_name(module_name)
    }

    fn is_module_in_ext(&self, module_name: &str) -> bool {
        let addr = self.get_name().addr();
        *addr == self.env.get_extlib_address() && self.match_module_name(module_name)
    }

    pub fn is_std_vector(&self) -> bool {
        self.is_module_in_std("vector")
    }

    pub fn is_table(&self) -> bool {
        self.is_module_in_std("table")
            || self.is_module_in_std("table_with_length")
            || self.is_module_in_ext("table")
            || self.is_module_in_ext("table_with_length")
    }
}

// =================================================================================================
/// # Struct Environment

#[derive(Debug)]
pub struct StructData {
    /// The name of this struct.
    pub(crate) name: Symbol,

    /// The location of this struct.
    pub(crate) loc: Loc,

    /// The definition index of this structure in its bytecode module, if a bytecode module
    /// is attached to the parent module data.
    pub(crate) def_idx: Option<StructDefinitionIndex>,

    /// Attributes attached to this struct.
    pub(crate) attributes: Vec<Attribute>,

    /// Type parameters of this struct.
    pub(crate) type_params: Vec<TypeParameter>,

    /// Abilities of this struct.
    pub(crate) abilities: AbilitySet,

    /// If this is a struct generated for a specification variable, the variable id.
    pub(crate) spec_var_opt: Option<SpecVarId>,

    /// Field definitions.
    pub(crate) field_data: BTreeMap<FieldId, FieldData>,

    /// If this structure has variants (i.e. is an `enum`), information about the
    /// names of the variants and the location of their declaration. The fields
    /// of variants can be identified via the variant name in `FieldData`.
    pub(crate) variants: Option<BTreeMap<Symbol, StructVariant>>,

    /// Associated specification.
    pub(crate) spec: RefCell<Spec>,

    /// Whether this struct is native
    pub is_native: bool,
}

impl StructData {
    pub fn new(name: Symbol, loc: Loc) -> Self {
        Self {
            name,
            loc,
            def_idx: None,
            attributes: vec![],
            type_params: vec![],
            abilities: AbilitySet::ALL,
            spec_var_opt: None,
            field_data: Default::default(),
            variants: None,
            spec: RefCell::new(Default::default()),
            is_native: false,
        }
    }
}

#[derive(Debug)]
pub(crate) struct StructVariant {
    pub(crate) loc: Loc,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) order: usize,
}

#[derive(Debug, Clone)]
pub struct StructEnv<'env> {
    /// Reference to enclosing module.
    pub module_env: ModuleEnv<'env>,

    /// Reference to the struct data.
    pub data: &'env StructData,
}

impl<'env> StructEnv<'env> {
    /// Shortcut to access the env
    pub fn env(&self) -> &GlobalEnv {
        self.module_env.env
    }

    /// Returns the name of this struct.
    pub fn get_name(&self) -> Symbol {
        self.data.name
    }

    /// Gets full name as string.
    pub fn get_full_name_str(&self) -> String {
        let module_name = self.module_env.get_name().display(self.module_env.env);
        if self.is_ghost_memory() {
            let spec_var = self.get_ghost_memory_spec_var().expect("spec var");
            let spec_var_name = self.module_env.get_spec_var(spec_var.id);
            format!(
                "{}::{}",
                module_name,
                spec_var_name.name.display(self.symbol_pool())
            )
        } else {
            format!(
                "{}::{}",
                module_name,
                self.get_name().display(self.symbol_pool())
            )
        }
    }

    /// Gets full name with module address as string.
    pub fn get_full_name_with_address(&self) -> String {
        format!(
            "{}::{}",
            self.module_env.get_full_name_str(),
            self.get_name().display(self.symbol_pool())
        )
    }

    /// Returns the VM identifier for this struct
    pub fn get_identifier(&self) -> Option<Identifier> {
        Identifier::new(self.symbol_pool().string(self.get_name()).as_str()).ok()
    }

    /// Shortcut for accessing the symbol pool.
    pub fn symbol_pool(&self) -> &SymbolPool {
        self.module_env.symbol_pool()
    }

    /// Returns the location of this struct.
    pub fn get_loc(&self) -> Loc {
        self.data.loc.clone()
    }

    /// Returns the attributes of this struct.
    pub fn get_attributes(&self) -> &[Attribute] {
        &self.data.attributes
    }

    /// Checks whether the struct has an attribute.
    pub fn has_attribute(&self, pred: impl Fn(&Attribute) -> bool) -> bool {
        Attribute::has(&self.data.attributes, pred)
    }

    /// Checks whether this item is only used in tests.
    pub fn is_test_only(&self) -> bool {
        self.has_attribute(|a| {
            let s = self.symbol_pool().string(a.name());
            well_known::is_test_only_attribute_name(s.as_str())
        })
    }

    /// Checks whether this item is only used in verification.
    pub fn is_verify_only(&self) -> bool {
        self.has_attribute(|a| {
            let s = self.symbol_pool().string(a.name());
            well_known::is_verify_only_attribute_name(s.as_str())
        })
    }

    /// Get documentation associated with this struct.
    pub fn get_doc(&self) -> &str {
        self.module_env.env.get_doc(&self.data.loc)
    }

    /// Returns properties from pragmas.
    pub fn get_properties(&self) -> PropertyBag {
        self.data.spec.borrow().properties.clone()
    }

    /// Gets the id associated with this struct.
    pub fn get_id(&self) -> StructId {
        StructId(self.data.name)
    }

    /// Gets the qualified id of this struct.
    pub fn get_qualified_id(&self) -> QualifiedId<StructId> {
        self.module_env.get_id().qualified(self.get_id())
    }

    /// Returns true if this struct has the pragma intrinsic set to true.
    pub fn is_intrinsic(&self) -> bool {
        self.is_pragma_true(INTRINSIC_PRAGMA, || {
            self.module_env
                .env
                .intrinsics
                .get_decl_for_struct(&self.get_qualified_id())
                .is_some()
        })
    }

    /// Returns true if this is an intrinsic struct of a given name
    pub fn is_intrinsic_of(&self, name: &str) -> bool {
        self.module_env.env.intrinsics.is_intrinsic_of_for_struct(
            self.symbol_pool(),
            &self.get_qualified_id(),
            name,
        )
    }

    /// Returns true if this struct is ghost memory for a specification variable.
    pub fn is_ghost_memory(&self) -> bool {
        self.data.spec_var_opt.is_some()
    }

    /// Get the specification variable associated with this struct if this is ghost memory.
    pub fn get_ghost_memory_spec_var(&self) -> Option<QualifiedId<SpecVarId>> {
        self.data
            .spec_var_opt
            .map(|v| self.module_env.get_id().qualified(v))
    }

    /// Get the abilities of this struct.
    pub fn get_abilities(&self) -> AbilitySet {
        self.data.abilities
    }

    /// Determines whether memory-related operations needs to be declared for this struct.
    pub fn has_memory(&self) -> bool {
        self.get_abilities().has_key()
    }

    /// Returns true if the struct has variants.
    pub fn has_variants(&self) -> bool {
        self.data.variants.is_some()
    }

    /// Returns an iteration of the variant names in the struct, in the order they
    /// are declared.
    pub fn get_variants(&self) -> impl Iterator<Item = Symbol> + 'env {
        self.data
            .variants
            .as_ref()
            .expect("struct has variants")
            .iter()
            .sorted_by(|(_, v1), (_, v2)| v1.order.cmp(&v2.order))
            .map(|(s, _)| *s)
    }

    /// Returns the location of the variant.
    pub fn get_variant_loc(&self, variant: Symbol) -> &Loc {
        self.data
            .variants
            .as_ref()
            .and_then(|vars| vars.get(&variant).map(|v| &v.loc))
            .expect("variant defined")
    }

    /// Get the index of the variant in the struct.
    pub fn get_variant_idx(&self, variant: Symbol) -> Option<VariantIndex> {
        self.get_variants()
            .position(|n| variant == n)
            .map(|p| p as VariantIndex)
    }

    /// Get the name of the variant in the struct by index.
    pub fn get_variant_name_by_idx(&self, variant: VariantIndex) -> Option<Symbol> {
        self.get_variants().nth(variant as usize)
    }

    /// Returns the attributes of the variant.
    pub fn get_variant_attributes(&self, variant: Symbol) -> &[Attribute] {
        self.data
            .variants
            .as_ref()
            .and_then(|vars| vars.get(&variant).map(|v| v.attributes.as_slice()))
            .expect("variant defined")
    }

    /// Get an iterator for the fields, ordered by offset. Notice if the struct has
    /// variants, all fields of all variants are returned.
    pub fn get_fields(&'env self) -> impl Iterator<Item = FieldEnv<'env>> {
        self.get_fields_optional_variant(None)
    }

    /// Get fields of a particular variant.
    pub fn get_fields_of_variant(
        &'env self,
        variant: Symbol,
    ) -> impl Iterator<Item = FieldEnv<'env>> {
        self.get_fields_optional_variant(Some(variant))
    }

    /// Get fields of a particular variant.
    pub fn get_fields_optional_variant(
        &'env self,
        variant: Option<Symbol>,
    ) -> impl Iterator<Item = FieldEnv<'env>> {
        self.data
            .field_data
            .values()
            .filter(|data| variant.is_none() || data.variant == variant)
            .sorted_by_key(|data| data.offset)
            .map(move |data| FieldEnv {
                struct_env: self.clone(),
                data,
            })
    }

    /// Return true if it is a native struct
    pub fn is_native(&self) -> bool {
        self.data.is_native
    }

    /// Return the number of fields in the struct. Notice of the struct has variants, this
    /// includes the sum of all fields in all variants.
    pub fn get_field_count(&self) -> usize {
        self.data.field_data.len()
    }

    /// Gets a field by its id.
    pub fn get_field(&'env self, id: FieldId) -> FieldEnv<'env> {
        let data = self.data.field_data.get(&id).expect("FieldId undefined");
        FieldEnv {
            struct_env: self.clone(),
            data,
        }
    }

    /// Find a field by its name.
    pub fn find_field(&'env self, name: Symbol) -> Option<FieldEnv<'env>> {
        let id = FieldId(name);
        self.data.field_data.get(&id).map(|data| FieldEnv {
            struct_env: self.clone(),
            data,
        })
    }

    /// Gets a field by its offset.
    pub fn get_field_by_offset(&'env self, offset: usize) -> FieldEnv<'env> {
        self.get_field_by_offset_optional_variant(None, offset)
    }

    /// Gets a field by its offset, in context of an optional variant.
    pub fn get_field_by_offset_optional_variant(
        &'env self,
        variant: Option<Symbol>,
        offset: usize,
    ) -> FieldEnv<'env> {
        // We may speed this up via a cache RefCell<BTreeMap<(variant, offset), FieldId>>
        for field in self.get_fields_optional_variant(variant) {
            if field.get_offset() == offset {
                return field;
            }
        }
        unreachable!("invalid field lookup")
    }

    /// Whether the type parameter at position `idx` is declared as phantom.
    pub fn is_phantom_parameter(&self, idx: usize) -> bool {
        self.data
            .type_params
            .get(idx)
            .map(|p| p.1.is_phantom)
            .unwrap_or(false)
    }

    /// Returns the type parameters associated with this struct.
    pub fn get_type_parameters(&self) -> &[TypeParameter] {
        &self.data.type_params
    }

    /// Returns true if this struct has specification conditions.
    pub fn has_conditions(&self) -> bool {
        !self.data.spec.borrow().conditions.is_empty()
    }

    /// Returns the data invariants associated with this struct.
    pub fn get_spec(&self) -> Ref<Spec> {
        self.data.spec.borrow()
    }

    /// Returns the value of a boolean pragma for this struct. This first looks up a
    /// pragma in this struct, then the enclosing module, and finally uses the provided default.
    /// value
    pub fn is_pragma_true(&self, name: &str, default: impl FnOnce() -> bool) -> bool {
        let env = self.module_env.env;
        if let Some(b) = env.is_property_true(&self.get_spec().properties, name) {
            return b;
        }
        if let Some(b) = env.is_property_true(&self.module_env.get_spec().properties, name) {
            return b;
        }
        default()
    }

    /// Produce a TypeDisplayContext to print types within the scope of this env
    pub fn get_type_display_ctx(&self) -> TypeDisplayContext {
        let type_param_names = self
            .get_type_parameters()
            .iter()
            .map(|param| param.0)
            .collect();
        TypeDisplayContext {
            type_param_names: Some(type_param_names),
            ..self.module_env.get_type_display_ctx()
        }
    }

    /// If this is a function type wrapper (`struct W(|T|R)`), get the underlying
    /// function type, instantiated.
    pub fn get_function_wrapper_type(&self, inst: &[Type]) -> Option<Type> {
        if self.get_field_count() == 1 {
            let field = self.get_fields().next().unwrap();
            let ty = field.get_type();
            if field.is_positional() && ty.is_function() {
                return Some(ty.instantiate(inst));
            }
        }
        None
    }
}

// =================================================================================================
/// # Field Environment

#[derive(Debug, Clone)]
pub struct FieldData {
    /// The name of this field.
    pub name: Symbol,

    /// The location of the field declaration.
    pub loc: Loc,

    /// The offset of this field.
    pub offset: usize,

    /// If the field is associated with a variant, the name of that variant.
    pub variant: Option<Symbol>,

    /// The type of this field.
    pub ty: Type,
}

#[derive(Debug)]
pub struct FieldEnv<'env> {
    /// Reference to enclosing struct.
    pub struct_env: StructEnv<'env>,

    /// Reference to the field data.
    data: &'env FieldData,
}

impl FieldEnv<'_> {
    /// Gets the name of this field.
    pub fn get_name(&self) -> Symbol {
        self.data.name
    }

    /// Gets the id of this field.
    pub fn get_id(&self) -> FieldId {
        if let Some(variant) = self.get_variant() {
            // TODO: this is inefficient and we may want to cache it
            let spool = self.struct_env.symbol_pool();
            let id_str = FieldId::make_variant_field_id_str(
                spool.string(variant).as_str(),
                spool.string(self.data.name).as_str(),
            );
            FieldId(spool.make(&id_str))
        } else {
            FieldId(self.data.name)
        }
    }

    /// Returns true if this is a positional field. Identified by that the name
    /// of the field is a number.
    pub fn is_positional(&self) -> bool {
        self.data
            .name
            .display(self.struct_env.symbol_pool())
            .to_string()
            .parse::<u64>()
            .is_ok()
    }

    /// Gets the location of the field declaration.
    pub fn get_loc(&self) -> &Loc {
        &self.data.loc
    }

    /// Get documentation associated with this field.
    pub fn get_doc(&self) -> &str {
        if let (Some(def_idx), Some(mmap)) = (
            self.struct_env.data.def_idx,
            &self.struct_env.module_env.data.source_map,
        ) {
            if let Some(loc) = mmap.get_struct_source_map(def_idx).ok().and_then(|smap| {
                smap.get_field_location(
                    self.data
                        .variant
                        .and_then(|v| self.struct_env.get_variant_idx(v)),
                    self.data.offset as MemberCount,
                )
            }) {
                self.struct_env
                    .module_env
                    .env
                    .get_doc(&self.struct_env.module_env.env.to_loc(&loc))
            } else {
                ""
            }
        } else {
            ""
        }
    }

    /// Gets the type of this field.
    pub fn get_type(&self) -> Type {
        self.data.ty.clone()
    }

    /// Get field offset.
    pub fn get_offset(&self) -> usize {
        self.data.offset
    }

    /// Gets the variant this field is associated with
    pub fn get_variant(&self) -> Option<Symbol> {
        self.data.variant
    }
}

// =================================================================================================
/// # Named Constant Environment

#[derive(Debug)]
pub struct NamedConstantData {
    /// The name of this constant
    pub(crate) name: Symbol,

    /// The location of this constant
    pub(crate) loc: Loc,

    /// The type of this constant
    pub(crate) type_: Type,

    /// The value of this constant, if known.
    pub(crate) value: Value,
}

#[derive(Debug)]
pub struct NamedConstantEnv<'env> {
    /// Reference to enclosing module.
    pub module_env: ModuleEnv<'env>,

    data: &'env NamedConstantData,
}

impl NamedConstantEnv<'_> {
    /// Returns the name of this constant
    pub fn get_name(&self) -> Symbol {
        self.data.name
    }

    /// Returns the id of this constant
    pub fn get_id(&self) -> NamedConstantId {
        NamedConstantId(self.data.name)
    }

    /// Returns documentation associated with this constant
    pub fn get_doc(&self) -> &str {
        self.module_env.env.get_doc(&self.data.loc)
    }

    /// Returns the location of this constant
    pub fn get_loc(&self) -> Loc {
        self.data.loc.clone()
    }

    /// Returns the type of the constant
    pub fn get_type(&self) -> Type {
        self.data.type_.clone()
    }

    /// Returns the value of this constant
    pub fn get_value(&self) -> Value {
        self.data.value.clone()
    }

    /// Returns a context to display types for this module.
    pub fn get_type_display_ctx(&self) -> TypeDisplayContext {
        TypeDisplayContext {
            module_name: Some(self.module_env.get_name().clone()),
            used_modules: self.module_env.get_used_modules(false),
            ..TypeDisplayContext::new(self.module_env.env)
        }
    }
}

// =================================================================================================
/// # Function Environment

pub trait EqIgnoringLoc {
    fn eq_ignoring_loc(&self, other: &Self) -> bool;
}

impl<T: EqIgnoringLoc> EqIgnoringLoc for Vec<T> {
    fn eq_ignoring_loc(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|(a, b)| a.eq_ignoring_loc(b))
    }
}

/// Represents a type parameter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeParameter(pub Symbol, pub TypeParameterKind, pub Loc);

impl EqIgnoringLoc for TypeParameter {
    /// equal ignoring Loc
    fn eq_ignoring_loc(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl TypeParameter {
    /// Creates a new type parameter of given name.
    pub fn new_named(sym: &Symbol, loc: &Loc) -> Self {
        Self(*sym, TypeParameterKind::default(), loc.clone())
    }

    /// Turns an ordered list of type parameters into a vector of type parameters
    pub fn vec_to_formals(params: &[TypeParameter]) -> Vec<Type> {
        params
            .iter()
            .enumerate()
            .map(|(pos, _)| Type::new_param(pos))
            .collect()
    }

    pub fn from_symbols<'a>(
        symbols: impl Iterator<Item = &'a (Symbol, Loc)>,
    ) -> Vec<TypeParameter> {
        symbols
            .map(|(name, loc)| TypeParameter(*name, TypeParameterKind::default(), loc.clone()))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeParameterKind {
    /// The set of abilities which constrain this parameter.
    pub abilities: AbilitySet,
    /// Whether the type is declared as a phantom.
    pub is_phantom: bool,
}

impl TypeParameterKind {
    pub fn new(abilities: AbilitySet) -> Self {
        TypeParameterKind {
            abilities,
            is_phantom: false,
        }
    }

    pub fn new_phantom(abilities: AbilitySet) -> Self {
        TypeParameterKind {
            abilities,
            is_phantom: true,
        }
    }
}

impl Default for TypeParameterKind {
    fn default() -> Self {
        TypeParameterKind::new(AbilitySet::EMPTY)
    }
}

/// Represents a parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter(pub Symbol, pub Type, pub Loc);

impl Parameter {
    pub fn get_name(&self) -> Symbol {
        self.0
    }

    pub fn get_type(&self) -> Type {
        self.1.clone()
    }

    pub fn get_loc(&self) -> Loc {
        self.2.clone()
    }
}

impl EqIgnoringLoc for Parameter {
    /// equal ignoring Loc
    fn eq_ignoring_loc(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

/// Represents source code locations associated with a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionLoc {
    /// Location of this function.
    pub(crate) full: Loc,

    /// Location of the function identifier, suitable for error messages alluding to the function.
    pub(crate) id_loc: Loc,

    /// Location of the function result type, suitable for error messages alluding to the result type.
    pub(crate) result_type_loc: Loc,
}

impl FunctionLoc {
    pub fn from_single(loc: Loc) -> Self {
        // TODO: consider making non-full locations optional, as we do not always have this
        //   information.
        Self {
            full: loc.clone(),
            id_loc: loc.clone(),
            result_type_loc: loc,
        }
    }
}

#[derive(Debug)]
pub struct FunctionData {
    /// Name of this function.
    pub(crate) name: Symbol,

    /// Locations of this function.
    pub(crate) loc: FunctionLoc,

    /// The definition index of this function in its bytecode module, if a bytecode module
    /// is attached to the parent module data.
    pub(crate) def_idx: Option<FunctionDefinitionIndex>,

    /// The handle index of this function in its module, if a bytecode module
    /// is attached to the parent module data.
    pub(crate) handle_idx: Option<FunctionHandleIndex>,

    /// Visibility of this function (private, friend, or public)
    pub(crate) visibility: Visibility,

    /// Whether this function has package visibility before the transformation.
    /// Invariant: when true, visibility is always friend.
    pub(crate) has_package_visibility: bool,

    /// Whether this is a native function
    pub(crate) is_native: bool,

    /// The kind of the function.
    pub(crate) kind: FunctionKind,

    /// Attributes attached to this function.
    pub(crate) attributes: Vec<Attribute>,

    /// Type parameters.
    pub(crate) type_params: Vec<TypeParameter>,

    /// Parameters
    pub(crate) params: Vec<Parameter>,

    /// Result type of the function, uses `Type::Tuple` for multiple values.
    pub(crate) result_type: Type,

    /// Access specifiers.
    pub(crate) access_specifiers: Option<Vec<AccessSpecifier>>,

    /// Acquires information, if available. This is either inferred or annotated by the
    /// user via a legacy acquires declaration.
    pub(crate) acquired_structs: Option<BTreeSet<StructId>>,

    /// Specification associated with this function.
    pub(crate) spec: RefCell<Spec>,

    /// Optional definition associated with this function.
    pub(crate) def: Option<Exp>,

    /// A cache for the called functions.
    pub(crate) called_funs: Option<BTreeSet<QualifiedId<FunId>>>,

    /// A cache for the calling functions.
    pub(crate) calling_funs: RefCell<Option<BTreeSet<QualifiedId<FunId>>>>,

    /// A cache for the transitive closure of the called functions.
    pub(crate) transitive_closure_of_called_funs: RefCell<Option<BTreeSet<QualifiedId<FunId>>>>,

    /// A cache for the used functions.  Used functions are those called or with values taken here.
    pub(crate) used_funs: Option<BTreeSet<QualifiedId<FunId>>>,

    /// A cache for the using functions.  Using functions are those which call or take value of this.
    pub(crate) using_funs: RefCell<Option<BTreeSet<QualifiedId<FunId>>>>,

    /// A cache for the transitive closure of the used functions.
    pub(crate) transitive_closure_of_used_funs: RefCell<Option<BTreeSet<QualifiedId<FunId>>>>,

    /// A cache for used functions including ones obtained by transitively traversing used inline functions.
    pub(crate) used_functions_with_transitive_inline: RefCell<Option<BTreeSet<QualifiedId<FunId>>>>,
}

impl FunctionData {
    pub fn new(name: Symbol, loc: Loc) -> Self {
        Self {
            name,
            loc: FunctionLoc::from_single(loc),
            def_idx: None,
            handle_idx: None,
            visibility: Default::default(),
            has_package_visibility: false,
            is_native: false,
            kind: FunctionKind::Regular,
            attributes: vec![],
            type_params: vec![],
            params: vec![],
            result_type: Type::unit(),
            access_specifiers: None,
            acquired_structs: None,
            spec: RefCell::new(Default::default()),
            def: None,
            called_funs: None,
            calling_funs: RefCell::new(None),
            transitive_closure_of_called_funs: RefCell::new(None),
            used_funs: None,
            using_funs: RefCell::new(None),
            transitive_closure_of_used_funs: RefCell::new(None),
            used_functions_with_transitive_inline: RefCell::new(None),
        }
    }
}

/// Kind of a function,
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionKind {
    Regular,
    Inline,
    Entry,
}

#[derive(Debug, Clone)]
pub struct FunctionEnv<'env> {
    /// Reference to enclosing module.
    pub module_env: ModuleEnv<'env>,

    /// Reference to the function data.
    data: &'env FunctionData,
}

impl<'env> FunctionEnv<'env> {
    /// Shortcut to access the env
    pub fn env(&self) -> &GlobalEnv {
        self.module_env.env
    }

    /// Returns the name of this function.
    pub fn get_name(&self) -> Symbol {
        self.data.name
    }

    /// Gets full name as string.
    pub fn get_full_name_str(&self) -> String {
        format!(
            "{}::{}",
            self.module_env.get_name().display(self.module_env.env),
            self.get_name_str()
        )
    }

    /// Gets full name with module address as string.
    pub fn get_full_name_with_address(&self) -> String {
        format!(
            "{}::{}",
            self.module_env.get_full_name_str(),
            self.get_name_str()
        )
    }

    pub fn get_name_str(&self) -> String {
        self.get_name().display(self.symbol_pool()).to_string()
    }

    /// Returns the VM identifier for this function, if a compiled module is available.
    pub fn get_identifier(&'env self) -> Option<Identifier> {
        let m = self.module_env.data.compiled_module.as_ref()?;
        Some(
            m.identifier_at(m.function_handle_at(self.data.handle_idx?).name)
                .to_owned(),
        )
    }

    /// Gets the id of this function.
    pub fn get_id(&self) -> FunId {
        FunId(self.data.name)
    }

    /// Gets the qualified id of this function.
    pub fn get_qualified_id(&self) -> QualifiedId<FunId> {
        self.module_env.get_id().qualified(self.get_id())
    }

    /// Get documentation associated with this function.
    pub fn get_doc(&self) -> &str {
        self.module_env.env.get_doc(&self.data.loc.full)
    }

    /// Gets the definition index of this function.
    pub fn get_def_idx(&self) -> Option<FunctionDefinitionIndex> {
        self.data.def_idx
    }

    /// Shortcut for accessing the symbol pool.
    pub fn symbol_pool(&self) -> &SymbolPool {
        self.module_env.symbol_pool()
    }

    /// Returns the location of this function.
    pub fn get_loc(&self) -> Loc {
        self.data.loc.full.clone()
    }

    /// Returns the location of the function identifier.
    pub fn get_id_loc(&self) -> Loc {
        self.data.loc.id_loc.clone()
    }

    /// Returns the location of the function's return type.
    pub fn get_result_type_loc(&self) -> Loc {
        self.data.loc.result_type_loc.clone()
    }

    /// Returns the attributes of this function.
    pub fn get_attributes(&self) -> &[Attribute] {
        &self.data.attributes
    }

    /// Checks whether the function has an attribute.
    pub fn has_attribute(&self, pred: impl Fn(&Attribute) -> bool) -> bool {
        Attribute::has(&self.data.attributes, pred)
    }

    /// Checks whether this item is only used in tests.
    pub fn is_test_only(&self) -> bool {
        self.has_attribute(|a| {
            let s = self.symbol_pool().string(a.name());
            well_known::is_test_only_attribute_name(s.as_str())
        })
    }

    /// Checks whether this item is only used in verification.
    pub fn is_verify_only(&self) -> bool {
        self.has_attribute(|a| {
            let s = self.symbol_pool().string(a.name());
            well_known::is_verify_only_attribute_name(s.as_str())
        })
    }

    /// Returns the location of the specification block of this function. If the function has
    /// none, returns that of the function itself.
    pub fn get_spec_loc(&self) -> Loc {
        if let Some(loc) = &self.data.spec.borrow().loc {
            loc.clone()
        } else {
            self.get_loc()
        }
    }

    /// Returns the location of the bytecode at the given offset.
    pub fn get_bytecode_loc(&self, offset: u16) -> Option<Loc> {
        let source_map = self.module_env.data.source_map.as_ref()?;
        if let Ok(fmap) = source_map.get_function_source_map(self.data.def_idx?) {
            if let Some(loc) = fmap.get_code_location(offset) {
                let loc = self.module_env.env.to_loc(&loc);
                return Some(loc);
            }
        }
        Some(self.get_loc())
    }

    /// Returns the bytecode associated with this function, if a compiled module is attached.
    pub fn get_bytecode(&self) -> Option<&[Bytecode]> {
        let module = self.module_env.data.compiled_module.as_ref()?;
        let function_definition = module.function_def_at(self.get_def_idx()?);
        let function_definition_view = FunctionDefinitionView::new(module, function_definition);
        Some(match function_definition_view.code() {
            Some(code) => &code.code,
            None => &[],
        })
    }

    /// Returns the value of a boolean pragma for this function. This first looks up a
    /// pragma in this function, then the enclosing module, and finally uses the provided default
    /// value.
    pub fn is_pragma_true(&self, name: &str, default: impl FnOnce() -> bool) -> bool {
        let env = self.module_env.env;
        if let Some(b) = env.is_property_true(&self.get_spec().properties, name) {
            return b;
        }
        if let Some(b) = env.is_property_true(&self.module_env.get_spec().properties, name) {
            return b;
        }
        default()
    }

    /// Returns true if the value of a boolean pragma for this function is false.
    pub fn is_pragma_false(&self, name: &str) -> bool {
        let env = self.module_env.env;
        if let Some(b) = env.is_property_true(&self.get_spec().properties, name) {
            return !b;
        }
        if let Some(b) = env.is_property_true(&self.module_env.get_spec().properties, name) {
            return !b;
        }
        false
    }

    /// Returns the value of a numeric pragma for this function. This first looks up a pragma in
    /// this function, then the enclosing module, and finally uses the provided default value.
    pub fn get_num_pragma(&self, name: &str) -> Option<usize> {
        let env = self.module_env.env;
        if let Some(n) = env.get_num_property(&self.get_spec().properties, name) {
            return Some(n);
        }
        if let Some(n) = env.get_num_property(&self.module_env.get_spec().properties, name) {
            return Some(n);
        }
        None
    }

    /// Returns the value of a pragma representing an identifier for this function.
    /// If such pragma is not specified for this function, None is returned.
    pub fn get_ident_pragma(&self, name: &str) -> Option<Rc<String>> {
        let sym = &self.symbol_pool().make(name);
        match self.get_spec().properties.get(sym) {
            Some(PropertyValue::Symbol(sym)) => Some(self.symbol_pool().string(*sym)),
            Some(PropertyValue::QualifiedSymbol(qsym)) => {
                let module_name = qsym.module_name.display(self.module_env.env);
                Some(Rc::from(format!(
                    "{}::{}",
                    module_name,
                    self.symbol_pool().string(qsym.symbol)
                )))
            },
            _ => None,
        }
    }

    /// Returns the FunctionEnv of the function identified by the pragma, if the pragma
    /// exists and its value represents a function in the system.
    pub fn get_func_env_from_pragma(&self, name: &str) -> Option<FunctionEnv<'env>> {
        let sym = &self.symbol_pool().make(name);
        match self.get_spec().properties.get(sym) {
            Some(PropertyValue::Symbol(sym)) => self.module_env.find_function(*sym),
            Some(PropertyValue::QualifiedSymbol(qsym)) => {
                if let Some(module_env) = self.module_env.env.find_module(&qsym.module_name) {
                    module_env.find_function(qsym.symbol)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Returns true if this function has the pragma intrinsic set to true.
    pub fn is_intrinsic(&self) -> bool {
        self.is_pragma_true(INTRINSIC_PRAGMA, || {
            self.module_env
                .env
                .intrinsics
                .get_decl_for_move_fun(&self.get_qualified_id())
                .is_some()
        })
    }

    /// Returns true if function is either native or intrinsic.
    pub fn is_native_or_intrinsic(&self) -> bool {
        self.is_native() || self.is_intrinsic()
    }

    /// Returns true if this is an intrinsic struct of a given name
    pub fn is_intrinsic_of(&self, name: &str) -> bool {
        self.module_env.env.intrinsics.is_intrinsic_of_for_move_fun(
            self.symbol_pool(),
            &self.get_qualified_id(),
            name,
        )
    }

    /// Returns true if this is the well-known native or intrinsic function of the given name.
    /// The function must reside either in stdlib or extlib address domain.
    pub fn is_well_known(&self, name: &str) -> bool {
        let env = self.module_env.env;
        if !self.is_native_or_intrinsic() {
            return false;
        }
        let addr = self.module_env.get_name().addr();
        (addr == &env.get_stdlib_address() || addr == &env.get_extlib_address())
            && self.get_full_name_str() == name
    }

    /// Returns true if this function is opaque.
    pub fn is_opaque(&self) -> bool {
        self.is_pragma_true(OPAQUE_PRAGMA, || false)
    }

    /// Return the visibility of this function
    pub fn visibility(&self) -> Visibility {
        self.data.visibility
    }

    /// Returns true if this function is native.
    pub fn is_native(&self) -> bool {
        self.data.is_native
    }

    /// Return true if the function is an entry function
    pub fn is_entry(&self) -> bool {
        self.data.kind == FunctionKind::Entry
    }

    /// Return true if the function is an inline function
    pub fn is_inline(&self) -> bool {
        self.data.kind == FunctionKind::Inline
    }

    /// Returns kind of this function.
    pub fn get_kind(&self) -> FunctionKind {
        self.data.kind
    }

    /// Return the visibility string for this function. Useful for formatted printing.
    pub fn visibility_str(&self) -> &str {
        match self.visibility() {
            Visibility::Public => "public ",
            Visibility::Friend => "public(friend) ",
            Visibility::Private => "",
        }
    }

    /// Return whether this function is exposed outside of the module.
    pub fn is_exposed(&self) -> bool {
        self.module_env.is_script_module()
            || self.is_entry()
            || match self.visibility() {
                Visibility::Public | Visibility::Friend => true,
                Visibility::Private => false,
            }
    }

    /// Return whether this function is exposed outside of the module.
    pub fn has_unknown_callers(&self) -> bool {
        self.module_env.is_script_module()
            || self.is_entry()
            || match self.visibility() {
                Visibility::Public => true,
                Visibility::Private | Visibility::Friend => false,
            }
    }

    /// Returns true if the function is a script or entry function
    pub fn is_script_or_entry(&self) -> bool {
        // The main function of a script is a entry function
        self.module_env.is_script_module() || self.is_entry()
    }

    /// Return true if this function is a friend function
    pub fn is_friend(&self) -> bool {
        self.visibility() == Visibility::Friend
    }

    /// Return true iff this function is has package visibility
    pub fn has_package_visibility(&self) -> bool {
        self.data.has_package_visibility
    }

    /// Returns true if invariants are declared disabled in body of function
    pub fn are_invariants_disabled_in_body(&self) -> bool {
        self.is_pragma_true(DISABLE_INVARIANTS_IN_BODY_PRAGMA, || false)
    }

    /// Returns true if invariants are declared disabled in body of function
    pub fn are_invariants_disabled_at_call(&self) -> bool {
        self.is_pragma_true(DELEGATE_INVARIANTS_TO_CALLER_PRAGMA, || false)
    }

    /// Returns true if this function mutates any references (i.e. has &mut parameters).
    pub fn is_mutating(&self) -> bool {
        self.get_parameters()
            .iter()
            .any(|Parameter(_, ty, _)| ty.is_mutable_reference())
    }

    /// Returns the name of the friend(the only allowed caller) of this function, if there is one.
    pub fn get_friend_name(&self) -> Option<Rc<String>> {
        self.get_ident_pragma(FRIEND_PRAGMA)
    }

    /// Returns true if a friend is specified for this function.
    pub fn has_friend(&self) -> bool {
        self.get_friend_name().is_some()
    }

    /// Returns the FunctionEnv of the friend function if the friend is specified
    /// and the friend was compiled into the environment.
    pub fn get_friend_env(&self) -> Option<FunctionEnv<'env>> {
        self.get_func_env_from_pragma(FRIEND_PRAGMA)
    }

    /// Returns the FunctionEnv of the transitive friend of the function.
    /// For example, if `f` has a friend `g` and `g` has a friend `h`, then
    /// `f`'s transitive friend is `h`.
    /// If a friend is not specified then the function itself is returned.
    pub fn get_transitive_friend(&self) -> FunctionEnv<'env> {
        if let Some(friend_env) = self.get_friend_env() {
            return friend_env.get_transitive_friend();
        }
        self.clone()
    }

    /// Returns the type parameters associated with this function.
    pub fn get_type_parameters(&self) -> Vec<TypeParameter> {
        self.data.type_params.clone()
    }

    pub fn get_type_parameters_ref(&self) -> &[TypeParameter] {
        &self.data.type_params
    }

    pub fn get_parameter_count(&self) -> usize {
        self.data.params.len()
    }

    /// Return the number of type parameters for self
    pub fn get_type_parameter_count(&self) -> usize {
        self.data.type_params.len()
    }

    /// Return `true` if idx is a formal parameter index, in contrast to being a temporary.
    pub fn is_parameter(&self, idx: usize) -> bool {
        idx < self.get_parameter_count()
    }

    /// Return true if this is a named parameter of this function.
    pub fn is_named_parameter(&self, name: &str) -> bool {
        self.get_parameters()
            .iter()
            .any(|p| self.symbol_pool().string(p.0).as_ref() == name)
    }

    /// Returns the parameter types associated with this function
    pub fn get_parameter_types(&self) -> Vec<Type> {
        self.get_parameters()
            .into_iter()
            .map(|Parameter(_, ty, _)| ty)
            .collect()
    }

    /// Returns the regular parameters associated with this function.
    pub fn get_parameters(&self) -> Vec<Parameter> {
        self.data.params.clone()
    }

    /// Returns the regular parameters associated with this function.
    pub fn get_parameters_ref(&self) -> &[Parameter] {
        &self.data.params
    }

    /// Returns the result type of this function, which is a tuple for multiple results.
    pub fn get_result_type(&self) -> Type {
        self.data.result_type.clone()
    }

    /// Returns return type at given index.
    pub fn get_result_type_at(&self, idx: usize) -> Type {
        self.data.result_type.clone().flatten()[idx].clone()
    }

    /// Returns the number of return values of this function.
    pub fn get_return_count(&self) -> usize {
        if let Type::Tuple(ts) = &self.data.result_type {
            ts.len()
        } else {
            1
        }
    }

    /// Returns the access specifiers of this function.
    /// If this is `None`, all accesses are allowed. If the list is empty,
    /// no accesses are allowed. Otherwise the list is divided into _inclusions_ and _exclusions_,
    /// the later being negated specifiers. Access is allowed if (a) any of the inclusion
    /// specifiers allows it (union of inclusion specifiers) (b) none of the exclusions
    /// specifiers disallows it (intersection of exclusion specifiers).
    pub fn get_access_specifiers(&self) -> Option<&[AccessSpecifier]> {
        self.data.access_specifiers.as_deref()
    }

    /// Returns the inferred acquired structs of this function. This is checked
    /// against declared acquires from `get_access_specifiers`.
    pub fn get_acquired_structs(&self) -> Option<&BTreeSet<StructId>> {
        self.data.acquired_structs.as_ref()
    }

    /// Get the name to be used for a local by index, if available.
    /// Otherwise generate a unique name.
    pub fn get_local_name(&self, idx: usize) -> Symbol {
        if idx < self.data.params.len() {
            return self.data.params[idx].0;
        }
        if let Some(source_map) = &self.module_env.data.source_map {
            // Try to obtain user name from source map
            if let Some(fmap) = self
                .data
                .def_idx
                .and_then(|idx| source_map.get_function_source_map(idx).ok())
            {
                if let Some((ident, _)) = fmap.get_parameter_or_local_name(idx as u64) {
                    // The Move compiler produces temporary names of the form `<foo>%#<num>`,
                    // where <num> seems to be generated non-deterministically.
                    // Substitute this by a deterministic name which the backend accepts.
                    let clean_ident = if ident.contains("%#") {
                        format!("tmp#${}", idx)
                    } else {
                        ident
                    };
                    return self.module_env.env.symbol_pool.make(clean_ident.as_str());
                }
            }
        }
        self.module_env.env.symbol_pool.make(&format!("$t{}", idx))
    }

    /// Returns true if the index is for a temporary, not user declared local. Requires an
    /// attached compiled module.
    pub fn is_temporary(&self, idx: usize) -> Option<bool> {
        if idx >= self.get_local_count()? {
            return Some(true);
        }
        let name = self.get_local_name(idx);
        Some(self.symbol_pool().string(name).contains("tmp#$"))
    }

    /// Gets the number of proper locals of this function, if there is a bytecode module attached.
    /// Those are locals which are declared by the user and also have a user assigned name which
    /// can be discovered via `get_local_name`. Note we may have more anonymous locals generated
    /// e.g by the 'stackless' transformation.
    pub fn get_local_count(&self) -> Option<usize> {
        let view = self.definition_view()?;
        Some(match view.locals_signature() {
            Some(locals_view) => locals_view.len(),
            None => view.parameters().len(),
        })
    }

    /// Gets the type of the local at index. This must use an index in the range as determined by
    /// `get_local_count`.
    pub fn get_local_type(&self, idx: usize) -> Option<Type> {
        let view = self.definition_view()?;
        let parameters = view.parameters();
        if idx < parameters.len() {
            self.module_env.globalize_signature(&parameters.0[idx])
        } else {
            self.module_env.globalize_signature(
                view.locals_signature()
                    .unwrap()
                    .token_at(idx as u8)
                    .signature_token(),
            )
        }
    }

    /// Returns associated specification.
    pub fn get_spec(&'env self) -> Ref<'env, Spec> {
        self.data.spec.borrow()
    }

    /// Returns associated mutable reference to specification.
    pub fn get_mut_spec(&'env self) -> RefMut<'env, Spec> {
        self.data.spec.borrow_mut()
    }

    /// Returns associated definition if available.
    pub fn get_def(&self) -> Option<&Exp> {
        self.data.def.as_ref()
    }

    /// Returns the acquired global resource types, if a bytecode module is attached.
    pub fn get_acquires_global_resources(&'env self) -> Option<Vec<StructId>> {
        let module = self.module_env.data.compiled_module.as_ref()?;
        let function_definition = module.function_def_at(self.get_def_idx()?);
        Some(
            function_definition
                .acquires_global_resources
                .iter()
                .map(|x| self.module_env.get_struct_id(*x))
                .collect(),
        )
    }

    /// Computes the modified targets of the spec clause, as a map from resource type names to
    /// resource indices (list of types and address).
    pub fn get_modify_targets(&self) -> BTreeMap<QualifiedId<StructId>, Vec<Exp>> {
        // Compute the modify targets from `modifies` conditions.
        let spec = &self.get_spec();
        let modify_conditions = spec.filter_kind(ConditionKind::Modifies);
        let mut modify_targets: BTreeMap<QualifiedId<StructId>, Vec<Exp>> = BTreeMap::new();
        for cond in modify_conditions {
            cond.all_exps().for_each(|target| {
                let node_id = target.node_id();
                let rty = &self.module_env.env.get_node_instantiation(node_id)[0];
                let (mid, sid, _) = rty.require_struct();
                let type_name = mid.qualified(sid);
                modify_targets
                    .entry(type_name)
                    .or_default()
                    .push(target.clone());
            });
        }
        modify_targets
    }

    /// Determine whether the function is target of verification.
    pub fn should_verify(&self, default_scope: &VerificationScope) -> bool {
        if let VerificationScope::Only(function_name) = default_scope {
            // Overrides pragmas.
            return self.matches_name(function_name);
        }
        if !self.module_env.is_target() {
            // Don't generate verify method for functions from dependencies.
            return false;
        }

        // We look up the `verify` pragma property first in this function, then in
        // the module, and finally fall back to the value specified by default_scope.
        let default = || match default_scope {
            // By using `is_exposed`, we essentially mark all of Public, Script, Friend to be
            // in the verification scope because they are "exposed" functions in this module.
            // We may want to change `VerificationScope::Public` to `VerificationScope::Exposed` as
            // well for consistency.
            VerificationScope::Public => self.is_exposed(),
            VerificationScope::All => true,
            VerificationScope::Only(_) => unreachable!(),
            VerificationScope::OnlyModule(module_name) => self.module_env.matches_name(module_name),
            VerificationScope::None => false,
        };
        self.is_pragma_true(VERIFY_PRAGMA, default)
    }

    /// Returns true if either the name or simple name of this function matches the given string
    pub fn matches_name(&self, name: &str) -> bool {
        name.eq(&*self.get_simple_name_string()) || name.eq(&*self.get_name_string())
    }

    /// Determine whether this function is explicitly deactivated for verification.
    pub fn is_explicitly_not_verified(&self, scope: &VerificationScope) -> bool {
        !matches!(scope, VerificationScope::Only(..)) && self.is_pragma_false(VERIFY_PRAGMA)
    }

    /// Get the functions that use this one, if available.
    pub fn get_using_functions(&self) -> Option<BTreeSet<QualifiedId<FunId>>> {
        if let Some(using) = &*self.data.using_funs.borrow() {
            return Some(using.clone());
        }
        let mut set: BTreeSet<QualifiedId<FunId>> = BTreeSet::new();
        for module_env in self.module_env.env.get_modules() {
            for fun_env in module_env.get_functions() {
                if fun_env
                    .get_used_functions()?
                    .contains(&self.get_qualified_id())
                {
                    set.insert(fun_env.get_qualified_id());
                }
            }
        }
        *self.data.using_funs.borrow_mut() = Some(set.clone());
        Some(set)
    }

    /// Get the functions that this one uses, if available.
    pub fn get_used_functions(&self) -> Option<&'_ BTreeSet<QualifiedId<FunId>>> {
        self.data.used_funs.as_ref()
    }

    /// Get the transitive closure of the used functions. This requires that all functions
    /// in the closure have `get_used_functions` available; if one of them not, this
    /// function panics.
    pub fn get_transitive_closure_of_used_functions(&self) -> BTreeSet<QualifiedId<FunId>> {
        if let Some(trans_used) = &*self.data.transitive_closure_of_used_funs.borrow() {
            return trans_used.clone();
        }

        let mut set = BTreeSet::new();
        let mut reachable_funcs = VecDeque::new();
        reachable_funcs.push_back(self.clone());

        // BFS in reachable_funcs to collect all reachable functions
        while !reachable_funcs.is_empty() {
            let fnc = reachable_funcs.pop_front().unwrap();
            for callee in fnc.get_used_functions().expect("call info available") {
                let f = self.module_env.env.get_function(*callee);
                let qualified_id = f.get_qualified_id();
                if set.insert(qualified_id) {
                    reachable_funcs.push_back(f.clone());
                }
            }
        }
        *self.data.transitive_closure_of_used_funs.borrow_mut() = Some(set.clone());
        set
    }

    /// Get used functions including ones obtained by transitively traversing used inline functions
    pub fn get_used_functions_with_transitive_inline(&self) -> BTreeSet<QualifiedId<FunId>> {
        if let Some(trans_used) = &*self.data.used_functions_with_transitive_inline.borrow() {
            return trans_used.clone();
        }

        let mut set = BTreeSet::new();
        let mut reachable_funcs = VecDeque::new();
        reachable_funcs.push_back(self.clone());

        while let Some(fnc) = reachable_funcs.pop_front() {
            for callee in fnc.get_used_functions().expect("call info available") {
                let f = self.module_env.env.get_function(*callee);
                let qualified_id = f.get_qualified_id();
                if set.insert(qualified_id) && f.is_inline() {
                    reachable_funcs.push_back(f.clone());
                }
            }
        }
        *self.data.used_functions_with_transitive_inline.borrow_mut() = Some(set.clone());
        set
    }

    /// Get the functions that call this one, if available.
    pub fn get_calling_functions(&self) -> Option<BTreeSet<QualifiedId<FunId>>> {
        if let Some(calling) = &*self.data.calling_funs.borrow() {
            return Some(calling.clone());
        }
        let mut set: BTreeSet<QualifiedId<FunId>> = BTreeSet::new();
        for module_env in self.module_env.env.get_modules() {
            for fun_env in module_env.get_functions() {
                if fun_env
                    .get_called_functions()?
                    .contains(&self.get_qualified_id())
                {
                    set.insert(fun_env.get_qualified_id());
                }
            }
        }
        *self.data.calling_funs.borrow_mut() = Some(set.clone());
        Some(set)
    }

    /// Get the functions that this one calls, if available.
    pub fn get_called_functions(&self) -> Option<&'_ BTreeSet<QualifiedId<FunId>>> {
        self.data.called_funs.as_ref()
    }

    /// Get the transitive closure of the called functions. This requires that all functions
    /// in the closure have `get_called_functions` available; if one of them not, this
    /// function panics.
    pub fn get_transitive_closure_of_called_functions(&self) -> BTreeSet<QualifiedId<FunId>> {
        if let Some(trans_called) = &*self.data.transitive_closure_of_called_funs.borrow() {
            return trans_called.clone();
        }

        let mut set = BTreeSet::new();
        let mut reachable_funcs = VecDeque::new();
        reachable_funcs.push_back(self.clone());

        // BFS in reachable_funcs to collect all reachable functions
        while !reachable_funcs.is_empty() {
            let fnc = reachable_funcs.pop_front().unwrap();
            for callee in fnc.get_called_functions().expect("call info available") {
                let f = self.module_env.env.get_function(*callee);
                let qualified_id = f.get_qualified_id();
                if set.insert(qualified_id) {
                    reachable_funcs.push_back(f.clone());
                }
            }
        }
        *self.data.transitive_closure_of_called_funs.borrow_mut() = Some(set.clone());
        set
    }

    /// Returns the function name excluding the address and the module name
    pub fn get_simple_name_string(&self) -> Rc<String> {
        self.symbol_pool().string(self.get_name())
    }

    /// Returns a string representation of the functions 'header', as it is declared in Move.
    pub fn get_header_string(&self) -> String {
        let mut s = String::new();
        s.push_str(match self.data.visibility {
            Visibility::Private => "private",
            Visibility::Public => "public",
            Visibility::Friend => "friend",
        });
        s.push_str(match self.data.kind {
            FunctionKind::Regular => "",
            FunctionKind::Inline => " inline",
            FunctionKind::Entry => " entry",
        });
        if self.is_native() {
            s.push_str(" native")
        }
        write!(
            s,
            " fun {}{}",
            self.get_name().display(self.symbol_pool()),
            self.module_env.env.get_fun_signature_string(
                &self.get_type_display_ctx(),
                &self.data.type_params,
                &self.data.params,
                &self.data.result_type
            )
        )
        .unwrap();
        s
    }

    /// Returns the function name with the module name excluding the address
    pub fn get_name_string(&self) -> Rc<str> {
        if self.module_env.is_script_module() {
            Rc::from(format!("Script::{}", self.get_simple_name_string()))
        } else {
            let module_name = self.module_env.get_name().display(self.module_env.env);
            Rc::from(format!(
                "{}::{}",
                module_name,
                self.get_simple_name_string()
            ))
        }
    }

    fn definition_view(&'env self) -> Option<FunctionDefinitionView<'env, CompiledModule>> {
        if self.is_inline() {
            return None;
        }
        let module = self.module_env.data.compiled_module.as_ref()?;
        Some(FunctionDefinitionView::new(
            module,
            module.function_def_at(self.data.def_idx?),
        ))
    }

    /// Produce a TypeDisplayContext to print types within the scope of this env
    pub fn get_type_display_ctx(&self) -> TypeDisplayContext {
        let type_param_names = self
            .get_type_parameters()
            .iter()
            .map(|param| param.0)
            .collect();
        TypeDisplayContext {
            type_param_names: Some(type_param_names),
            ..self.module_env.get_type_display_ctx()
        }
    }
}

// =================================================================================================
/// # Expression Environment

/// Represents context for an expression.
#[derive(Debug, Clone)]
pub struct ExpInfo {
    /// The associated location of this expression.
    loc: Loc,
    /// The type of this expression.
    ty: Type,
    /// The associated instantiation of type parameters for this expression, if applicable
    instantiation: Option<Vec<Type>>,
}

impl ExpInfo {
    pub fn new(loc: Loc, ty: Type) -> Self {
        ExpInfo {
            loc,
            ty,
            instantiation: None,
        }
    }
}

// =================================================================================================
/// # Formatting

enum Mode {
    LineOnly,
    FileAndLine,
    Full,
}

pub struct LocDisplay<'env> {
    loc: &'env Loc,
    env: &'env GlobalEnv,
    mode: Mode,
}

impl Loc {
    pub fn display<'env>(&'env self, env: &'env GlobalEnv) -> LocDisplay<'env> {
        LocDisplay {
            loc: self,
            env,
            mode: Mode::Full,
        }
    }

    pub fn display_file_name_and_line<'env>(&'env self, env: &'env GlobalEnv) -> LocDisplay<'env> {
        LocDisplay {
            loc: self,
            env,
            mode: Mode::FileAndLine,
        }
    }

    pub fn display_line_only<'env>(&'env self, env: &'env GlobalEnv) -> LocDisplay<'env> {
        LocDisplay {
            loc: self,
            env,
            mode: Mode::LineOnly,
        }
    }
}

impl fmt::Display for LocDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((fname, pos)) = self.env.get_file_and_location(self.loc) {
            match &self.mode {
                Mode::LineOnly => {
                    write!(f, "at line {}", pos.line + LineOffset(1))
                },
                Mode::FileAndLine => {
                    write!(f, "at {}:{}", fname, pos.line + LineOffset(1))
                },
                Mode::Full => {
                    let offset = self.loc.span.end() - self.loc.span.start();
                    write!(
                        f,
                        "at {}:{}:{}+{}",
                        fname,
                        pos.line + LineOffset(1),
                        pos.column + ColumnOffset(1),
                        offset,
                    )
                },
            }
        } else {
            write!(f, "{:?}", self.loc)
        }
    }
}

pub trait GetNameString {
    fn get_name_for_display(&self, env: &GlobalEnv) -> String;
}

impl GetNameString for QualifiedId<StructId> {
    fn get_name_for_display(&self, env: &GlobalEnv) -> String {
        env.get_struct_qid(*self).get_full_name_str()
    }
}

impl GetNameString for QualifiedId<FunId> {
    fn get_name_for_display(&self, env: &GlobalEnv) -> String {
        env.get_function_qid(*self).get_full_name_str()
    }
}

impl GetNameString for QualifiedId<SpecFunId> {
    fn get_name_for_display(&self, env: &GlobalEnv) -> String {
        env.get_spec_fun(*self)
            .name
            .display(env.symbol_pool())
            .to_string()
    }
}

impl<'a, T> fmt::Display for EnvDisplay<'a, Vec<T>>
where
    EnvDisplay<'a, T>: std::fmt::Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let xs = self
            .val
            .iter()
            .map(|x| format!("{}", self.env.display(x)))
            .collect_vec();
        write!(f, "({})", xs.iter().join(","))
    }
}

impl<Id: Clone> fmt::Display for EnvDisplay<'_, QualifiedId<Id>>
where
    QualifiedId<Id>: GetNameString,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.val.get_name_for_display(self.env))
    }
}

impl<Id: Clone> fmt::Display for EnvDisplay<'_, QualifiedInstId<Id>>
where
    QualifiedId<Id>: GetNameString,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.env.display(&self.val.to_qualified_id()))?;
        if !self.val.inst.is_empty() {
            let tctx = TypeDisplayContext::new(self.env);
            write!(f, "<")?;
            let mut sep = "";
            for ty in &self.val.inst {
                write!(f, "{}{}", sep, ty.display(&tctx))?;
                sep = ", ";
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl fmt::Display for EnvDisplay<'_, Symbol> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.val.display(self.env.symbol_pool()))
    }
}

impl fmt::Display for EnvDisplay<'_, Parameter> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let p = self.val;
        write!(
            f,
            "{}:{}",
            p.get_name().display(self.env.symbol_pool()),
            p.get_type().display(&self.env.get_type_display_ctx())
        )
    }
}
