// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosAccountObject } from 'aptos';

export type AptosAccountState = AptosAccount | undefined;

export interface LocalStorageState {
  aptosAccounts?: {
    [address: string]: AptosAccountObject
  },
  currAccountAddress?: string;
}
