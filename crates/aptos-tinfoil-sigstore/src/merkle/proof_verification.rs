// Adapted from sigstore-rs (Apache 2.0 License)
// https://github.com/sigstore/sigstore-rs/blob/34f232af72ba6108f001f1612fdb03c87af8ca62/src/crypto/merkle/proof_verification.rs
//
// Copyright 2023 The Sigstore Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0

use super::rfc6962::Rfc6269HasherTrait;
use MerkleProofError::*;
use digest::{Digest, Output};
use std::fmt::Debug;

#[derive(Debug)]
pub enum MerkleProofError {
    MismatchedRoot { expected: String, got: String },
    IndexGtTreeSize,
    UnexpectedNonEmptyProof,
    UnexpectedEmptyProof,
    NewTreeSmaller { new: u64, old: u64 },
    WrongProofSize { got: u64, want: u64 },
    WrongEmptyTreeHash,
}

impl std::fmt::Display for MerkleProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MismatchedRoot { expected, got } => {
                write!(f, "root hash mismatch: expected {expected}, got {got}")
            }
            IndexGtTreeSize => write!(f, "leaf index >= tree size"),
            UnexpectedNonEmptyProof => write!(f, "unexpected non-empty proof"),
            UnexpectedEmptyProof => write!(f, "unexpected empty proof"),
            NewTreeSmaller { new, old } => {
                write!(f, "new tree size {new} < old tree size {old}")
            }
            WrongProofSize { got, want } => {
                write!(f, "wrong proof size: got {got}, want {want}")
            }
            WrongEmptyTreeHash => write!(f, "wrong empty tree hash"),
        }
    }
}

