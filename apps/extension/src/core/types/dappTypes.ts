// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export const MessageMethod = Object.freeze({
  CONNECT: 'connect',
  DISCONNECT: 'disconnect',
  GET_ACCOUNT_ADDRESS: 'getAccountAddress',
  GET_CHAIN_ID: 'getChainID',
  GET_NETWORK: 'getNetwork',
  GET_SEQUENCE_NUMBER: 'getSequenceNumber',
  IS_CONNECTED: 'is_connected',
  SIGN_MESSAGE: 'signMessage',
  SIGN_TRANSACTION: 'signTransaction',
  SUBMIT_TRANSACTION: 'submitTransaction',
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
