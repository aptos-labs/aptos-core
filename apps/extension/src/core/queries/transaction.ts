// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosClient } from 'aptos';
import { useQuery, UseQueryOptions } from 'react-query';
import { EntryFunctionPayload, Event, UserTransaction } from 'aptos/dist/generated';
import { useNetworks } from 'core/hooks/useNetworks';
import { accountNamespace, coinNamespace } from 'core/constants';

export const transactionQueryKeys = Object.freeze({
  getAccountLatestTransactionTimestamp: 'getAccountLatestTransactionTimestamp',
  getCoinTransferSimulation: 'getCoinTransferSimulation',
  getCoinTransferTransactions: 'getCoinTransferTransactions',
  getTransaction: 'getTransaction',
  getUserTransactions: 'getUserTransactions',
} as const);

/**
 * Get successful user transactions for the specified account
 */
export async function getUserTransactions(aptosClient: AptosClient, address: string) {
  const transactions = await aptosClient.getAccountTransactions(address, { limit: 200 });
  return transactions
    .filter((t) => t.type === 'user_transaction')
    .map((t) => t as UserTransaction)
    .filter((t) => t.success);
}

export async function getEntryFunctionTransactions(
  aptosClient: AptosClient,
  address: string,
  functionName: string | string[],
) {
  const transactions = await getUserTransactions(aptosClient, address);
  const functionNames = Array.isArray(functionName) ? functionName : [functionName];
  return transactions
    .filter((t) => t.payload.type === 'entry_function_payload'
      && functionNames.indexOf((t.payload as EntryFunctionPayload).function) >= 0);
}

export async function getTransactionEvents(
  aptosClient: AptosClient,
  address: string,
  eventType: string | string[],
) {
  const transactions = await getUserTransactions(aptosClient, address);
  const eventTypes = Array.isArray(eventType) ? eventType : [eventType];
  const events: Event[] = [];
  transactions.forEach((t) => {
    const foundEvents = t.events.filter((event) => eventTypes.indexOf(event.type) !== -1);
    events.push(...foundEvents);
  });
  return events;
}

// region Use transactions

export function useUserTransactions(
  address: string | undefined,
  options?: UseQueryOptions<UserTransaction[]>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<UserTransaction[]>(
    [transactionQueryKeys.getUserTransactions, address],
    async () => getUserTransactions(aptosClient, address!),
    {
      ...options,
      enabled: Boolean(aptosClient && address) && options?.enabled,
    },
  );
}

export function useCoinTransferTransactions(
  address: string | undefined,
  options?: UseQueryOptions<UserTransaction[]>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<UserTransaction[]>(
    [transactionQueryKeys.getCoinTransferTransactions, address],
    async () => getEntryFunctionTransactions(
      aptosClient,
      address!,
      [`${coinNamespace}::transfer`, `${accountNamespace}::transfer`],
    ),
    {
      ...options,
      enabled: Boolean(aptosClient && address) && options?.enabled,
    },
  );
}

// endregion

export const useTransaction = (
  version: number | undefined,
  options?: UseQueryOptions<UserTransaction>,
) => {
  const { aptosClient } = useNetworks();

  return useQuery<UserTransaction>(
    [transactionQueryKeys.getTransaction, version],
    async () => aptosClient.getTransactionByVersion(BigInt(version!)) as Promise<UserTransaction>,
    {
      ...options,
      enabled: Boolean(aptosClient && version) && options?.enabled,
    },
  );
};

export function useAccountLatestTransactionTimestamp(
  address?: string,
  options?: UseQueryOptions<Date | undefined>,
) {
  const { aptosClient } = useNetworks();

  return useQuery<Date | undefined>(
    [
      transactionQueryKeys.getAccountLatestTransactionTimestamp,
      address,
    ],
    async () => {
      const txns = await aptosClient.getAccountTransactions(address!, { limit: 1 });
      const latestTxn = (txns as UserTransaction[]).pop();
      return latestTxn && new Date(Number(latestTxn?.timestamp) / 1000);
    },
    {
      ...options,
      enabled: Boolean(address) && options?.enabled,
    },
  );
}
