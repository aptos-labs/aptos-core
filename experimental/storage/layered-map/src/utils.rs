// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[inline]
pub(crate) fn binary_tree_height(num_leaves: usize) -> usize {
    if num_leaves == 0 {
        0
    } else {
        num_leaves.next_power_of_two().trailing_zeros() as usize + 1
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::binary_tree_height;
    #[test]
    fn test_height() {
        assert_eq!(binary_tree_height(0), 0);
        assert_eq!(binary_tree_height(1), 1);
        assert_eq!(binary_tree_height(2), 2);
        assert_eq!(binary_tree_height(3), 3);
        assert_eq!(binary_tree_height(4), 3);
        assert_eq!(binary_tree_height(5), 4);
        assert_eq!(binary_tree_height(6), 4);
        assert_eq!(binary_tree_height(7), 4);
        assert_eq!(binary_tree_height(8), 4);
        assert_eq!(binary_tree_height(9), 5);
        assert_eq!(binary_tree_height(16), 5);
        assert_eq!(binary_tree_height(17), 6);
    }
}
