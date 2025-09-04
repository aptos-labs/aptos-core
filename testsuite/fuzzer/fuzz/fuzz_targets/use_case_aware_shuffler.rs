#![no_main]
// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_consensus::transaction_shuffler_fuzzing::use_case_aware::{Config, UseCaseAwareShuffler};
use velor_types::transaction::use_case::{UseCaseAwareTransaction, UseCaseKey};
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;

#[derive(Arbitrary, Debug, PartialEq, Eq, Clone)]
pub enum UseCaseKeyU8 {
    Platform,
    ContractAddress(u8),
    Others,
}

#[derive(Arbitrary, Debug, PartialEq, Eq, Clone)]
struct Tx {
    sender: u8,
    k: UseCaseKeyU8,
    id: usize,
}

#[derive(Arbitrary, Debug)]
struct FuzzData {
    data: Vec<Tx>,
    test_determinism: bool,
}

impl UseCaseAwareTransaction for Tx {
    fn parse_sender(&self) -> AccountAddress {
        let mut addr = [0u8; AccountAddress::LENGTH];
        addr[AccountAddress::LENGTH - 1] = self.sender;
        AccountAddress::new(addr)
    }

    fn parse_use_case(&self) -> UseCaseKey {
        match self.k {
            UseCaseKeyU8::Platform => UseCaseKey::Platform,
            UseCaseKeyU8::ContractAddress(account_address) => {
                let mut addr = [0u8; AccountAddress::LENGTH];
                addr[AccountAddress::LENGTH - 1] = account_address;
                UseCaseKey::ContractAddress(AccountAddress::new(addr))
            },
            UseCaseKeyU8::Others => UseCaseKey::Others,
        }
    }
}

fn run(mut fuzz_data: FuzzData) {
    // production config
    let shuf = UseCaseAwareShuffler {
        config: Config {
            sender_spread_factor: 32,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 4,
        },
    };
    for i in 0..fuzz_data.data.len() {
        fuzz_data.data[i].id = i;
    }

    let res = shuf.shuffle_generic(fuzz_data.data.clone());
    if fuzz_data.test_determinism {
        let res1 = shuf.shuffle_generic(fuzz_data.data);
        assert!(res == res1);
    }
}

fuzz_target!(|fuzz_data: FuzzData| {
    run(fuzz_data);
});
