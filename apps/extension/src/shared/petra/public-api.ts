// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PublicAccount } from 'shared/types';
import { Types } from 'aptos';

type EntryFunctionPayload = Types.EntryFunctionPayload;
type PendingTransaction = Types.PendingTransaction;

export interface SignMessagePayload {
  address?: boolean;
  application?: boolean;
  chainId?: boolean;
  message: string;
  nonce: string;
}

export interface SignMessageResponse {
  address: string;
  application: string;
  chainId: number;
  fullMessage: string;
  message: string;
  nonce: string,
  prefix: string,
  signature: string;
}

export interface PetraPublicApi {
  account(): Promise<PublicAccount>;
  connect(): Promise<PublicAccount>;
  disconnect(): Promise<void>;
  isConnected(): Promise<boolean>;
  network(): Promise<string>;
  signAndSubmitTransaction(payload: EntryFunctionPayload): Promise<PendingTransaction>;
  signMessage(payload: SignMessagePayload): Promise<SignMessageResponse>;
  signTransaction(payload: EntryFunctionPayload): Promise<Uint8Array>;
}

export default PetraPublicApi;
