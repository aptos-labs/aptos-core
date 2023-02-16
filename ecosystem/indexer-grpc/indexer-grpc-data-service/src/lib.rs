// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod service;

pub fn get_grpc_address() -> String {
    std::env::var("GRPC_ADDRESS").expect("GRPC_ADDRESS not set")
}
