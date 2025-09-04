// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::needless_return)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::let_and_return)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::declare_interior_mutable_const)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::map_identity)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::vec_init_then_push)]
#![allow(clippy::to_string_in_format_args)]
#![allow(clippy::borrow_interior_mutable_const)]

use crate::constants::SCALAR_FIELD_ORDER;
pub use constants::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_NUM_BYTES};
pub use utils::random::DST_RAND_CORE_HELL;

pub mod algebra;
pub mod constants;
pub mod pvss;
pub mod utils;
pub mod weighted_vuf;
