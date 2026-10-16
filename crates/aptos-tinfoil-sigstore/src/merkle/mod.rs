// Adapted from sigstore-rs (Apache 2.0 License)
// https://github.com/sigstore/sigstore-rs/blob/34f232af72ba6108f001f1612fdb03c87af8ca62/src/crypto/merkle/mod.rs
//
// Copyright 2023 The Sigstore Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0

pub mod proof_verification;
pub mod rfc6962;

pub use proof_verification::MerkleProofVerifier;
pub use rfc6962::{Rfc6269Default, Rfc6269HasherTrait};
