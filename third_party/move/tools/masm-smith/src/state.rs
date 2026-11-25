// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! What the generator knows while writing a function body: the types on the operand stack, which
//! locals hold values, the blocks so far, the outstanding references, and the structs and
//! functions it can use. Instruction generators consult it to emit only what the verifier
//! accepts.

use crate::{
    borrows::{BorrowGraph, RefId},
    config::{Config, DimensionFlag},
    error::Result,
    ir::{BlockId, Function, StructDecl, Type},
};
use arbitrary::Unstructured;
use move_core_types::{ability::AbilitySet, identifier::Identifier};
use shared_dsa::UnorderedMap;
use std::collections::{BTreeMap, BTreeSet};

/// Reserved name for the dedicated bounded-counter local that the CFG builder uses to
/// terminate any loop it emits. The signature pass plants this when `cfg.enable_loops` is
/// set; the body pass picks it up via [`FunctionState::loop_counter_idx`].
pub const LOOP_COUNTER_NAME: &str = "__loop_counter";

/// Initialization state of a local slot.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LocalInit {
    /// The slot has not been assigned a value yet.
    Uninit,
    /// The slot holds a usable value.
    Available,
    /// The value has been moved out.
    Moved,
}

/// Lattice meet on local-init states. `Available` is the only "usable" element; meeting two
/// disagreeing states yields a non-`Available` result so the generator (and the verifier)
/// treats the slot as not safe to read/move.
pub fn meet_local_init(a: LocalInit, b: LocalInit) -> LocalInit {
    if a == b {
        a
    } else {
        LocalInit::Moved
    }
}

#[derive(Clone, Debug)]
pub struct LocalSlot {
    pub name: Identifier,
    pub ty: Type,
    pub init: LocalInit,
}

/// A resource instance known to exist: `exists<resource<type_args>>(account)` held.
#[derive(Clone, Debug)]
pub struct GuaranteedResource {
    pub resource: crate::ir::QualifiedStruct,
    pub type_args: Vec<Type>,
    pub account: AccountSource,
}

/// Where a global operation's address comes from.
#[derive(Clone, Debug)]
pub enum AccountSource {
    Literal(move_core_types::account_address::AccountAddress),
    /// `signer::address_of` of a `&signer` local: the account the transaction's resources are
    /// stored under.
    Signer(Identifier),
}

/// State threaded through generation of a single function body.
pub struct FunctionState<'cfg, 'mod_> {
    pub cfg: &'cfg Config,
    pub module_ctx: &'mod_ ModuleCtx,

    /// Local slots indexed by `LocalIndex` (params first, then declared locals).
    pub locals: Vec<LocalSlot>,
    /// Constraints declared on this function's type parameters.
    pub type_param_constraints: Vec<AbilitySet>,
    /// Return types of the function being generated.
    pub return_types: Vec<Type>,
    /// Local key structs accessed by `borrow_global`, `mut_borrow_global`, or `move_from`.
    /// The orchestrator retains only these acquires annotations. `exists` and `move_to`
    /// require no annotation and are excluded.
    pub accessed_key_structs: BTreeSet<Identifier>,
    /// Key structs already stored by this function's generated `move_to` instructions.
    /// Suppresses repeated stores that could abort on an existing resource and discard writes.
    pub stored_key_structs: BTreeSet<Identifier>,
    /// Whether this function already calls a function that stores a resource; a second such
    /// call would usually store the same resource again.
    pub calls_storing_function: bool,
    /// Call sites (calls and packed closures) this function has per generated callee, bounded
    /// by [`Config::max_call_sites_per_callee`].
    pub call_sites: UnorderedMap<crate::ir::QualifiedFunction, u8>,
    /// A resource the current block may read without aborting: the block is entered only when
    /// `exists` held for it. Global reads target it; it is cleared at every block entry.
    pub guaranteed_resource: Option<GuaranteedResource>,
    /// The guarantee the next block inherits from the terminator being emitted.
    pub next_guaranteed_resource: Option<GuaranteedResource>,

