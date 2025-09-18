// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::language_storage::TypeTag;
use once_cell::sync::Lazy;
use std::str::FromStr;

pub static RANDOMNESS_GENERATED_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::from_str("0x1::randomness::RandomnessGeneratedEvent").expect("Cannot fail")
});
