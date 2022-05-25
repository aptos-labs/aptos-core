// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from 'aptos';
import { NODE_URL } from 'core/constants';

export interface GetTransactionProps {
  nodeUrl?: string,
  txnHashOrVersion?: string;
}

export const getTransaction = async ({
  txnHashOrVersion,
  nodeUrl = NODE_URL,
}: GetTransactionProps) => {
  const aptosClient = new AptosClient(nodeUrl);
  if (txnHashOrVersion) {
    const txn = await aptosClient.getTransaction(txnHashOrVersion);
    return txn;
  }
  return undefined;
};

export const getUserTransaction = async ({
  txnHashOrVersion,
  nodeUrl = NODE_URL,
}: GetTransactionProps) => {
  const aptosClient = new AptosClient(nodeUrl);
  if (txnHashOrVersion) {
    const txn = await aptosClient.getTransaction(txnHashOrVersion);
    if ('events' in txn && 'signature' in txn) {
      return txn;
    }
  }
  return undefined;
};
