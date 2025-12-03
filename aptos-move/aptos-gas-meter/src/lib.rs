// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This crate serves as the implementation of the standard gas meter and algebra used in the Aptos VM.
//! It also defines traits that enable composability of gas meters and algebra.

mod algebra;
mod meter;
mod traits;

pub use algebra::StandardGasAlgebra;
pub use meter::StandardGasMeter;
pub use traits::{AptosGasMeter, CacheValueSizes, GasAlgebra};
