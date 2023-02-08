// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//:!:>section_1
export const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
export const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";
export const INDEXER_URL = process.env.INDEXER_URL || "https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql";
//<:!:section_1

export const aptosCoinStore = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
