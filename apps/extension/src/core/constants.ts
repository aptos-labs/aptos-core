// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export const KEY_LENGTH: number = 64;
export const WALLET_STATE_LOCAL_STORAGE_KEY = 'aptosWalletState';
export const WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY = 'aptosWalletNetworkState';

export const STATIC_GAS_AMOUNT = 150;

export const coinNamespace = '0x1::coin';
export const aptosCoinStructTag = '0x1::aptos_coin::AptosCoin';
export const coinStoreStructTag = `${coinNamespace}::CoinStore`;
export const aptosCoinStoreStructTag = `${coinStoreStructTag}<${aptosCoinStructTag}>`;

export const validStorageUris = [
  'amazonaws.com',
  'ipfs.io',
  'arweave.net',
];
