// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// A set of `u32`s with O(1) insert, membership test, iteration over the
/// members, and clear.
///
/// # Representation
///
/// Briggs and Torczon's sparse set: `dense` lists the members in insertion
/// order, `sparse[i]` is the position of `i` in `dense`. Membership is
/// `sparse[i] < dense.len() && dense[sparse[i]] == i`, which holds only for
/// members, so `sparse` never has to be initialized and clearing is just
/// truncating `dense`.
///
/// The cost is memory: `sparse` is as long as the largest member ever
/// inserted. Use it for small sets drawn from a dense id space.
#[derive(Default)]
pub struct SparseSet {
    sparse: Vec<u32>,
    dense: Vec<u32>,
}

impl SparseSet {
    /// Creates an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `value`, returning whether it was not already a member.
    //
    // TODO(security): `sparse` grows to the largest value ever inserted and is
    // never shrunk, not even by `clear`. Bound the id space feeding this.
    pub fn insert(&mut self, value: u32) -> bool {
        if self.contains(value) {
            return false;
        }
        let idx = value as usize;
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, 0);
        }
        // Members are distinct `u32`s, so `dense` holds at most `u32::MAX + 1`
        // of them and its length before a push is at most `u32::MAX`.
        self.sparse[idx] = self.dense.len() as u32;
        self.dense.push(value);
        true
    }

    /// Whether `value` is a member.
    pub fn contains(&self, value: u32) -> bool {
        match self.sparse.get(value as usize) {
            Some(&pos) => self.dense.get(pos as usize) == Some(&value),
            None => false,
        }
    }

    /// Iterates over the members in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.dense.iter().copied()
    }

    /// Returns the number of members.
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Returns whether the set has no members.
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    /// Removes all members, keeping the allocated capacity.
    pub fn clear(&mut self) {
        self.dense.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_reports_novelty() {
        let mut set = SparseSet::new();
        assert!(set.insert(7));
        assert!(!set.insert(7));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn empty_set_contains_nothing() {
        let set = SparseSet::new();
        assert!(set.is_empty());
        assert!(!set.contains(0));
        assert!(!set.contains(u32::MAX));
    }

    #[test]
    fn spread_out_members() {
        let mut set = SparseSet::new();
        for value in [1, 2, 9999, 100000] {
            assert!(set.insert(value));
        }
        for value in [1, 2, 9999, 100000] {
            assert!(set.contains(value));
        }
        assert!(!set.contains(0));
        assert!(!set.contains(3));
        assert!(!set.contains(99998));
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![1, 2, 9999, 100000]);
    }

    #[test]
    fn clear_keeps_growth() {
        let mut set = SparseSet::new();
        set.insert(100000);
        set.clear();
        assert!(set.is_empty());
        assert!(!set.contains(100000));
        // Reinserting after a clear must still work, and must not report the
        // value as already present.
        assert!(set.insert(100000));
        assert!(set.contains(100000));
    }

    #[test]
    fn stale_sparse_entry_is_not_a_member() {
        let mut set = SparseSet::new();
        set.insert(5);
        set.insert(100000);
        set.clear();
        // `sparse[5]` still reads 0, and position 0 of `dense` is live again.
        // Membership must compare the value back, not just the position.
        set.insert(100000);
        assert!(!set.contains(5));
    }
}
