// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tracks the references a function body holds, so the generator only takes new borrows that the
//! verifier's reference-safety check allows.

use crate::ir::QualifiedStruct;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RefId(pub u32);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BorrowRoot {
    /// Reference into a local slot (index into the function's params+locals).
    Local(u16),
    /// Reference into a global resource, keyed by struct name alone: borrows of different
    /// instantiations or addresses conservatively conflict.
    Global { struct_name: String },
}

impl BorrowRoot {
    pub fn global(resource: &QualifiedStruct) -> Self {
        BorrowRoot::Global {
            struct_name: resource.name.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BorrowStep {
    Field(u16),
    VariantField { variant: u16, field: u16 },
}

pub type BorrowPath = Vec<BorrowStep>;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mutability {
    Mut,
    Imm,
}

impl Mutability {
    pub fn of(is_mut: bool) -> Self {
        if is_mut {
            Mutability::Mut
        } else {
            Mutability::Imm
        }
    }
}

#[derive(Clone, Debug)]
pub struct BorrowEntry {
    pub root: BorrowRoot,
    pub path: BorrowPath,
    pub mutability: Mutability,
    pub parent: Option<RefId>,
}

#[derive(Clone, Debug, Default)]
pub struct BorrowGraph {
    next_id: u32,
    entries: BTreeMap<RefId, BorrowEntry>,
    by_root: BTreeMap<BorrowRoot, BTreeSet<RefId>>,
    children: BTreeMap<RefId, BTreeSet<RefId>>,
}

impl BorrowGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn outstanding_count(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, id: RefId) -> Option<&BorrowEntry> {
        self.entries.get(&id)
    }

    /// Iterates over the [`BorrowEntry`]s rooted at `root` without scanning other roots.
    pub fn entries_for_root<'a>(
        &'a self,
        root: &BorrowRoot,
    ) -> impl Iterator<Item = &'a BorrowEntry> + 'a {
        self.by_root
            .get(root)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.entries.get(id))
    }

    /// Conflict rule:
    ///   - A new `Mut` borrow at `(root, path)` is allowed iff there are no outstanding borrows
    ///     that overlap `(root, path)` (overlap = prefix containment along the path).
    ///   - A new `Imm` borrow at `(root, path)` is allowed iff there are no outstanding `Mut`
    ///     borrows overlapping `(root, path)`.
    pub fn can_borrow(
        &self,
        root: &BorrowRoot,
        path: &[BorrowStep],
        mutability: Mutability,
    ) -> bool {
        let Some(siblings) = self.by_root.get(root) else {
            return true;
        };
        for id in siblings {
            let Some(entry) = self.entries.get(id) else {
                continue;
            };
            if !overlaps(&entry.path, path) {
                continue;
            }
            match (mutability, entry.mutability) {
                (Mutability::Mut, _) | (_, Mutability::Mut) => return false,
                (Mutability::Imm, Mutability::Imm) => {},
            }
        }
        true
    }

    /// Creates a reference. Callers must first check [`Self::can_borrow`]; conflicts panic
    /// only when debug assertions are enabled. Use [`Self::force_create`] to bypass this check.
    pub fn create(
        &mut self,
        root: BorrowRoot,
        path: BorrowPath,
        mutability: Mutability,
        parent: Option<RefId>,
    ) -> RefId {
        debug_assert!(
            self.can_borrow(&root, &path, mutability),
            "BorrowGraph::create called with conflicting borrow"
        );
        self.force_create(root, path, mutability, parent)
    }

    /// Force-create a reference without conflict checking. Used by the violation injector
    /// when `valid_reference_safety = false`.
    pub fn force_create(
        &mut self,
        root: BorrowRoot,
        path: BorrowPath,
        mutability: Mutability,
        parent: Option<RefId>,
    ) -> RefId {
        let id = RefId(self.next_id);
        self.next_id = self.next_id.checked_add(1).expect("RefId overflow");
        self.by_root.entry(root.clone()).or_default().insert(id);
        if let Some(p) = parent {
            self.children.entry(p).or_default().insert(id);
        }
        self.entries.insert(id, BorrowEntry {
            root,
            path,
            mutability,
            parent,
        });
        id
    }

    /// Drop a reference and all its descendants. Used when an outstanding ref is consumed
    /// (read_ref, write_ref, returning a non-ref, etc.).
    pub fn consume(&mut self, id: RefId) {
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if let Some(kids) = self.children.remove(&cur) {
                stack.extend(kids);
            }
            if let Some(entry) = self.entries.remove(&cur) {
                if let Some(set) = self.by_root.get_mut(&entry.root) {
                    set.remove(&cur);
                    if set.is_empty() {
                        self.by_root.remove(&entry.root);
                    }
                }
                if let Some(parent) = entry.parent
                    && let Some(siblings) = self.children.get_mut(&parent)
                {
                    siblings.remove(&cur);
                }
            }
        }
    }

    /// Change mutability from Mut to Imm (the effect of `freeze_ref`).
    pub fn freeze(&mut self, id: RefId) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.mutability = Mutability::Imm;
        }
    }
}

/// Two paths overlap iff one is a prefix of the other.
fn overlaps(a: &[BorrowStep], b: &[BorrowStep]) -> bool {
    let n = a.len().min(b.len());
    a[..n] == b[..n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mut_blocks_other_mut() {
        let mut g = BorrowGraph::new();
        let root = BorrowRoot::Local(0);
        let r1 = g.create(root.clone(), vec![], Mutability::Mut, None);
        assert!(!g.can_borrow(&root, &[], Mutability::Mut));
        assert!(!g.can_borrow(&root, &[], Mutability::Imm));
        g.consume(r1);
        assert!(g.can_borrow(&root, &[], Mutability::Mut));
    }

    #[test]
    fn imm_allows_other_imm_not_mut() {
        let mut g = BorrowGraph::new();
        let root = BorrowRoot::Local(0);
        let _r1 = g.create(root.clone(), vec![], Mutability::Imm, None);
        assert!(g.can_borrow(&root, &[], Mutability::Imm));
        assert!(!g.can_borrow(&root, &[], Mutability::Mut));
    }

    #[test]
    fn field_overlap_blocks_root() {
        let mut g = BorrowGraph::new();
        let root = BorrowRoot::Local(0);
        let _f = g.create(
            root.clone(),
            vec![BorrowStep::Field(2)],
            Mutability::Mut,
            None,
        );
        // Borrowing the root mutably while a field is borrowed mutably is a conflict.
        assert!(!g.can_borrow(&root, &[], Mutability::Mut));
        // Borrowing a *different* field is fine.
        assert!(g.can_borrow(&root, &[BorrowStep::Field(3)], Mutability::Mut));
        // Borrowing the same field is a conflict.
        assert!(!g.can_borrow(&root, &[BorrowStep::Field(2)], Mutability::Mut));
    }

    #[test]
    fn consume_invalidates_descendants() {
        // `force_create` allows the child to overlap its mutable parent. Consuming the
        // parent must remove the child as well.
        let mut g = BorrowGraph::new();
        let root = BorrowRoot::Local(0);
        let parent = g.create(root.clone(), vec![], Mutability::Mut, None);
        let _child = g.force_create(
            root.clone(),
            vec![BorrowStep::Field(0)],
            Mutability::Mut,
            Some(parent),
        );
        assert_eq!(g.outstanding_count(), 2);
        g.consume(parent);
        assert_eq!(g.outstanding_count(), 0);
    }
}
