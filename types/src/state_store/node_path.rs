use bitvec::{order::Msb0, vec::BitVec};

pub type ChildIndex = bool;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd /*, Serialize, Deserialize*/)]
pub struct NodePath {
    bits: BitVec<u8, Msb0>,
}

impl NodePath {
    pub fn new(bits: BitVec<u8, Msb0>) -> Self {
        Self { bits }
    }

    pub fn new_from_vec(num_bits: usize, bytes: Vec<u8>) -> Self {
        let mut bits = BitVec::try_from_vec(bytes).unwrap();
        assert!(
            bits.len() / 8 == (num_bits + 7) / 8,
            "Invalid bits: {:?}",
            bits
        );
        bits.truncate(num_bits);
        Self { bits }
    }

    pub fn push(&mut self, child_index: ChildIndex) {
        self.bits.push(child_index)
    }

    pub fn pop(&mut self) -> Option<bool> {
        self.bits.pop()
    }

    pub fn num_bits(&self) -> usize {
        self.bits.len()
    }

    pub fn bytes(&self) -> &[u8] {
        self.bits.as_raw_slice()
    }

    pub fn bit(&self, n: usize) -> Option<bool> {
        match self.bits.get(n) {
            Some(bit) => Some(bit.as_ref().clone()),
            None => None,
        }
    }
}
