// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// The Union-Find data structure, also known as Disjoint Set Union
/// (DSU), is a data structure that maintains a collection of disjoint
/// (non-overlapping) sets and supports two primary operations:
///
///  1) Find: Determines which set a particular element belongs
///     to. This operation returns a representative (or root) of the set
///     containing the element.
///  2) Union: Merges two sets into a single set.

/// - Each element is represented as a node, and sets are represented
///   as trees where each node points to its parent.
/// - The root of the tree serves as the representative of the set.
/// - Two key optimizations ensure efficient operation:
/// - Path compression: During a find operation, nodes along the path
///   to the root are compressed by directly pointing them to the
///   root. This flattens the tree structure, speeding up future
///   operations.
/// - Union by rank/size: During a union operation, the smaller tree
///   (by rank or size) is attached under the root of the larger tree,
///   keeping the trees as flat as possible.
///
/// Amortized Time Complexities:
///
/// - Find: O(\alpha(n)), where \alpha(n) is the inverse Ackermann
///   function. For all practical purposes, \alpha(n) is a very slowly
///   growing function and is effectively constant for typical input
///   sizes.
/// - Union: O(\alpha(n)), due to the optimizations of path
///   compression and union by rank/size.
///
/// Overall - Nearly constant time for both operations.

#[derive(Debug)]
pub struct UnionFind {
    parent: Vec<u32>,
    rank: Vec<u8>,
    num_sets: usize,
    capacity: usize,
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

impl UnionFind {
    // Creates a new [UnionFind] instance with a given number of
    // elements and capacity
    pub fn new_with_size_and_capacity(num_elements: usize, capacity: usize) -> Self {
        if num_elements > std::u32::MAX as usize {
            panic!(
                "The number of elements ({:?}) is larger than std::u32::MAX",
                num_elements
            );
        }
        if capacity > std::u32::MAX as usize {
            panic!("Capacity ({:?}) is larger than std::u32::MAX", capacity);
        }
        if num_elements > capacity {
            panic!(
                "The number of elements ({:?}) is larger than given capacity ({:?})",
                num_elements, capacity
            );
        }

        let parent = (0..num_elements as u32).collect::<Vec<u32>>();
        let rank = vec![0u8; num_elements];
        Self {
            parent,
            rank,
            num_sets: num_elements,
            capacity,
        }
    }

    // Creates a new [UnionFind] instance with given capacity and 0 elements.
    pub fn new_with_capacity(capacity: usize) -> Self {
        Self::new_with_size_and_capacity(0, capacity)
    }

    // Creates a new [UnionFind] instance with given number of elements and maximal capacity (u32::MAX).
    pub fn new_with_size(num_elements: usize) -> Self {
        Self::new_with_size_and_capacity(num_elements, std::u32::MAX as usize)
    }

    // Creates a new [UnionFind] instance with 0 elements and maximal capacity.
    pub fn new() -> Self {
        Self::new_with_size_and_capacity(0, std::u32::MAX as usize)
    }

    // Finds the representative of the set containing a given id without bound checking
    fn find_set_internal(&mut self, id: u32) -> u32 {
        let mut i = id;

        let mut root = id;
        while root != self.parent[root as usize] {
            root = self.parent[root as usize];
        }

        while i != root {
            i = std::mem::replace(&mut self.parent[i as usize], root);
        }

        root
    }

    // Finds the representative of the set containing a given id
    pub fn find_set(&mut self, id: u32) -> Result<u32, &'static str> {
        if (id as usize) >= self.size() {
            return Err("Element out of bounds");
        }

        Ok(self.find_set_internal(id))
    }

    // Joins sets represented by `x` and `y` (both have to be set representative)
    pub fn join_sets_internal(&mut self, x: u32, y: u32) -> u32 {
        if x == y {
            return x;
        }

        // Merging two different sets
        self.num_sets -= 1;

        if self.rank[x as usize] >= self.rank[y as usize] {
            self.parent[y as usize] = x;
            if self.rank[x as usize] == self.rank[y as usize] {
                self.rank[x as usize] += 1;
            }
            return x;
        }

        self.parent[x as usize] = y;
        y
    }

    // Joins the sets containing `x` and `y`.
    pub fn join(&mut self, mut x: u32, mut y: u32) -> Result<u32, &'static str> {
        if ((x as usize) >= self.size()) || ((y as usize) >= self.size()) {
            return Err("Element out of bounds");
        }

        x = self.find_set(x)?;
        y = self.find_set(y)?;

        Ok(self.join_sets_internal(x, y))
    }

    // Adds a new element to the [UnionFind] structure.
    pub fn add_one(&mut self) -> Result<u32, &'static str> {
        if self.size() >= self.capacity() {
            return Err("Reached maximal capacity");
        }

        self.num_sets += 1;
        let ret = self.parent.len() as u32;
        self.parent.push(ret);
        self.rank.push(0u8);
        Ok(ret)
    }

    // Returns the total number of elements.
    pub fn size(&self) -> usize {
        self.parent.len()
    }

    // Returns the number of disjoint sets.
    pub fn set_count(&self) -> usize {
        self.num_sets
    }

    // Returns the capacity of disjoint sets.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
#[allow(dead_code)] // TODO (add prop tests)
mod tests {

    use super::*;

