// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';

export interface ConnectPermission {
  type: 'connect',
}

export interface SignAndSubmitTransactionPermission {
  payload: Types.EntryFunctionPayload,
  type: 'signAndSubmitTransaction',
}

export interface SignMessagePermission {
  message: string,
  type: 'signMessage',
}

export interface SignTransactionPermission {
  payload: Types.EntryFunctionPayload,
  type: 'signTransaction',
}

export type Permission = ConnectPermission
| SignAndSubmitTransactionPermission
| SignMessagePermission
| SignTransactionPermission;
