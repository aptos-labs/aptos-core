// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

mod dynamic_analysis;
mod normalize;

pub use dynamic_analysis::ConcretizedSecondaryIndexes;
pub use normalize::NormalizedReadWriteSetAnalysis;
