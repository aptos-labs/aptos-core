// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient, MaybeHexString } from 'aptos';
import useWalletState from 'core/hooks/useWalletState';
import { useCallback } from 'react';
import { useQuery } from 'react-query';
import { ScriptFunctionPayload, UserTransaction } from 'aptos/dist/api/data-contracts';

export const transactionQueryKeys = Object.freeze({
  getCoinTransferTransactions: 'getCoinTransferTransactions',
} as const);

export interface GetTransactionProps {
  nodeUrl: string,
  txnHashOrVersion?: string;
}

export const getTransaction = async ({
  nodeUrl,
  txnHashOrVersion,
}: GetTransactionProps) => {
  const aptosClient = new AptosClient(nodeUrl);
  if (txnHashOrVersion) {
    return aptosClient.getTransaction(txnHashOrVersion);
  }
  return undefined;
};

export const getUserTransaction = async ({
  nodeUrl,
  txnHashOrVersion,
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

export interface GetAccountUserTransactionsProps {
  address: MaybeHexString;
  nodeUrl: string,
}

/**
 * Get successful user transactions for the specified account
 */
export async function getAccountUserTransactions({
  address,
  nodeUrl,
}: GetAccountUserTransactionsProps) {
  const aptosClient = new AptosClient(nodeUrl);
  const transactions = await aptosClient.getAccountTransactions(address);

  return transactions
    .filter((txn) => !txn.vm_status.includes('Move abort'))
    .filter((t) => t.type === 'user_transaction')
    .map((t) => t as UserTransaction);
}

export interface GetScriptFunctionTransactionsProps {
  address: MaybeHexString;
  functionName: string,
  nodeUrl: string,
}

/**
 * Get transactions that ran a specific function for the provided account
 */
export async function getScriptFunctionTransactions({
  address,
  functionName,
  nodeUrl,
}: GetScriptFunctionTransactionsProps) {
  const userTransactions = await getAccountUserTransactions({ address, nodeUrl });
  return userTransactions
    .filter((t) => t.payload.type === 'script_function_payload'
      && (t.payload as ScriptFunctionPayload).function === functionName);
}

export function useCoinTransferTransactions() {
  const { aptosAccount, aptosNetwork } = useWalletState();

  const getCoinTransferTransactionsQuery = useCallback(
    async () => (aptosAccount ? getScriptFunctionTransactions({
      address: aptosAccount.address(),
      functionName: '0x1::Coin::transfer',
      nodeUrl: aptosNetwork,
    }) : null),
    [aptosAccount, aptosNetwork],
  );

  return useQuery(
    transactionQueryKeys.getCoinTransferTransactions,
    getCoinTransferTransactionsQuery,
  );
}
