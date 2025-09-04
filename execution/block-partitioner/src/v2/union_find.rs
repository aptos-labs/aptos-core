// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

/// A union-find implementation with [path compression](https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Finding_set_representatives)
/// and [union by rank](https://en.wikipedia.org/wiki/Disjoint-set_data_structure#Union_by_rank),
/// where elements are organized as a forest, each tree representing a set.
///
/// The amortized time complexity for both `union()` and `find()` is `O(a(n))`,
/// where:
/// `a()` is the extremely slow-growing inverse Ackermann function.
/// `n` is the total number of elements.
pub struct UnionFind {
    /// Tracks the parent of each element in the forest.
    /// Initially pointing to self and can be updated during `union()` (and also `find()`, due to path compression).
    parent_of: Vec<usize>,
    /// Tracks the height of each sub-tree.
    /// This state is required by "union by rank" to guarantee each tree heights are less than `log2(n)`.
    height_of: Vec<usize>,
}

impl UnionFind {
    pub fn new(num_participants: usize) -> Self {
        Self {
            parent_of: (0..num_participants).collect(),
            height_of: vec![0; num_participants],
        }
    }

    pub fn find(&mut self, a: usize) -> usize {
        let mut root = self.parent_of[a];
        while self.parent_of[root] != root {
            root = self.parent_of[root];
        }

        let mut element = a;
        while element != root {
            let next_element = self.parent_of[element];
            self.parent_of[element] = root;
            element = next_element;
        }
        root
    }

    pub fn union(&mut self, x: usize, y: usize) {
        let px = self.find(x);
        let py = self.find(y);
        if px == py {
            return;
        }

        match self.height_of[px].cmp(&self.height_of[py]) {
            Ordering::Less => {
                self.parent_of[py] = px;
            },
            Ordering::Greater => {
                self.parent_of[px] = py;
            },
            Ordering::Equal => {
                self.parent_of[px] = py;
                self.height_of[py] += 1;
            },
        }
    }
}

#[test]
fn test_union_find() {
    let mut uf = UnionFind::new(5);
    uf.union(0, 3);
    assert_eq!(uf.find(0), uf.find(3));
    assert_ne!(uf.find(1), uf.find(4));
    uf.union(1, 4);
    assert_ne!(uf.find(0), uf.find(4));
    uf.union(3, 1);
    assert_eq!(uf.find(0), uf.find(4));
    assert_ne!(uf.find(2), uf.find(4));
    assert_ne!(uf.find(2), uf.find(3));
    assert_ne!(uf.find(2), uf.find(1));
    assert_ne!(uf.find(2), uf.find(0));
}