    struct NaiveUnionFind {
        representative: Vec<u32>,
        num_sets: usize,
        capacity: usize,
    }

    impl NaiveUnionFind {
        /// Creates a new UnionFind instance with `num_elements` and `capacity`
        pub fn new_with_size_and_capacity(
            num_elements: usize,
            capacity: usize,
        ) -> Result<Self, &'static str> {
            if num_elements > std::u32::MAX as usize {
                return Err("num_elements larger than std::u32::MAX");
            }
            if capacity > std::u32::MAX as usize {
                return Err("capacity larger than std::u32::MAX");
            }
            if num_elements > capacity {
                return Err("num_elements larger than given capacity");
            }

            let representative = (0..num_elements as u32).collect::<Vec<u32>>();
            Ok(Self {
                representative,
                num_sets: num_elements,
                capacity,
            })
        }

        /// Creates a new UnionFind instance with given `capacity` and 0 elements
        pub fn new_with_capacity(capacity: usize) -> Result<Self, &'static str> {
            Self::new_with_size_and_capacity(0, capacity)
        }

        /// Creates a new UnionFind instance with `num_elements` and maximal capacity.
        pub fn new_with_size(num_elements: usize) -> Result<Self, &'static str> {
            Self::new_with_size_and_capacity(num_elements, std::u32::MAX as usize)
        }

        /// Creates a new UnionFind instance with `0` elements and maximal capacity.
        pub fn new() -> Result<Self, &'static str> {
            Self::new_with_size_and_capacity(0, std::u32::MAX as usize)
        }

        /// Finds the representative of the set containing `id`
        pub fn find_set(&mut self, id: u32) -> Result<u32, &'static str> {
            if (id as usize) >= self.size() {
                return Err("Element out of bounds");
            }
            Ok(self.representative[id as usize])
        }

        /// Unites the sets containing `x` and `y`.
        pub fn join(&mut self, mut x: u32, mut y: u32) -> Result<u32, &'static str> {
            if ((x as usize) >= self.size()) || ((y as usize) >= self.size()) {
                return Err("Element out of bounds");
            }

            x = self.find_set(x)?;
            y = self.find_set(y)?;

            for e in &mut self.representative {
                if *e == x {
                    *e = y;
                }
            }

            Ok(y)
        }

        /// Adds a new element to the UnionFind structure.
        pub fn add_one(&mut self) -> Result<u32, &'static str> {
            if self.size() >= self.capacity() {
                return Err("Reached maximal capacity");
            }

            self.num_sets += 1;
            let ret = self.representative.len() as u32;
            self.representative.push(ret);
            Ok(ret)
        }

        /// Returns the total number of elements.
        pub fn size(&self) -> usize {
            self.representative.len()
        }

        /// Returns the number of disjoint sets.
        pub fn set_count(&self) -> usize {
            self.num_sets
        }

        /// Returns the capacity of disjoint sets.
        pub fn capacity(&self) -> usize {
            self.capacity
        }
    }

    #[test]
    fn basic_tests() -> Result<(), &'static str> {
        let mut sets = UnionFind::new_with_size_and_capacity(1, 16);
        assert_eq!(sets.find_set(0)?, 0);
        for i in 1..16 {
            assert_eq!(sets.add_one()?, i);
            assert_eq!(sets.find_set(i)?, i);
            assert_eq!(sets.set_count(), (i + 1) as usize);
        }
        assert!(sets.add_one().is_err());
        assert!(sets.find_set(16).is_err());

        for i in 0..8 {
            assert_ne!(sets.find_set(i * 2)?, sets.find_set(i * 2 + 1)?);
            assert!(sets.join(i * 2, i * 2 + 1).is_ok());
            assert_eq!(sets.find_set(i * 2)?, sets.find_set(i * 2 + 1)?);
            assert!(sets.join(i * 2, i * 2 + 1).is_ok());
            assert_eq!(sets.set_count(), 15 - i as usize);
        }

        for i in 0..4 {
            assert_ne!(sets.find_set(i * 4)?, sets.find_set(i * 4 + 2)?);
            assert!(sets.join(i * 4, i * 4 + 2).is_ok());
            assert_eq!(sets.find_set(i * 4)?, sets.find_set(i * 4 + 2)?);
            assert!(sets.join(i * 4, i * 4 + 2).is_ok());
            assert_eq!(sets.set_count(), 7 - i as usize);
        }

        for i in 0..2 {
            assert_ne!(sets.find_set(i * 8)?, sets.find_set(i * 8 + 4)?);
            assert!(sets.join(i * 8, i * 8 + 4).is_ok());
            assert_eq!(sets.find_set(i * 8)?, sets.find_set(i * 8 + 4)?);
            assert!(sets.join(i * 8, i * 8 + 4).is_ok());
            assert_eq!(sets.set_count(), 3 - i as usize);
        }

        for i in 0..7 {
            assert_eq!(sets.find_set(i)?, sets.find_set(i + 1)?);
            assert_ne!(sets.find_set(i)?, sets.find_set(8 + i)?);
            assert_eq!(sets.find_set(8 + i)?, sets.find_set(8 + i + 1)?);
        }

        assert!(sets.join(0, 16).is_err());
        assert!(sets.join(16, 0).is_err());
        assert!(sets.join(0, 15).is_ok());

        Ok(())
    }
}