    /// Operand stack — type of each pushed value, top of stack at the back.
    pub ty_stack: Vec<Type>,
    /// Parallel stack of borrow-graph IDs. Tracked references carry `Some(ref_id)`;
    /// other values, including untracked parameter references, carry `None`. Popping a
    /// tracked reference consumes its borrow-graph entry.
    pub ref_stack: Vec<Option<RefId>>,
    /// Operand stack depth at the start of each basic block, which the block's back edges
    /// drain to.
    pub block_entry_depths: BTreeMap<BlockId, usize>,
    /// Local initialization states at block entry, merged from predecessor exits.
    /// Equal states are retained; disagreements become `Moved`. A local is readable only
    /// when every merged predecessor leaves it `Available`.
    pub block_entry_locals: BTreeMap<BlockId, Vec<LocalInit>>,

    pub borrows: BorrowGraph,

    /// Dimensions actually violated during body generation. Bubbled up to
    /// [`crate::ir::MasmTest::injected_violations`] by the orchestrator.
    pub injected_violations: Vec<DimensionFlag>,

    /// Binary mutations to apply after assembly. The orchestrator pairs each intent with
    /// its publish-step index in [`crate::ir::PostProcessRecipe::intents`].
    pub post_process_intents: Vec<crate::ir::PostProcessIntent>,

    /// Count of `ld_const` instructions emitted by previous functions in this module.
    /// Seeded by the orchestrator at [`FunctionState::new`] time. Combined with
    /// [`Self::ld_const_count_in_function`] this gives the absolute constant-pool index
    /// the next `ld_const` will allocate (modulo assembler dedup — see
    /// [`crate::ir::PostProcessIntent::CorruptConstant`]).
    pub ld_const_base: u16,
    /// Number of `ld_const` instructions this function has emitted so far. Bumped via
    /// [`Self::record_ld_const_emit`] at every emit site.
    pub ld_const_count_in_function: u16,

    /// Index of the `__loop_counter` local reserved by the signature pass, if present.
    /// The CFG builder uses it to bound reducible loop iterations.
    pub loop_counter_idx: Option<u16>,

    /// Locals added during body generation; appended to the function's declared locals.
    pub scratch_locals: Vec<crate::ir::Local>,

    /// Name of the function whose body is being generated; `None` for scripts.
    pub function_name: Option<Identifier>,
}

/// Snapshot of module-level declarations visible during body generation.
#[derive(Default)]
pub struct ModuleCtx {
    /// Module identity (used when constructing `QualifiedStruct` / `QualifiedFunction`
    /// references for the current module's own declarations).
    pub module_id: Option<move_core_types::language_storage::ModuleId>,
    pub structs: Vec<StructDecl>,
    /// Functions declared in the *current* module. Populated with signatures (empty bodies)
    /// before any bodies are generated, so cross-function and self-referential calls inside
    /// the module can pick from the full set of siblings.
    pub functions: Vec<Function>,
    /// Already-published modules in the same test. Bodies generated under this context can
    /// reference any `public` (or this-module-is-friend) function from these modules.
    pub external_modules: Vec<crate::ir::Module>,
    /// Functions of the current module chosen for bounded recursion whose bodies may not be
    /// generated yet; functions with bodies are recognised by their prologue.
    pub recursive: BTreeSet<Identifier>,
}

impl ModuleCtx {
    /// Resolves `qs` to a struct declared in this module. Structs from other modules sharing
    /// the same name resolve to `None`: their definitions are not addressable by struct ops.
    pub fn local_struct(&self, qs: &crate::ir::QualifiedStruct) -> Option<&StructDecl> {
        if self.module_id.as_ref() != Some(&qs.module) {
            return None;
        }
        self.structs.iter().find(|decl| decl.name == qs.name)
    }
}

