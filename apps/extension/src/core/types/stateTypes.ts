// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosAccountObject } from 'aptos';

export type AptosAccountState = AptosAccount | undefined;

export interface Mnemonic {
  mnemonic: string;
  seed: Uint8Array;
}
export type MnemonicState = Mnemonic | undefined;

export interface WalletAccount {
  aptosAccount: AptosAccountObject
  mnemonic: MnemonicState
}

export interface LocalStorageState {
  accounts?: {
    [address: string]: WalletAccount
  },
  currAccountAddress?: string;
}
