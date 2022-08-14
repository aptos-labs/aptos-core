// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export const accountNamespace = '0x1::account';
export const coinNamespace = '0x1::coin';
export const aptosCoinStructTag = '0x1::aptos_coin::AptosCoin';
export const coinStoreStructTag = `${coinNamespace}::CoinStore`;
export const aptosCoinStoreStructTag = `${coinStoreStructTag}<${aptosCoinStructTag}>`;

export const validStorageUris = [
  'amazonaws.com',
  'ipfs.io',
  'arweave.net',
];
