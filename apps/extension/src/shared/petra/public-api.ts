// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PublicAccount } from 'shared/types';
import {
  EntryFunctionPayload,
  PendingTransaction,
} from 'aptos/dist/generated';

export interface PetraPublicApi {
  account(): Promise<PublicAccount>;
  connect(): Promise<PublicAccount>;
  disconnect(): Promise<void>;
  isConnected(): Promise<boolean>;
  network(): Promise<string>;
  signAndSubmitTransaction(payload: EntryFunctionPayload): Promise<PendingTransaction>;
  signMessage(message: string): Promise<string>;
  signTransaction(payload: EntryFunctionPayload): Promise<Uint8Array>;
}

export default PetraPublicApi;
