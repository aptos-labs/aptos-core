// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::create_object_address;
use move_core_types::account_address::AccountAddress;

const OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR: &[u8] = b"velor_framework::object_code_deployment";

pub fn create_object_code_deployment_address(
    creator: AccountAddress,
    creator_sequence_number: u64,
) -> AccountAddress {
    let mut seed = vec![];
    seed.extend(bcs::to_bytes(OBJECT_CODE_DEPLOYMENT_DOMAIN_SEPARATOR).unwrap());
    seed.extend(bcs::to_bytes(&creator_sequence_number).unwrap());
    create_object_address(creator, &seed)
}
