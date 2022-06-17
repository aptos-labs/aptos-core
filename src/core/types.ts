/* eslint-disable class-methods-use-this */
/* eslint-disable max-classes-per-file */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount, AptosAccountObject } from 'aptos';

export type AptosAccountState = AptosAccount | undefined;

export interface LocalStorageState {
  aptosAccountObject?: AptosAccountObject,
}

export const MessageMethod = Object.freeze({
  CONNECT: 'connect',
  DISCONNECT: 'disconnect',
  GET_ACCOUNT_ADDRESS: 'getAccountAddress',
  IS_CONNECTED: 'is_connected',
  SIGN_AND_SUBMIT_TRANSACTION: 'signAndSubmitTransaction',
  SIGN_TRANSACTION: 'signTransaction',
} as const);