pub trait MerkleProofVerifier<O>: Rfc6269HasherTrait<O>
where
    O: Eq + AsRef<[u8]> + Clone + Debug,
{
    /// Used to verify hashes.
    fn verify_match(a: &O, b: &O) -> Result<(), ()> {
        (a == b).then_some(()).ok_or(())
    }

    /// `verify_inclusion` verifies the correctness of the inclusion proof for the leaf
    /// with the specified `leaf_hash` and `index`, relatively to the tree of the given `tree_size`
    /// and `root_hash`. Requires `0 <= index < tree_size`.
    fn verify_inclusion(
        index: u64,
        leaf_hash: &O,
        tree_size: u64,
        proof_hashes: &[O],
        root_hash: &O,
    ) -> Result<(), MerkleProofError> {
        if index >= tree_size {
            return Err(IndexGtTreeSize);
        }
        Self::root_from_inclusion_proof(index, leaf_hash, tree_size, proof_hashes).and_then(
            |calc_root| {
                Self::verify_match(calc_root.as_ref(), root_hash).map_err(|_| MismatchedRoot {
                    got: hex::encode(root_hash.as_ref()),
                    expected: hex::encode(calc_root.as_ref()),
                })
            },
        )
    }

    /// `root_from_inclusion_proof` calculates the expected root hash for a tree of the
    /// given size, provided a leaf index and hash with the corresponding inclusion
    /// proof. Requires `0 <= index < tree_size`.
    fn root_from_inclusion_proof(
        index: u64,
        leaf_hash: &O,
        tree_size: u64,
        proof_hashes: &[O],
    ) -> Result<Box<O>, MerkleProofError> {
        if index >= tree_size {
            return Err(IndexGtTreeSize);
        }
        let (inner, border) = Self::decomp_inclusion_proof(index, tree_size);
        match (proof_hashes.len() as u64, inner + border) {
            (got, want) if got != want => {
                return Err(WrongProofSize {
                    got: proof_hashes.len() as u64,
                    want: inner + border,
                });
            }
            _ => {}
        }
        let res_left = Self::chain_inner(leaf_hash, &proof_hashes[..inner as usize], index);
        let res = Self::chain_border_right(&res_left, &proof_hashes[inner as usize..]);
        Ok(Box::new(res))
    }

    /// `verify_consistency` checks that the passed-in consistency proof is valid
    /// between the passed in tree sizes, with respect to the corresponding root
    /// hashes. Requires `0 <= old_size <= new_size`.
    fn verify_consistency(
        old_size: u64,
        new_size: u64,
        proof_hashes: &[O],
        old_root: &O,
        new_root: &O,
    ) -> Result<(), MerkleProofError> {
        use std::cmp::Ordering;

        match (
            Ord::cmp(&old_size, &new_size),
            old_size == 0,
            proof_hashes.is_empty(),
        ) {
            (Ordering::Greater, _, _) => {
                return Err(NewTreeSmaller {
                    new: new_size,
                    old: old_size,
                });
            }
            (Ordering::Equal, _, true) => {
                return Self::verify_match(old_root, new_root).map_err(|_| MismatchedRoot {
                    got: hex::encode(new_root.as_ref()),
                    expected: hex::encode(old_root.as_ref()),
                });
            }
            (Ordering::Equal, _, false) | (Ordering::Less, true, false) => {
                return Err(UnexpectedNonEmptyProof);
            }
            (Ordering::Less, true, true) => {
                return Self::verify_match(old_root, &Self::empty_root())
                    .map_err(|_| WrongEmptyTreeHash);
            }
            (Ordering::Less, false, true) => return Err(UnexpectedEmptyProof),
            (Ordering::Less, false, false) => {}
        }

        let shift = old_size.trailing_zeros() as u64;
        let (inner, border) = Self::decomp_inclusion_proof(old_size - 1, new_size);
        let inner = inner - shift;

        let (seed, start) = if old_size == 1 << shift {
            (old_root, 0)
        } else {
            (&proof_hashes[0], 1)
        };

        match (proof_hashes.len() as u64, start + inner + border) {
            (got, want) if got != want => return Err(WrongProofSize { got, want }),
            _ => {}
        }

        let proof = &proof_hashes[start as usize..];
        let mask = (old_size - 1) >> shift;

        let hash1 = Self::chain_inner_right(seed, &proof[..inner as usize], mask);
        let hash1 = Self::chain_border_right(&hash1, &proof[inner as usize..]);
        Self::verify_match(&hash1, old_root).map_err(|_| MismatchedRoot {
            got: hex::encode(old_root.as_ref()),
            expected: hex::encode(hash1.as_ref()),
        })?;

        let hash2 = Self::chain_inner(seed, &proof[..inner as usize], mask);
        let hash2 = Self::chain_border_right(&hash2, &proof[inner as usize..]);
        Self::verify_match(&hash2, new_root).map_err(|_| MismatchedRoot {
            got: hex::encode(new_root.as_ref()),
            expected: hex::encode(hash2.as_ref()),
        })?;

        Ok(())
    }

    /// `chain_inner` computes a subtree hash for a node on or below the tree's right
    /// border. Assumes `proof_hashes` are ordered from lower levels to upper, and
    /// `seed` is the initial subtree/leaf hash on the path located at the specified
    /// `index` on its level.
    fn chain_inner(seed: &O, proof_hashes: &[O], index: u64) -> O {
        proof_hashes
            .iter()
            .enumerate()
            .fold(seed.clone(), |seed, (i, h)| {
                let (left, right) = if ((index >> i) & 1) == 0 {
                    (&seed, h)
                } else {
                    (h, &seed)
                };
                Self::hash_children(left, right)
            })
    }

    /// `chain_inner_right` computes a subtree hash like `chain_inner`, but only takes
    /// hashes to the left from the path into consideration, which effectively means
    /// the result is a hash of the corresponding earlier version of this subtree.
    fn chain_inner_right(seed: &O, proof_hashes: &[O], index: u64) -> O {
        proof_hashes
            .iter()
            .enumerate()
            .fold(seed.clone(), |seed, (i, h)| {
                if ((index >> i) & 1) == 1 {
                    Self::hash_children(h, seed)
                } else {
                    seed
                }
            })
    }

    /// `chain_border_right` chains proof hashes along tree borders. This differs from
    /// inner chaining because `proof` contains only left-side subtree hashes.
    fn chain_border_right(seed: &O, proof_hashes: &[O]) -> O {
        proof_hashes
            .iter()
            .fold(seed.clone(), |seed, h| Self::hash_children(h, seed))
    }

    /// `decomp_inclusion_proof` breaks down inclusion proof for a leaf at the specified
    /// `index` in a tree of the specified `size` into 2 components. The splitting
    /// point between them is where paths to leaves `index` and `tree_size-1` diverge.
    /// Returns lengths of the bottom and upper proof parts correspondingly. The sum
    /// of the two determines the correct length of the inclusion proof.
    fn decomp_inclusion_proof(index: u64, tree_size: u64) -> (u64, u64) {
        let inner: u64 = Self::inner_proof_size(index, tree_size);
        let border = (index >> inner).count_ones() as u64;
        (inner, border)
    }

    /// `inner_proof_size` computes the number of inner levels (hashes) required in the audit path
    /// given a leaf at index in a tree of tree_size.
    fn inner_proof_size(index: u64, tree_size: u64) -> u64 {
        u64::BITS as u64 - ((index ^ (tree_size - 1)).leading_zeros() as u64)
    }
}

