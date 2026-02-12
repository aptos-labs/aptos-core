// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod homomorphism;
pub mod proof;
pub mod traits;

pub use proof::{FirstProofItem, Proof};
pub use traits::{Statement, Trait, Witness};
