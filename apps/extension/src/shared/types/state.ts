// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Accounts, EncryptedAccounts } from './account';
import { Networks } from './network';

export interface PersistentState {
  activeAccountAddress: string | undefined,
  activeAccountPublicKey: string | undefined,
  activeNetworkName: string | undefined,
  customNetworks: Networks | undefined,
  encryptedAccounts: EncryptedAccounts | undefined,
  encryptedStateVersion: number,
  salt: string | undefined,
}

export interface SessionState {
  accounts: Accounts | undefined,
  encryptionKey: string | undefined,
}

export type PersistentStorageKey = keyof PersistentState;
export type SessionStorageKey = keyof SessionState;
