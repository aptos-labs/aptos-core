// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { MNEMONIC } from 'core/enums';

export const coinStoreResource = 'CoinStore';
export const coinInfoResource = 'CoinInfo';
export const accountNamespace = '0x1::aptos_account';
export const coinNamespace = '0x1::coin';
export const aptosCoinStructTag = '0x1::aptos_coin::AptosCoin';
export const coinStoreStructTag = `${coinNamespace}::${coinStoreResource}` as const;
export const aptosCoinStoreStructTag = `${coinStoreStructTag}<${aptosCoinStructTag}>` as const;
export const aptosStakePoolStructTag = '0x1::stake::StakePool' as const;

// faucet
export const defaultFundAmount = 1000000000;

export const latestVersion = 1;

export const passwordStrength = 2;

export const validStorageUris = [
  'amazonaws.com',
  'ipfs.io',
  'arweave.net',
];

export const settingsItemLabel = {
  EXPLORER: 'View on explorer',
  EXPORT_PUBLIC_PRIVATE_KEY: 'Show public & private keys',
  HELP_SUPPORT: 'Help & Support',
  LOCK_WALLET: 'Lock wallet',
  NETWORK: 'Network',
  REMOVE_ACCOUNT: 'Remove account',
  SECRET_RECOVERY_PHRASE: 'Show secret recovery phrase',
  SHOW_CREDENTIALS: 'Show credentials',
  SWITCH_ACCOUNT: 'Switch account',
};

export const mnemonicValues = [
  MNEMONIC.A,
  MNEMONIC.B,
  MNEMONIC.C,
  MNEMONIC.D,
  MNEMONIC.E,
  MNEMONIC.F,
  MNEMONIC.G,
  MNEMONIC.H,
  MNEMONIC.I,
  MNEMONIC.J,
  MNEMONIC.K,
  MNEMONIC.L,
];
