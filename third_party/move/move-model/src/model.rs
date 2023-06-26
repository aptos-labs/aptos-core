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
        Address, Attribute, ConditionKind, Exp, ExpData, GlobalInvariant, ModuleName, PropertyBag,
        PropertyValue, Spec, SpecBlockInfo, SpecFunDecl, SpecVarDecl, Value,
    },
    intrinsics::IntrinsicsAnnotation,
    pragmas::{
        DELEGATE_INVARIANTS_TO_CALLER_PRAGMA, DISABLE_INVARIANTS_IN_BODY_PRAGMA, FRIEND_PRAGMA,
        INTRINSIC_PRAGMA, OPAQUE_PRAGMA, VERIFY_PRAGMA,
    },
    symbol::{Symbol, SymbolPool},
    ty::{PrimitiveType, Type, TypeDisplayContext, TypeUnificationAdapter, Variance},
};
use codespan::{ByteIndex, ByteOffset, ColumnOffset, FileId, Files, LineOffset, Location, Span};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label, Severity},
    term::{emit, termcolor::WriteColor, Config},
};
use itertools::Itertools;
#[allow(unused_imports)]
use log::{info, warn};
pub use move_binary_format::file_format::{AbilitySet, Visibility};
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    file_format::{
        Bytecode, CodeOffset, Constant as VMConstant, ConstantPoolIndex, FunctionDefinitionIndex,
        FunctionHandleIndex, SignatureIndex, SignatureToken, StructDefinitionIndex,
    },
    normalized::Type as MType,
    views::{FunctionDefinitionView, FunctionHandleView, StructHandleView},
    CompiledModule,
};
use move_bytecode_source_map::{mapping::SourceMapping, source_map::SourceMap};
use move_command_line_common::{address::NumericalAddress, files::FileHash};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage,
    value::MoveValue,
};
use move_disassembler::disassembler::{Disassembler, DisassemblerOptions};
use num::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::{BTreeMap, BTreeSet, VecDeque},
    ffi::OsStr,
    fmt::{self, Formatter},
    rc::Rc,
};

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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Loc {
    file_id: FileId,
    span: Span,
}

impl Loc {
    pub fn new(file_id: FileId, span: Span) -> Loc {
        Loc { file_id, span }
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
            Loc::new(
                self.file_id,
                Span::new(self.span.end() - ByteOffset(1), self.span.end()),
            )
        } else {
            self.clone()
        }
    }

    // Delivers a location pointing to the start of this one.
    pub fn at_start(&self) -> Loc {
        Loc::new(
            self.file_id,
            Span::new(self.span.start(), self.span.start() + ByteOffset(1)),
        )
    }

    /// Creates a location which encloses all the locations in the provided slice,
    /// which must not be empty. All locations are expected to be in the same file.
    pub fn enclosing(locs: &[&Loc]) -> Loc {
        assert!(!locs.is_empty());
        let loc = locs[0];
        let mut start = loc.span.start();
        let mut end = loc.span.end();
        for l in locs.iter().skip(1) {
            if l.file_id() == loc.file_id() {
                start = std::cmp::min(start, l.span().start());
                end = std::cmp::max(end, l.span().end());
            }
        }
        Loc::new(loc.file_id(), Span::new(start, end))
    }

    /// Returns true if the other location is enclosed by this location.
    pub fn is_enclosing(&self, other: &Loc) -> bool {
        self.file_id == other.file_id && GlobalEnv::enclosing_span(self.span, other.span)
    }
}

impl Default for Loc {
    fn default() -> Self {
        let mut files = Files::new();
        let dummy_id = files.add(String::new(), String::new());
        Loc::new(dummy_id, Span::default())
    }
}

