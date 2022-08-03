// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export const KEY_LENGTH: number = 64;
export const WALLET_ENCRYPTED_ACCOUNTS_KEY = 'aptosEncryptedAccounts';
export const WALLET_SESSION_ACCOUNTS_KEY = 'aptosSessionAccounts';
export const WALLET_STATE_ACCOUNT_ADDRESS_KEY = 'accountAddress';
export const WALLET_STATE_NETWORK_LOCAL_STORAGE_KEY = 'aptosWalletNetworkState';

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
