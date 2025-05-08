use sha2::{Sha256, Digest};

pub fn hash_to_fixed_array(input: &Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    
    let mut output: [u8; 32] = [0; 32];
    output.copy_from_slice(&result[..]);
    output
}