impl<T> MerkleProofVerifier<Output<T>> for T where T: Digest {}

#[cfg(test)]
mod test_verify {
    use super::*;
    use crate::merkle::Rfc6269Default;
    use crate::merkle::rfc6962::Rfc6269HasherTrait;
    use hex_literal::hex;

    #[derive(Debug)]
    struct InclusionProofTestVector<'a> {
        leaf: u64,
        size: u64,
        proof: &'a [[u8; 32]],
    }

    #[derive(Debug)]
    struct InclusionProbe {
        leaf_index: u64,
        tree_size: u64,
        root: [u8; 32],
        leaf_hash: [u8; 32],
        proof: Vec<[u8; 32]>,
        desc: &'static str,
    }

    const SHA256_SOME_HASH: [u8; 32] =
        hex!("abacaba000000000000000000000000000000000000000000060061e00123456");

    const SHA256_EMPTY_TREE_HASH: [u8; 32] =
        hex!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

    const ZERO_HASH: [u8; 32] = [0; 32];

    const INCLUSION_PROOFS: [InclusionProofTestVector; 6] = [
        InclusionProofTestVector {
            leaf: 0,
            size: 0,
            proof: &[],
        },
        InclusionProofTestVector {
            leaf: 1,
            size: 1,
            proof: &[],
        },
        InclusionProofTestVector {
            leaf: 1,
            size: 8,
            proof: &[
                hex!("96a296d224f285c67bee93c30f8a309157f0daa35dc5b87e410b78630a09cfc7"),
                hex!("5f083f0a1a33ca076a95279832580db3e0ef4584bdff1f54c8a360f50de3031e"),
                hex!("6b47aaf29ee3c2af9af889bc1fb9254dabd31177f16232dd6aab035ca39bf6e4"),
            ],
        },
        InclusionProofTestVector {
            leaf: 6,
            size: 8,
            proof: &[
                hex!("bc1a0643b12e4d2d7c77918f44e0f4f79a838b6cf9ec5b5c283e1f4d88599e6b"),
                hex!("ca854ea128ed050b41b35ffc1b87b8eb2bde461e9e3b5596ece6b9d5975a0ae0"),
                hex!("d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7"),
            ],
        },
        InclusionProofTestVector {
            leaf: 3,
            size: 3,
            proof: &[hex!(
                "fac54203e7cc696cf0dfcb42c92a1d9dbaf70ad9e621f4bd8d98662f00e3c125"
            )],
        },
        InclusionProofTestVector {
            leaf: 2,
            size: 5,
            proof: &[
                hex!("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"),
                hex!("5f083f0a1a33ca076a95279832580db3e0ef4584bdff1f54c8a360f50de3031e"),
                hex!("bc1a0643b12e4d2d7c77918f44e0f4f79a838b6cf9ec5b5c283e1f4d88599e6b"),
            ],
        },
    ];

    const ROOTS: [[u8; 32]; 8] = [
        hex!("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"),
        hex!("fac54203e7cc696cf0dfcb42c92a1d9dbaf70ad9e621f4bd8d98662f00e3c125"),
        hex!("aeb6bcfe274b70a14fb067a5e5578264db0fa9b51af5e0ba159158f329e06e77"),
        hex!("d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7"),
        hex!("4e3bbb1f7b478dcfe71fb631631519a3bca12c9aefca1612bfce4c13a86264d4"),
        hex!("76e67dadbcdf1e10e1b74ddc608abd2f98dfb16fbce75277b5232a127f2087ef"),
        hex!("ddb89be403809e325750d3d263cd78929c2942b7942a34b77e122c9594a74c8c"),
        hex!("5dc9da79a70659a9ad559cb701ded9a2ab9d823aad2f4960cfe370eff4604328"),
    ];

    const LEAVES: &[&[u8]] = &[
        &hex!(""),
        &hex!("00"),
        &hex!("10"),
        &hex!("2021"),
        &hex!("3031"),
        &hex!("40414243"),
        &hex!("5051525354555657"),
        &hex!("606162636465666768696a6b6c6d6e6f"),
    ];

    fn corrupt_inclusion_proof(
        leaf_index: u64,
        tree_size: u64,
        proof: &[[u8; 32]],
        root: &[u8; 32],
        leaf_hash: &[u8; 32],
    ) -> Vec<InclusionProbe> {
        vec![
            InclusionProbe {
                leaf_index: leaf_index.wrapping_sub(1),
                tree_size,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: proof.to_vec(),
                desc: "leaf_index - 1",
            },
            InclusionProbe {
                leaf_index: leaf_index + 1,
                tree_size,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: proof.to_vec(),
                desc: "leaf_index + 1",
            },
            InclusionProbe {
                leaf_index: leaf_index ^ 2,
                tree_size,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: proof.to_vec(),
                desc: "leaf_index ^ 2",
            },
            InclusionProbe {
                leaf_index,
                tree_size: tree_size / 2,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: proof.to_vec(),
                desc: "tree_size / 2",
            },
            InclusionProbe {
                leaf_index,
                tree_size: tree_size * 2,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: proof.to_vec(),
                desc: "tree_size * 2",
            },
            InclusionProbe {
                leaf_index,
                tree_size,
                root: *root,
                leaf_hash: *b"WrongLeafWrongLeafWrongLeafWrong",
                proof: proof.to_vec(),
                desc: "wrong leaf",
            },
            InclusionProbe {
                leaf_index,
                tree_size,
                root: SHA256_EMPTY_TREE_HASH,
                leaf_hash: *leaf_hash,
                proof: proof.to_vec(),
                desc: "empty root",
            },
            InclusionProbe {
                leaf_index,
                tree_size,
                root: SHA256_SOME_HASH,
                leaf_hash: *leaf_hash,
                proof: proof.to_vec(),
                desc: "random root",
            },
            InclusionProbe {
                leaf_index,
                tree_size,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: [proof.to_vec(), [[0_u8; 32]].to_vec()].concat(),
                desc: "trailing garbage",
            },
            InclusionProbe {
                leaf_index,
                tree_size,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: [proof.to_vec(), [*root].to_vec()].concat(),
                desc: "trailing root",
            },
            InclusionProbe {
                leaf_index,
                tree_size,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: [[[0_u8; 32]].to_vec(), proof.to_vec()].concat(),
                desc: "preceding garbage",
            },
            InclusionProbe {
                leaf_index,
                tree_size,
                root: *root,
                leaf_hash: *leaf_hash,
                proof: [[*root].to_vec(), proof.to_vec()].concat(),
                desc: "preceding root",
            },
        ]
    }

    fn verifier_check(
        leaf_index: u64,
        tree_size: u64,
        proof_hashes: &[[u8; 32]],
        root: &[u8; 32],
        leaf_hash: &[u8; 32],
    ) -> Result<(), String> {
        let probes = corrupt_inclusion_proof(leaf_index, tree_size, proof_hashes, root, leaf_hash);
        let leaf_hash = leaf_hash.into();
        let root_hash = root.into();
        let proof_hashes = proof_hashes.iter().map(|&h| h.into()).collect::<Vec<_>>();
        let got = Rfc6269Default::root_from_inclusion_proof(
            leaf_index,
            leaf_hash,
            tree_size,
            &proof_hashes,
        )
        .map_err(|err| format!("{err:?}"))?;
        Rfc6269Default::verify_match(got.as_ref(), root_hash)
            .map_err(|_| format!("roots did not match got: {got:x?} expected: {root:x?}"))?;
        Rfc6269Default::verify_inclusion(
            leaf_index,
            leaf_hash,
            tree_size,
            &proof_hashes,
            root_hash,
        )
        .map_err(|err| format!("{err:?}"))?;

        probes
            .into_iter()
            .map(|p| {
                Rfc6269Default::verify_inclusion(
                    p.leaf_index,
                    (&p.leaf_hash).into(),
                    p.tree_size,
                    &p.proof.iter().map(|&h| h.into()).collect::<Vec<_>>(),
                    (&p.root).into(),
                )
                .err()
                .ok_or(format!("accepted incorrect inclusion proof: {:?}", p.desc))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    #[derive(Debug)]
    struct ConsistencyTestVector<'a> {
        size1: u64,
        size2: u64,
        proof: &'a [[u8; 32]],
    }

    #[derive(Debug)]
    struct ConsistencyProbe<'a> {
        size1: u64,
        size2: u64,
        root1: &'a [u8; 32],
        root2: &'a [u8; 32],
        proof: Vec<[u8; 32]>,
        desc: &'static str,
    }

    const CONSISTENCY_PROOFS: [ConsistencyTestVector; 5] = [
        ConsistencyTestVector {
            size1: 1,
            size2: 1,
            proof: &[],
        },
        ConsistencyTestVector {
            size1: 1,
            size2: 8,
            proof: &[
                hex!("96a296d224f285c67bee93c30f8a309157f0daa35dc5b87e410b78630a09cfc7"),
                hex!("5f083f0a1a33ca076a95279832580db3e0ef4584bdff1f54c8a360f50de3031e"),
                hex!("6b47aaf29ee3c2af9af889bc1fb9254dabd31177f16232dd6aab035ca39bf6e4"),
            ],
        },
        ConsistencyTestVector {
            size1: 6,
            size2: 8,
            proof: &[
                hex!("0ebc5d3437fbe2db158b9f126a1d118e308181031d0a949f8dededebc558ef6a"),
                hex!("ca854ea128ed050b41b35ffc1b87b8eb2bde461e9e3b5596ece6b9d5975a0ae0"),
                hex!("d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7"),
            ],
        },
        ConsistencyTestVector {
            size1: 2,
            size2: 5,
            proof: &[
                hex!("5f083f0a1a33ca076a95279832580db3e0ef4584bdff1f54c8a360f50de3031e"),
                hex!("bc1a0643b12e4d2d7c77918f44e0f4f79a838b6cf9ec5b5c283e1f4d88599e6b"),
            ],
        },
        ConsistencyTestVector {
            size1: 6,
            size2: 7,
            proof: &[
                hex!("0ebc5d3437fbe2db158b9f126a1d118e308181031d0a949f8dededebc558ef6a"),
                hex!("b08693ec2e721597130641e8211e7eedccb4c26413963eee6c1e2ed16ffb1a5f"),
                hex!("d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7"),
            ],
        },
    ];

    fn corrupt_consistency_proof<'a>(
        size1: u64,
        size2: u64,
        root1: &'a [u8; 32],
        root2: &'a [u8; 32],
        proof: &[[u8; 32]],
    ) -> Vec<ConsistencyProbe<'a>> {
        let ln = proof.len();
        let mut ret = vec![
            ConsistencyProbe {
                size1: size1 - 1,
                size2,
                root1,
                root2,
                proof: proof.to_vec(),
                desc: "size1 - 1",
            },
            ConsistencyProbe {
                size1: size1 + 1,
                size2,
                root1,
                root2,
                proof: proof.to_vec(),
                desc: "size1 + 1",
            },
            ConsistencyProbe {
                size1: size1 ^ 2,
                size2,
                root1,
                root2,
                proof: proof.to_vec(),
                desc: "size1 ^ 2",
            },
            ConsistencyProbe {
                size1,
                size2: size2 * 2,
                root1,
                root2,
                proof: proof.to_vec(),
                desc: "size2 * 2",
            },
            ConsistencyProbe {
                size1,
                size2: size2 / 2,
                root1,
                root2,
                proof: proof.to_vec(),
                desc: "size2 / 2",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1: &ZERO_HASH,
                root2,
                proof: proof.to_vec(),
                desc: "wrong root1",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2: &ZERO_HASH,
                proof: proof.to_vec(),
                desc: "wrong root2",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1: root2,
                root2: root1,
                proof: proof.to_vec(),
                desc: "swapped roots",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: vec![],
                desc: "empty proof",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: [proof, &[ZERO_HASH]].concat(),
                desc: "trailing garbage",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: [proof, &[*root1]].concat(),
                desc: "trailing root1",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: [proof, &[*root2]].concat(),
                desc: "trailing root2",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: [&[ZERO_HASH], proof].concat(),
                desc: "preceding garbage",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: [&[*root1], proof].concat(),
                desc: "preceding root1",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: [&[*root2], proof].concat(),
                desc: "preceding root2",
            },
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: [&[proof[0]], proof].concat(),
                desc: "preceding proof[0]",
            },
        ];
        if ln > 0 {
            ret.push(ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: proof[..ln - 1].to_vec(),
                desc: "truncated proof",
            });
        }
        ret.extend((0..ln).map(|i| {
            let mut wrong_proof = proof.to_vec();
            wrong_proof[i][i] ^= 4;
            ConsistencyProbe {
                size1,
                size2,
                root1,
                root2,
                proof: wrong_proof,
                desc: "proof with flipped bit",
            }
        }));
        ret
    }

    fn verifier_consistency_check(
        size1: u64,
        size2: u64,
        proof: &[[u8; 32]],
        root1: &[u8; 32],
        root2: &[u8; 32],
    ) -> Result<(), String> {
        let proof_hashes = proof.iter().map(|&h| h.into()).collect::<Vec<_>>();
        Rfc6269Default::verify_consistency(size1, size2, &proof_hashes, root1.into(), root2.into())
            .map_err(|err| format!("incorrectly rejected with {err:?}"))?;
        if proof.is_empty() {
            return Ok(());
        }
        for (i, p) in corrupt_consistency_proof(size1, size2, root1, root2, proof)
            .iter()
            .enumerate()
        {
            Rfc6269Default::verify_consistency(
                p.size1,
                p.size2,
                &p.proof.iter().map(|&h| h.into()).collect::<Vec<_>>(),
                <&Output<Rfc6269Default>>::try_from(p.root1.as_slice()).unwrap(),
                <&Output<Rfc6269Default>>::try_from(p.root2.as_slice()).unwrap(),
            )
            .err()
            .ok_or(format!("[{i}] incorrectly accepted: {:?}", p.desc))?;
        }
        Ok(())
    }

    #[test]
    fn test_verify_inclusion_single_entry() {
        let data = b"data";
        let hash = &Rfc6269Default::hash_leaf(data);
        let proof = [];
        let zero_hash = <&Output<Rfc6269Default>>::try_from(ZERO_HASH.as_slice()).unwrap();
        let test_cases = [
            (hash, hash, false),
            (hash, zero_hash, true),
            (zero_hash, hash, true),
        ];
        for (i, (root, leaf, want_err)) in test_cases.into_iter().enumerate() {
            let res = Rfc6269Default::verify_inclusion(0, leaf, 1, &proof, root);
            assert_eq!(
                res.is_err(),
                want_err,
                "unexpected inclusion proof result {res:?} for case {i:?}"
            )
        }
    }

    #[test]
    fn test_verify_inclusion() {
        let proof = [];
        let probes = [(0, 0), (0, 1), (1, 0), (2, 1)];
        probes.into_iter().for_each(|(index, size)| {
            let result = Rfc6269Default::verify_inclusion(
                index,
                <&Output<Rfc6269Default>>::try_from(SHA256_SOME_HASH.as_slice()).unwrap(),
                size,
                &proof,
                <&Output<Rfc6269Default>>::try_from(ZERO_HASH.as_slice()).unwrap(),
            );
            assert!(result.is_err(), "Incorrectly verified invalid root/leaf",);
            let result = Rfc6269Default::verify_inclusion(
                index,
                <&Output<Rfc6269Default>>::try_from(ZERO_HASH.as_slice()).unwrap(),
                size,
                &proof,
                <&Output<Rfc6269Default>>::try_from(SHA256_EMPTY_TREE_HASH.as_slice()).unwrap(),
            );
            assert!(result.is_err(), "Incorrectly verified invalid root/leaf",);
            let result = Rfc6269Default::verify_inclusion(
                index,
                <&Output<Rfc6269Default>>::try_from(SHA256_SOME_HASH.as_slice()).unwrap(),
                size,
                &proof,
                <&Output<Rfc6269Default>>::try_from(SHA256_EMPTY_TREE_HASH.as_slice()).unwrap(),
            );
            assert!(result.is_err(), "Incorrectly verified invalid root/leaf");
        });
        for i in 1..6 {
            let p = &INCLUSION_PROOFS[i];
            let leaf_hash = &Rfc6269Default::hash_leaf(LEAVES[i]).into();
            let result = verifier_check(
                p.leaf - 1,
                p.size,
                p.proof,
                &ROOTS[p.size as usize - 1],
                leaf_hash,
            );
            assert!(result.is_err(), "{result:?}")
        }
    }

    #[test]
    fn test_verify_consistency() {
        let root1 = &[0; 32].into();
        let root2 = &[1; 32].into();
        let proof1 = [].as_slice();
        let proof2 = [SHA256_EMPTY_TREE_HASH.into()];
        let empty_tree_hash = &SHA256_EMPTY_TREE_HASH.into();
        let test_cases = [
            // Same sizes but the root hashes differ.
            (0, 0, root1, root2, proof1, true),
            (1, 1, root1, root2, proof1, true),
            // Sizes that are always consistent.
            (0, 0, empty_tree_hash, empty_tree_hash, proof1, false),
            (0, 1, empty_tree_hash, root2, proof1, false),
            (1, 1, root2, root2, proof1, false),
            // Time travel to the past.
            (1, 0, root1, root2, proof1, true),
            (2, 1, root1, root2, proof1, true),
            // Empty proof.
            (1, 2, root1, root2, proof1, true),
            // Roots don't match with equal size, append-only violated.
            (0, 0, empty_tree_hash, root2, proof1, true),
            (1, 1, empty_tree_hash, root2, proof1, true),
            // Roots match but the proof is not empty.
            (0, 0, empty_tree_hash, empty_tree_hash, &proof2, true),
            (0, 1, empty_tree_hash, empty_tree_hash, &proof2, true),
            (1, 1, empty_tree_hash, empty_tree_hash, &proof2, true),
        ];
        for (i, (size1, size2, r1, r2, proof, want_err)) in test_cases.into_iter().enumerate() {
            let res = Rfc6269Default::verify_consistency(size1, size2, proof, r1, r2);
            assert_eq!(
                res.is_err(),
                want_err,
                "unexpected proof result {res:?}, case {i}"
            );
        }

        for p in CONSISTENCY_PROOFS.into_iter() {
            let result = verifier_consistency_check(
                p.size1,
                p.size2,
                p.proof,
                &ROOTS[p.size1 as usize - 1],
                &ROOTS[p.size2 as usize - 1],
            );
            assert!(result.is_ok(), "failed with error: {result:?}");
        }
    }
}
