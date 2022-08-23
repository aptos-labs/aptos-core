// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export const KEY_LENGTH: number = 64;
export const WALLET_STATE_LOADED_KEY = 'aptosStateLoaded';
export const WALLET_ENCRYPTED_ACCOUNTS_KEY = 'aptosEncryptedAccounts';
export const WALLET_ACCOUNTS_KEY = 'accounts';
export const WALLET_STATE_ACCOUNT_ADDRESS_KEY = 'activeAccount';
export const WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY = 'activeNetwork';
export const WALLET_STATE_CUSTOM_NETWORKS_STORAGE_KEY = 'customNetworks';
export const WALLET_STATE_STYLE_INDEX_KEY = 'accountStyleIndex';

export const STATIC_GAS_AMOUNT = 150;

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