/// Alias for the Loc variant of MoveIR. This uses a `&static str` instead of `FileId` for the
/// file name.
pub type MoveIrLoc = move_ir_types::location::Loc;

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
    /// A Files database for the codespan crate which supports diagnostics.
    pub(crate) source_files: Files<String>,
    /// A map of FileId in the Files database to information about documentation comments in a file.
    /// The comments are represented as map from ByteIndex into string, where the index is the
    /// start position of the associated language item in the source.
    pub(crate) doc_comments: BTreeMap<FileId, BTreeMap<ByteIndex, String>>,
    /// A mapping from file hash to file name and associated FileId. Though this information is
    /// already in `source_files`, we can't get it out of there so need to book keep here.
    pub(crate) file_hash_map: BTreeMap<FileHash, (String, FileId)>,
    /// A mapping from file id to associated alias map.
    pub(crate) file_alias_map: BTreeMap<FileId, Rc<BTreeMap<Symbol, NumericalAddress>>>,
    /// Bijective mapping between FileId and a plain int. FileId's are themselves wrappers around
    /// ints, but the inner representation is opaque and cannot be accessed. This is used so we
    /// can emit FileId's to generated code and read them back.
    pub(crate) file_id_to_idx: BTreeMap<FileId, u16>,
    pub(crate) file_idx_to_id: BTreeMap<u16, FileId>,
    /// A set indicating whether a file id is a target or a dependency.
    pub(crate) file_id_is_dep: BTreeSet<FileId>,
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
    /// The address of the standard and extension libaries.
    pub(crate) stdlib_address: Option<Address>,
    pub(crate) extlib_address: Option<Address>,
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
        let mut file_id_to_idx = BTreeMap::new();
        let mut file_idx_to_id = BTreeMap::new();
        let mut fake_loc = |content: &str| {
            let file_id = source_files.add(content, content.to_string());
            let file_hash = FileHash::new(content);
            file_hash_map.insert(file_hash, (content.to_string(), file_id));
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
            source_files,
            doc_comments: Default::default(),
            unknown_loc,
            unknown_move_ir_loc,
            internal_loc,
            file_hash_map,
            file_alias_map: BTreeMap::new(),
            file_id_to_idx,
            file_idx_to_id,
            file_id_is_dep: BTreeSet::new(),
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
        }
    }

    /// Creates a display container for the given value. There must be an implementation
    /// of fmt::Display for an instance to work in formatting.
    pub fn display<'a, T>(&'a self, val: &'a T) -> EnvDisplay<'a, T> {
        EnvDisplay { env: self, val }
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
        is_dep: bool,
    ) -> FileId {
        let file_id = self.source_files.add(file_name, source.to_string());
        self.stdlib_address =
            self.resolve_std_address_alias(self.stdlib_address.clone(), "std", &address_aliases);
        self.extlib_address = self.resolve_std_address_alias(
            self.extlib_address.clone(),
            "Extensions",
            &address_aliases,
        );
        self.file_alias_map.insert(file_id, address_aliases);
        self.file_hash_map
            .insert(file_hash, (file_name.to_string(), file_id));
        let file_idx = self.file_id_to_idx.len() as u16;
        self.file_id_to_idx.insert(file_id, file_idx);
        self.file_idx_to_id.insert(file_idx, file_id);
        if is_dep {
            self.file_id_is_dep.insert(file_id);
        }
        file_id
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

    /// Adds an error to this environment, with notes.
    pub fn error_with_notes(&self, loc: &Loc, msg: &str, notes: Vec<String>) {
        self.diag_with_notes(Severity::Error, loc, msg, notes)
    }

    /// Adds a diagnostic of given severity to this environment.
    pub fn diag(&self, severity: Severity, loc: &Loc, msg: &str) {
        let diag = Diagnostic::new(severity)
            .with_message(msg)
            .with_labels(vec![Label::primary(loc.file_id, loc.span)]);
        self.add_diag(diag);
    }

    /// Adds a diagnostic of given severity to this environment, with notes.
    pub fn diag_with_notes(&self, severity: Severity, loc: &Loc, msg: &str, notes: Vec<String>) {
        let diag = Diagnostic::new(severity)
            .with_message(msg)
            .with_labels(vec![Label::primary(loc.file_id, loc.span)]);
        let diag = diag.with_notes(notes);
        self.add_diag(diag);
    }

    /// Adds a diagnostic of given severity to this environment, with secondary labels.
    pub fn diag_with_labels(
        &self,
        severity: Severity,
        loc: &Loc,
        msg: &str,
        labels: Vec<(Loc, String)>,
    ) {
        let diag = Diagnostic::new(severity)
            .with_message(msg)
            .with_labels(vec![Label::primary(loc.file_id, loc.span)]);
        let labels = labels
            .into_iter()
            .map(|(l, m)| Label::secondary(l.file_id, l.span).with_message(m))
            .collect_vec();
        let diag = diag.with_labels(labels);
        self.add_diag(diag);
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

    /// Converts a Loc as used by the move-compiler compiler to the one we are using here.
    /// TODO: move-compiler should use FileId as well so we don't need this here. There is already
    /// a todo in their code to remove the current use of `&'static str` for file names in Loc.
    pub fn to_loc(&self, loc: &MoveIrLoc) -> Loc {
        let file_id = self.get_file_id(loc.file_hash()).unwrap_or_else(|| {
            panic!(
                "Unable to find source file '{}' in the environment",
                loc.file_hash()
            )
        });
        Loc {
            file_id,
            span: Span::new(loc.start(), loc.end()),
        }
    }

    /// Returns the file id for a file name, if defined.
    pub fn get_file_id(&self, fhash: FileHash) -> Option<FileId> {
        self.file_hash_map.get(&fhash).map(|(_, id)| id).cloned()
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
        self.report_diag_with_filter(writer, |d| d.severity >= severity)
    }

    /// Writes accumulated diagnostics that pass through `filter`
    pub fn report_diag_with_filter<W: WriteColor, F: Fn(&Diagnostic<FileId>) -> bool>(
        &self,
        writer: &mut W,
        filter: F,
    ) {
        let mut shown = BTreeSet::new();
        for (diag, reported) in self
            .diags
            .borrow_mut()
            .iter_mut()
            .filter(|(d, _)| filter(d))
        {
            if !*reported {
                // Avoid showing the same message twice. This can happen e.g. because of
                // duplication of expressions via schema inclusion.
                if shown.insert(format!("{:?}", diag)) {
                    emit(writer, &Config::default(), &self.source_files, diag)
                        .expect("emit must not fail");
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
                .or_insert_with(BTreeSet::new)
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
            let rel = adapter.unify(Variance::Allow, true);
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
            decl.callees.contains(&fun)
                || decl
                    .callees
                    .iter()
                    .any(|trans_caller| is_caller(env, visited, *trans_caller, fun))
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
            module_spec,
            loc,
            attributes,
            spec_block_infos,
            used_modules: Default::default(),
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
            if let Some(mut struct_data) = mod_data.struct_data.get_mut(&struct_id) {
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
            let fun_id = if name_str == SCRIPT_BYTECODE_FUN_NAME {
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

            // While releasing any mutation, compute the called functions if needed.
            let fun_data = &self.module_data[module_id.0 as usize]
                .function_data
                .get(&fun_id)
                .unwrap();
            let called_funs = if fun_data.called_funs.is_none() {
                Some(self.get_called_funs_from_bytecode(&module, def_idx))
            } else {
                None
            };

            let mod_data = &mut self.module_data[module_id.0 as usize];
            if let Some(mut fun_data) = mod_data.function_data.get_mut(&fun_id) {
                fun_data.def_idx = Some(def_idx);
                fun_data.handle_idx = Some(handle_idx);
                mod_data.function_idx_to_id.insert(def_idx, fun_id);
                if let Some(called_funs) = called_funs {
                    fun_data.called_funs = Some(called_funs);
                }
            } else {
                panic!("attaching mismatching bytecode module")
            }
        }

        let used_modules = self.get_used_modules_from_bytecode(&module);
        let friend_modules = self.get_friend_modules_from_bytecode(&module);
        let mut mod_data = &mut self.module_data[module_id.0 as usize];
        mod_data.used_modules = used_modules;
        mod_data.friend_modules = friend_modules;
        mod_data.compiled_module = Some(module);
        mod_data.source_map = Some(source_map);
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
            offset: 0,
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
            spec: Spec::default(),
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

    /// Return the `StructEnv` for `str`
    pub fn get_struct(&self, str: QualifiedId<StructId>) -> StructEnv<'_> {
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
            &storage_id.address().to_string(),
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
        module_data.module_spec = spec;
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
}

impl Default for GlobalEnv {
    fn default() -> Self {
        Self::new()
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
    attributes: Vec<Attribute>,

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
    pub(crate) module_spec: Spec,

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

/// Represents a module environment.
#[derive(Debug, Clone)]
pub struct ModuleEnv<'env> {
    /// Reference to the outer env.
    pub env: &'env GlobalEnv,

    /// Reference to the data of the module.
    data: &'env ModuleData,
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

    /// Returns true of this module is target of compilation. A non-target module is
    /// a dependency only but not explicitly requested to process.
    pub fn is_target(&self) -> bool {
        let file_id = self.data.loc.file_id;
        !self.env.file_id_is_dep.contains(&file_id)
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
            add_usage_of_spec(&mut usage, self.get_spec());
            for struct_env in self.get_structs() {
                add_usage_of_spec(&mut usage, struct_env.get_spec())
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

    /// Gets the underlying bytecode module, if one is attached.
    pub fn get_verified_module(&'env self) -> Option<&'env CompiledModule> {
        self.data.compiled_module.as_ref()
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
        let data = self.data.function_data.get(&id).expect("FunId undefined");
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
        Some(self.internal_globalize_signature(self.data.compiled_module.as_ref()?, sig))
    }

    pub(crate) fn internal_globalize_signature(
        &self,
        module: &CompiledModule,
        sig: &SignatureToken,
    ) -> Type {
        match sig {
            SignatureToken::Bool => Type::Primitive(PrimitiveType::Bool),
            SignatureToken::U8 => Type::Primitive(PrimitiveType::U8),
            SignatureToken::U16 => Type::Primitive(PrimitiveType::U16),
            SignatureToken::U32 => Type::Primitive(PrimitiveType::U32),
            SignatureToken::U64 => Type::Primitive(PrimitiveType::U64),
            SignatureToken::U128 => Type::Primitive(PrimitiveType::U128),
            SignatureToken::U256 => Type::Primitive(PrimitiveType::U256),
            SignatureToken::Address => Type::Primitive(PrimitiveType::Address),
            SignatureToken::Signer => Type::Primitive(PrimitiveType::Signer),
            SignatureToken::Reference(t) => Type::Reference(
                false,
                Box::new(self.internal_globalize_signature(module, t)),
            ),
            SignatureToken::MutableReference(t) => {
                Type::Reference(true, Box::new(self.internal_globalize_signature(module, t)))
            },
            SignatureToken::TypeParameter(index) => Type::TypeParameter(*index),
            SignatureToken::Vector(bt) => {
                Type::Vector(Box::new(self.internal_globalize_signature(module, bt)))
            },
            SignatureToken::Struct(handle_idx) => {
                let struct_view =
                    StructHandleView::new(module, module.struct_handle_at(*handle_idx));
                let declaring_module_env = self
                    .env
                    .find_module(&self.env.to_module_name(&struct_view.module_id()))
                    .expect("undefined module");
                let struct_env = declaring_module_env
                    .find_struct(self.env.symbol_pool.make(struct_view.name().as_str()))
                    .expect("undefined struct");
                Type::Struct(declaring_module_env.data.id, struct_env.get_id(), vec![])
            },
            SignatureToken::StructInstantiation(handle_idx, args) => {
                let struct_view =
                    StructHandleView::new(module, module.struct_handle_at(*handle_idx));
                let declaring_module_env = self
                    .env
                    .find_module(&self.env.to_module_name(&struct_view.module_id()))
                    .expect("undefined module");
                let struct_env = declaring_module_env
                    .find_struct(self.env.symbol_pool.make(struct_view.name().as_str()))
                    .expect("undefined struct");
                Type::Struct(
                    declaring_module_env.data.id,
                    struct_env.get_id(),
                    self.internal_globalize_signatures(module, args),
                )
            },
        }
    }

    /// Globalizes a list of signatures.
    pub fn globalize_signatures(&self, sigs: &[SignatureToken]) -> Option<Vec<Type>> {
        Some(self.internal_globalize_signatures(self.data.compiled_module.as_ref()?, sigs))
    }

    pub(crate) fn internal_globalize_signatures(
        &self,
        module: &CompiledModule,
        sigs: &[SignatureToken],
    ) -> Vec<Type> {
        sigs.iter()
            .map(|sig| self.internal_globalize_signature(module, sig))
            .collect()
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
    pub fn get_spec(&self) -> &Spec {
        &self.data.module_spec
    }

    /// Returns whether a spec fun is ever called or not.
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
        let disas = Disassembler::new(
            SourceMapping::new(
                self.data.source_map.clone()?,
                BinaryIndexedView::Module(self.get_verified_module()?),
            ),
            DisassemblerOptions {
                only_externally_visible: false,
                print_code: true,
                print_basic_blocks: true,
                print_locals: true,
            },
        );
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

    /// Associated specification.
    pub(crate) spec: Spec,
}

#[derive(Debug, Clone)]
pub struct StructEnv<'env> {
    /// Reference to enclosing module.
    pub module_env: ModuleEnv<'env>,

    /// Reference to the struct data.
    data: &'env StructData,
}

impl<'env> StructEnv<'env> {
    /// Returns the name of this struct.
    pub fn get_name(&self) -> Symbol {
        self.data.name
    }

    /// Gets full name as string.
    pub fn get_full_name_str(&self) -> String {
        format!(
            "{}::{}",
            self.module_env.get_name().display(self.module_env.env),
            self.get_name().display(self.symbol_pool())
        )
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

    /// Get documentation associated with this struct.
    pub fn get_doc(&self) -> &str {
        self.module_env.env.get_doc(&self.data.loc)
    }

    /// Returns properties from pragmas.
    pub fn get_properties(&self) -> &PropertyBag {
        &self.data.spec.properties
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

    /// Get an iterator for the fields, ordered by offset.
    pub fn get_fields(&'env self) -> impl Iterator<Item = FieldEnv<'env>> {
        self.data
            .field_data
            .values()
            .sorted_by_key(|data| data.offset)
            .map(move |data| FieldEnv {
                struct_env: self.clone(),
                data,
            })
    }

    /// Return the number of fields in the struct.
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
        for data in self.data.field_data.values() {
            if data.offset == offset {
                return FieldEnv {
                    struct_env: self.clone(),
                    data,
                };
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
        !self.data.spec.conditions.is_empty()
    }

    /// Returns the data invariants associated with this struct.
    pub fn get_spec(&'env self) -> &'env Spec {
        &self.data.spec
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
}

// =================================================================================================
/// # Field Environment

#[derive(Debug)]
pub struct FieldData {
    /// The name of this field.
    pub name: Symbol,

    /// The offset of this field.
    pub offset: usize,

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

impl<'env> FieldEnv<'env> {
    /// Gets the name of this field.
    pub fn get_name(&self) -> Symbol {
        self.data.name
    }

    /// Gets the id of this field.
    pub fn get_id(&self) -> FieldId {
        FieldId(self.data.name)
    }

    /// Get documentation associated with this field.
    pub fn get_doc(&self) -> &str {
        if let (Some(def_idx), Some(mmap)) = (
            self.struct_env.data.def_idx,
            &self.struct_env.module_env.data.source_map,
        ) {
            if let Ok(smap) = mmap.get_struct_source_map(def_idx) {
                let loc = self
                    .struct_env
                    .module_env
                    .env
                    .to_loc(&smap.fields[self.data.offset]);
                self.struct_env.module_env.env.get_doc(&loc)
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

impl<'env> NamedConstantEnv<'env> {
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
}

// =================================================================================================
/// # Function Environment

/// Represents a type parameter.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeParameter(pub Symbol, pub TypeParameterKind);

impl TypeParameter {
    /// Creates a new type parameter of given name.
    pub fn new_named(sym: &Symbol) -> Self {
        Self(*sym, TypeParameterKind::default())
    }

    /// Turns an ordered list of type parameters into a vector of type parameters
    pub fn vec_to_formals(params: &[TypeParameter]) -> Vec<Type> {
        params
            .iter()
            .enumerate()
            .map(|(pos, _)| Type::new_param(pos))
            .collect()
    }

    pub fn from_symbols<'a>(symbols: impl Iterator<Item = &'a Symbol>) -> Vec<TypeParameter> {
        symbols
            .map(|name| TypeParameter(*name, TypeParameterKind::default()))
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
pub struct Parameter(pub Symbol, pub Type);

#[derive(Debug)]
pub struct FunctionData {
    /// Name of this function.
    pub(crate) name: Symbol,

    /// Location of this function.
    pub(crate) loc: Loc,

    /// The definition index of this function in its bytecode module, if a bytecode module
    /// is attached to the parent module data.
    pub(crate) def_idx: Option<FunctionDefinitionIndex>,

    /// The handle index of this function in its modul, if a bytecode module
    /// is attached to the parent module data.
    pub(crate) handle_idx: Option<FunctionHandleIndex>,

    /// Visibility of this function (private, friend, or public)
    pub(crate) visibility: Visibility,

    /// Whether this is a native function
    pub(crate) is_native: bool,

    /// Whether this an entry function.
    pub(crate) is_entry: bool,

    /// Whether this is an inline function.
    pub(crate) is_inline: bool,

    /// Attributes attached to this function.
    pub(crate) attributes: Vec<Attribute>,

    /// Type parameters.
    pub(crate) type_params: Vec<TypeParameter>,

    /// Parameters
    pub(crate) params: Vec<Parameter>,

    /// Result type of the function, uses `Type::Tuple` for multiple values.
    pub(crate) result_type: Type,

    /// Specification associated with this function.
    pub(crate) spec: RefCell<Spec>,

    /// Optional definition associated with this function. The definition is available if
    /// the model is build with option `ModelBuilderOptions::compile_via_model`.
    pub(crate) def: Option<Exp>,

    /// A cache for the called functions.
    pub(crate) called_funs: Option<BTreeSet<QualifiedId<FunId>>>,

    /// A cache for the calling functions.
    pub(crate) calling_funs: RefCell<Option<BTreeSet<QualifiedId<FunId>>>>,

    /// A cache for the transitive closure of the called functions.
    pub(crate) transitive_closure_of_called_funs: RefCell<Option<BTreeSet<QualifiedId<FunId>>>>,
}

#[derive(Debug, Clone)]
pub struct FunctionEnv<'env> {
    /// Reference to enclosing module.
    pub module_env: ModuleEnv<'env>,

    /// Reference to the function data.
    data: &'env FunctionData,
}

impl<'env> FunctionEnv<'env> {
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
        self.module_env.env.get_doc(&self.data.loc)
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
        self.data.loc.clone()
    }

    /// Returns the attributes of this function.
    pub fn get_attributes(&self) -> &[Attribute] {
        &self.data.attributes
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
                return Some(self.module_env.env.to_loc(&loc));
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
    /// pragma in this function, then the enclosing module, and finally uses the provided default.
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

    /// Return true if the function is an entry fucntion
    pub fn is_entry(&self) -> bool {
        self.data.is_entry
    }

    /// Return true if the function is an inline fucntion
    pub fn is_inline(&self) -> bool {
        self.data.is_inline
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
            .any(|Parameter(_, ty)| ty.is_mutable_reference())
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
            .map(|Parameter(_, ty)| ty)
            .collect()
    }

    /// Returns the regular parameters associated with this function.
    pub fn get_parameters(&self) -> Vec<Parameter> {
        self.data.params.clone()
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

    /// Get the name to be used for a local by index, if a compiled module and source map
    /// is attached. If the local is an argument, use that for naming, otherwise generate
    /// a unique name.
    pub fn get_local_name(&self, idx: usize) -> Option<Symbol> {
        if idx < self.data.params.len() {
            return Some(self.data.params[idx].0);
        }
        // Try to obtain name from source map.
        let source_map = self.module_env.data.source_map.as_ref()?;
        if let Ok(fmap) = source_map.get_function_source_map(self.data.def_idx?) {
            if let Some((ident, _)) = fmap.get_parameter_or_local_name(idx as u64) {
                // The Move compiler produces temporary names of the form `<foo>%#<num>`,
                // where <num> seems to be generated non-deterministically.
                // Substitute this by a deterministic name which the backend accepts.
                let clean_ident = if ident.contains("%#") {
                    format!("tmp#${}", idx)
                } else {
                    ident
                };
                return Some(self.module_env.env.symbol_pool.make(clean_ident.as_str()));
            }
        }
        Some(self.module_env.env.symbol_pool.make(&format!("$t{}", idx)))
    }

    /// Returns true if the index is for a temporary, not user declared local. Requires an
    /// attached compiled module.
    pub fn is_temporary(&self, idx: usize) -> Option<bool> {
        if idx >= self.get_local_count()? {
            return Some(true);
        }
        let name = self.get_local_name(idx)?;
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
    pub fn get_spec(&'env self) -> Ref<Spec> {
        self.data.spec.borrow()
    }

    /// Returns associated mutable reference to specification.
    pub fn get_mut_spec(&'env self) -> RefMut<Spec> {
        self.data.spec.borrow_mut()
    }

    /// Returns associated definition. The definition of the function, in Exp form, is available
    /// if the model is build with `ModelBuilderOptions::compile_via_model`
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
                    .or_insert_with(Vec::new)
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
    /// in the closure have `get_called_functions` available; if one of them not, None is returned.
    pub fn get_transitive_closure_of_called_functions(
        &self,
    ) -> Option<BTreeSet<QualifiedId<FunId>>> {
        if let Some(trans_called) = &*self.data.transitive_closure_of_called_funs.borrow() {
            return Some(trans_called.clone());
        }

        let mut set = BTreeSet::new();
        let mut reachable_funcs = VecDeque::new();
        reachable_funcs.push_back(self.clone());

        // BFS in reachable_funcs to collect all reachable functions
        while !reachable_funcs.is_empty() {
            let fnc = reachable_funcs.pop_front().unwrap();
            if let Some(callees) = fnc.get_called_functions() {
                for callee in callees {
                    let f = self.module_env.env.get_function(*callee);
                    let qualified_id = f.get_qualified_id();
                    if !set.contains(&qualified_id) {
                        set.insert(qualified_id);
                        reachable_funcs.push_back(f.clone());
                    }
                }
            } else {
                return None;
            }
        }
        *self.data.transitive_closure_of_called_funs.borrow_mut() = Some(set.clone());
        Some(set)
    }

    /// Returns the function name excluding the address and the module name
    pub fn get_simple_name_string(&self) -> Rc<String> {
        self.symbol_pool().string(self.get_name())
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
        assert!(
            !self.is_inline(),
            "attempt to access bytecode info for inline function"
        );
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
        TypeDisplayContext::new_with_params(self.module_env.env, type_param_names)
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

pub struct LocDisplay<'env> {
    loc: &'env Loc,
    env: &'env GlobalEnv,
    only_line: bool,
}

impl Loc {
    pub fn display<'env>(&'env self, env: &'env GlobalEnv) -> LocDisplay<'env> {
        LocDisplay {
            loc: self,
            env,
            only_line: false,
        }
    }

    pub fn display_line_only<'env>(&'env self, env: &'env GlobalEnv) -> LocDisplay<'env> {
        LocDisplay {
            loc: self,
            env,
            only_line: true,
        }
    }
}

impl<'env> fmt::Display for LocDisplay<'env> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((fname, pos)) = self.env.get_file_and_location(self.loc) {
            if self.only_line {
                write!(f, "at {}:{}", fname, pos.line + LineOffset(1))
            } else {
                let offset = self.loc.span.end() - self.loc.span.start();
                write!(
                    f,
                    "at {}:{}:{}+{}",
                    fname,
                    pos.line + LineOffset(1),
                    pos.column + ColumnOffset(1),
                    offset,
                )
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

impl<'a, Id: Clone> fmt::Display for EnvDisplay<'a, QualifiedId<Id>>
where
    QualifiedId<Id>: GetNameString,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.val.get_name_for_display(self.env))
    }
}

impl<'a, Id: Clone> fmt::Display for EnvDisplay<'a, QualifiedInstId<Id>>
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
