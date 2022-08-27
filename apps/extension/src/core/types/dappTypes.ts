// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export enum MessageMethod {
  CONNECT = 'connect',
  DISCONNECT = 'disconnect',
  GET_ACCOUNT_ADDRESS = 'getAccountAddress',
  GET_NETWORK = 'getNetwork',
  IS_CONNECTED = 'is_connected',
  SIGN_AND_SUBMIT_TRANSACTION = 'signAndSubmitTransaction',
  SIGN_MESSAGE = 'signMessage',
  SIGN_TRANSACTION = 'signTransaction',
}

export enum Permission {
  CONNECT = 'connect',
  SIGN_AND_SUBMIT_TRANSACTION = 'signAndSubmitTransaction',
  SIGN_MESSAGE = 'signMessage',
  SIGN_TRANSACTION = 'signTransaction',
}

export interface PermissionPromptType {
  kind: 'permission'
  permission: Permission
}

export interface WarningPromptType {
  kind: 'warning'
  warning: 'noAccounts' // we may add more in the future
}

export type PromptType = PermissionPromptType | WarningPromptType;

export function warningPrompt(): WarningPromptType {
  return {
    kind: 'warning',
    warning: 'noAccounts',
  };
}

export function permissionPrompt(permission: Permission): PermissionPromptType {
  return {
    kind: 'permission',
    permission,
  };
}

export const PromptMessage = Object.freeze({
  APPROVED: 'approved',
  LOADED: 'loaded',
  REJECTED: 'rejected',
  TIME_OUT: 'timeout',
} as const);

export interface PromptInfo {
  domain: string | undefined;
  imageURI: string | undefined;
  promptType: PromptType;
  title: string | undefined;
}
