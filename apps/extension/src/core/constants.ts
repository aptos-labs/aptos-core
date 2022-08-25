// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export const accountNamespace = '0x1::aptos_account';
export const coinNamespace = '0x1::coin';
export const stakeNamespace = '0x1::stake';
export const aptosCoinStructTag = '0x1::aptos_coin::AptosCoin';
export const coinStoreStructTag = `${coinNamespace}::CoinStore` as const;
export const aptosCoinStoreStructTag = `${coinStoreStructTag}<${aptosCoinStructTag}>` as const;
export const aptosStakePoolStructTag = `${stakeNamespace}::StakePool` as const;

export const validStorageUris = [
  'amazonaws.com',
  'ipfs.io',
  'arweave.net',
];

export const settingsItemLabel = {
  HELP_SUPPORT: 'Help & Support',
  LOCK_WALLET: 'Lock wallet',
  NETWORK: 'Network',
  SECRET_RECOVERY_PHRASE: 'Show secret recovery phrase',
  SHOW_CREDENTIALS: 'Show credentials',
};