impl<'cfg, 'mod_> FunctionState<'cfg, 'mod_> {
    pub fn new(
        cfg: &'cfg Config,
        module_ctx: &'mod_ ModuleCtx,
        params: Vec<LocalSlot>,
        declared_locals: Vec<LocalSlot>,
        type_param_constraints: Vec<AbilitySet>,
        return_types: Vec<Type>,
        ld_const_base: u16,
    ) -> Self {
        let mut locals = params;
        locals.extend(declared_locals);
        let loop_counter_idx = locals
            .iter()
            .position(|s| s.name.as_str() == LOOP_COUNTER_NAME && matches!(s.ty, Type::U64))
            .map(|i| i as u16);
        Self {
            cfg,
            module_ctx,
            locals,
            type_param_constraints,
            return_types,
            accessed_key_structs: BTreeSet::new(),
            stored_key_structs: BTreeSet::new(),
            calls_storing_function: false,
            call_sites: UnorderedMap::new(),
            guaranteed_resource: None,
            next_guaranteed_resource: None,
            ty_stack: Vec::new(),
            ref_stack: Vec::new(),
            block_entry_depths: BTreeMap::new(),
            block_entry_locals: BTreeMap::new(),
            borrows: BorrowGraph::new(),
            injected_violations: Vec::new(),
            post_process_intents: Vec::new(),
            ld_const_base,
            ld_const_count_in_function: 0,
            loop_counter_idx,
            scratch_locals: Vec::new(),
            function_name: None,
        }
    }

    /// Adds a fresh local of type `ty`, returning its name, or `None` once the local count
    /// would exceed `cfg.locals_per_fun_max`. The local starts `Moved` so no other generator
    /// picks it; the caller owns it for one straight-line instruction sequence.
    pub fn alloc_scratch_local(&mut self, ty: Type) -> Option<Identifier> {
        if self.locals.len() >= usize::from(self.cfg.locals_per_fun_max) {
            return None;
        }
        let name = Identifier::new(format!("__scratch{}", self.scratch_locals.len()))
            .expect("scratch local names are identifiers");
        // Paths recorded before the slot existed reach their blocks without a value in it.
        for entry in self.block_entry_locals.values_mut() {
            entry.push(LocalInit::Uninit);
        }
        self.locals.push(LocalSlot {
            name: name.clone(),
            ty: ty.clone(),
            init: LocalInit::Moved,
        });
        self.scratch_locals.push(crate::ir::Local {
            name: name.clone(),
            ty,
        });
        Some(name)
    }

    /// Record that an `ld_const` instruction is being emitted. Returns the module-wide
    /// constant-pool index the assembler will most likely allocate for this constant
    /// (subject to assembler dedup — see [`crate::ir::PostProcessIntent::CorruptConstant`]).
    /// Call this at every `ld_const` emit site (whether or not the call is wrapped in a
    /// constants-violation hook) so the count stays accurate across the module.
    pub fn record_ld_const_emit(&mut self) -> u16 {
        let idx = self
            .ld_const_base
            .saturating_add(self.ld_const_count_in_function);
        self.ld_const_count_in_function = self.ld_const_count_in_function.saturating_add(1);
        idx
    }

    /// Decides and records a violation using this function's configuration. Callers must
    /// emit the violation whenever this returns `true`.
    pub fn should_violate(&mut self, u: &mut Unstructured<'_>, dim: DimensionFlag) -> Result<bool> {
        crate::violations::should_violate(u, self.cfg, dim, &mut self.injected_violations)
    }

    /// Decision-only wrapper over [`crate::violations::wants_violation`]: same dice roll as
    /// [`Self::should_violate`] but nothing is recorded. Pair with
    /// [`Self::record_violation`] at the point the violating shape is actually emitted, for
    /// generators whose positive decision can still fall back to valid output.
    pub fn wants_violation(&self, u: &mut Unstructured<'_>, dim: DimensionFlag) -> Result<bool> {
        crate::violations::wants_violation(u, self.cfg, dim)
    }

