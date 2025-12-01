use aptos_crypto::HashValue;
use core::num;

pub struct SMT {
    top_levels: Vec<Vec<HashValue>>,
    bottom_ptrs: Vec<Option<Box<Node>>>,
    num_items: usize,
}

impl std::fmt::Debug for SMT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SMT {{\n")?;
        for (i, level) in self.top_levels.iter().enumerate() {
            write!(f, "\tlevel {}: {:x?},\n", i, level)?;
        }
        write!(f, "\tbottom: {:?},\n", self.bottom_ptrs)?;
        write!(f, "\tlen: {},\n", self.num_items)?;
        write!(f, "}}\n")?;
        Ok(())
    }
}

#[derive(Debug)]
enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}

#[derive(Debug)]
struct InternalNode {
    hash: HashValue,
    left: Box<Node>,
    right: Box<Node>,
}

#[derive(Debug)]
struct LeafNode {
    key: HashValue,
    value_hash: HashValue,
}

impl SMT {
    pub fn new(max_num_elements: usize) -> Self {
        assert!(max_num_elements.is_power_of_two());
        assert!(max_num_elements >= 16);

        // For example: 1 million items == 2^20 items.
        // Let's do 18 levels.
        let num_top_levels = max_num_elements.trailing_zeros() as usize - 2;
        let mut top_levels = Vec::new();
        top_levels.resize_with(num_top_levels, Vec::new);
        for i in 0..num_top_levels {
            top_levels[i].resize(1 << i, HashValue::zero());
        }

        let mut bottom_ptrs = Vec::new();
        bottom_ptrs.resize_with(1 << (num_top_levels - 1), || None);

        Self {
            top_levels,
            bottom_ptrs,
            num_items: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.num_items
    }

    pub fn root_hash(&self) -> HashValue {
        unimplemented!()
    }

    pub fn update(&mut self, mut updates: Vec<(HashValue, Option<HashValue>)>) {
        updates.sort_unstable_by_key(|x| x.0);
    }
}

#[cfg(test)]
mod tests {
    use crate::SMT;

    #[test]
    fn test_basic() {
        let smt = SMT::new(32);
        assert_eq!(smt.len(), 0);

        println!("smt: {:?}", smt);
    }
}
