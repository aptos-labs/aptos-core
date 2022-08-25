// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { PublicAccount } from 'shared/types';
import {
  PendingTransaction,
  SubmitTransactionRequest,
  TransactionPayload,
} from 'aptos/dist/generated';

export interface PetraPublicApi {
  account(): Promise<PublicAccount>;
  connect(): Promise<PublicAccount>;
  disconnect(): Promise<void>;
  isConnected(): Promise<boolean>;
  network(): Promise<string>;
  signAndSubmitTransaction(payload: TransactionPayload): Promise<PendingTransaction>;
  signMessage(message: string): Promise<string>;
  signTransaction(payload: TransactionPayload): Promise<SubmitTransactionRequest>;
}

export default PetraPublicApi;