    /// Record that a violation for `dim` was actually emitted.
    pub fn record_violation(&mut self, dim: DimensionFlag) {
        self.injected_violations.push(dim);
    }

    pub fn push(&mut self, ty: Type) {
        self.ty_stack.push(ty);
        self.ref_stack.push(None);
    }

    pub fn push_ref(&mut self, ty: Type, ref_id: RefId) {
        self.ty_stack.push(ty);
        self.ref_stack.push(Some(ref_id));
    }

    pub fn pop(&mut self) -> Option<Type> {
        let ty = self.ty_stack.pop()?;
        if let Some(Some(ref_id)) = self.ref_stack.pop() {
            self.borrows.consume(ref_id);
        }
        Some(ty)
    }

    /// Like [`Self::pop`] but returns the [`RefId`] for the caller to release later (used
    /// by `read_ref` / `write_ref` which need to consume the borrow at a different point
    /// than the stack pop).
    pub fn pop_with_ref(&mut self) -> Option<(Type, Option<RefId>)> {
        let ty = self.ty_stack.pop()?;
        let ref_id = self.ref_stack.pop().flatten();
        Some((ty, ref_id))
    }

    pub fn top_ref_id(&self) -> Option<RefId> {
        self.ref_stack.last().copied().flatten()
    }

    pub fn top(&self) -> Option<&Type> {
        self.ty_stack.last()
    }

    /// Snapshot the operand stack at the entry of a basic block, and adopt any previously
    /// merged locals-entry state as the new "current" locals state. This is how forward
    /// joins from earlier predecessors take effect at the start of each block.
    pub fn record_block_entry(&mut self, id: BlockId) {
        self.guaranteed_resource = self.next_guaranteed_resource.take();
        self.block_entry_depths.insert(id, self.ty_stack.len());
        if let Some(entry) = self.block_entry_locals.get(&id).cloned() {
            for (slot, init) in self.locals.iter_mut().zip(entry) {
                slot.init = init;
            }
        } else {
            // Block has no recorded predecessor contribution — keep current state and seed
            // the entry map so subsequent merges have a baseline.
            let snap: Vec<LocalInit> = self.locals.iter().map(|l| l.init).collect();
            self.block_entry_locals.insert(id, snap);
        }
    }

    /// Merge the current (exit) locals state into the recorded entry state of `successor`,
    /// taking the conservative meet (Available only if both agree).
    pub fn merge_into_successor(&mut self, successor: BlockId) {
        let exit_locals: Vec<LocalInit> = self.locals.iter().map(|l| l.init).collect();
        match self.block_entry_locals.get_mut(&successor) {
            None => {
                self.block_entry_locals.insert(successor, exit_locals);
            },
            Some(existing) => {
                for (slot, incoming) in existing.iter_mut().zip(exit_locals) {
                    *slot = meet_local_init(*slot, incoming);
                }
            },
        }
    }

    pub fn stack_len(&self) -> usize {
        self.ty_stack.len()
    }

    pub fn local(&self, idx: u16) -> Option<&LocalSlot> {
        self.locals.get(idx as usize)
    }

    pub fn local_mut(&mut self, idx: u16) -> Option<&mut LocalSlot> {
        self.locals.get_mut(idx as usize)
    }

    /// Whether the local at `idx` currently has any outstanding borrow.
    pub fn is_local_borrowed(&self, idx: u16) -> bool {
        use crate::borrows::BorrowRoot;
        self.borrows
            .entries_for_root(&BorrowRoot::Local(idx))
            .next()
            .is_some()
    }

    /// Whether the local at `idx` currently has an outstanding *mutable* borrow.
    pub fn has_mut_borrow_on_local(&self, idx: u16) -> bool {
        use crate::borrows::{BorrowRoot, Mutability};
        self.borrows
            .entries_for_root(&BorrowRoot::Local(idx))
            .any(|entry| entry.mutability == Mutability::Mut)
    }
}
