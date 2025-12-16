// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_core_types::language_storage::TypeTag;
use once_cell::sync::Lazy;
use std::str::FromStr;

pub static RANDOMNESS_GENERATED_EVENT_MOVE_TYPE_TAG: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::from_str("0x1::randomness::RandomnessGeneratedEvent").expect("Cannot fail")
});
