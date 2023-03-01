// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//:!:>section_1
export const NODE_URL = process.env.APTOS_NODE_URL || "http://0.0.0.0:8080";
export const FAUCET_URL = process.env.APTOS_FAUCET_URL || "http://0.0.0.0:8081";
//<:!:section_1

export const aptosCoinStore = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
