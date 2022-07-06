// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable class-methods-use-this */
/* eslint-disable max-classes-per-file */

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
  SIGN_MESSAGE: 'signMessage',
  SIGN_TRANSACTION: 'signTransaction',
} as const);

export const PermissionType = Object.freeze({
  CONNECT: 'connect',
  SIGN_AND_SUBMIT_TRANSACTION: 'signAndSubmitTransaction',
  SIGN_MESSAGE: 'signMessage',
  SIGN_TRANSACTION: 'signTransaction',
} as const);

export const PromptMessage = Object.freeze({
  APPROVED: 'approved',
  LOADED: 'loaded',
  REJECTED: 'rejected',
} as const);

export interface PromptInfo {
  domain: string | undefined;
  imageURI: string | undefined;
  promptType: string;
  title: string | undefined;
}
