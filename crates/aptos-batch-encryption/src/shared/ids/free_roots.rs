// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::{Id, IdSet, OssifiedIdSet};
use crate::{
    group::{Fr, G1Affine, G1Projective},
    shared::{
        algebra::mult_tree::{compute_mult_tree, quotient},
        ark_serialize::*,
    },
};
use ark_ec::VariableBaseMSM;
use ark_ff::field_hashers::{DefaultFieldHasher, HashToField};
use ark_poly::univariate::DensePolynomial;
use ed25519_dalek::VerifyingKey;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
