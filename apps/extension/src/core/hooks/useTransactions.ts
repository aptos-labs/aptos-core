// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { UserTransaction } from 'aptos/dist/generated';
import { RawTransaction, TransactionPayload } from 'aptos/dist/transaction_builder/aptos_types';
import {
  buildRawTransaction as buildRawTransactionInternal,
  simulateTransaction as simulateTransactionInternal,
  submitTransaction as submitTransactionInternal,
  TransactionOptions,
} from 'shared/transactions';

import {
  QueryKey, useMutation, UseMutationOptions, useQuery, useQueryClient, UseQueryOptions,
} from 'react-query';

import { useNetworks } from 'core/hooks/useNetworks';
import { MoveVmError, MoveStatusCode } from 'shared/move';
import { useActiveAccount } from 'core/hooks/useAccounts';

function isSequenceNumberTooOldError(err: unknown) {
  return err instanceof MoveVmError
    && err.statusCode === MoveStatusCode.SEQUENCE_NUMBER_TOO_OLD;
}

/**
 * Query sequence number for current account,
 * which is required to BCS-encode a transaction locally.
 * The value is queried lazily the first time `get` is called, and is
 * refetched only when an error occurs, by invalidating the cache
 */
export function useSequenceNumber() {
  const { activeAccountAddress } = useActiveAccount();
  const { aptosClient } = useNetworks();
  const queryClient = useQueryClient();

  const queryKey = ['getSequenceNumber', activeAccountAddress];

  const fetchSeqNumber = async () => queryClient.fetchQuery(queryKey, async () => {
    const account = await aptosClient.getAccount(activeAccountAddress);
    return BigInt(account.sequence_number);
  }, {
    staleTime: Infinity,
  });

  return {
    get: fetchSeqNumber,
    increment: async () => {
      const currSeqNumber = queryClient.getQueryData<bigint>(queryKey)
        ?? (await fetchSeqNumber());
      return queryClient.setQueryData<bigint>(queryKey, currSeqNumber + 1n);
    },
    invalidate: () => {
      // eslint-disable-next-line no-console
      console.warn('Invalidating sequence number');
      return queryClient.invalidateQueries(queryKey);
    },
  };
}

export function useTransactions() {
  const { aptosClient } = useNetworks();
  const { aptosAccount } = useActiveAccount();

  const { get: getSequenceNumber } = useSequenceNumber();

  async function buildRawTransaction(payload: TransactionPayload, options?: TransactionOptions) {
    const [chainId, sequenceNumber] = await Promise.all([
      aptosClient.getChainId(),
      getSequenceNumber(),
    ]);

    return buildRawTransactionInternal(
      aptosAccount.address(),
      sequenceNumber,
      chainId,
      payload,
      options,
    );
  }

  const simulateTransaction = (rawTxn: RawTransaction) => simulateTransactionInternal(
    aptosAccount,
    aptosClient,
    rawTxn,
  );

  const submitTransaction = (rawTxn: RawTransaction) => submitTransactionInternal(
    aptosAccount,
    aptosClient,
    rawTxn,
  );

  return {
    aptosClient,
    buildRawTransaction,
    simulateTransaction,
    submitTransaction,
  };
}

type PayloadFactory<TParams = void> = (params: TParams) => TransactionPayload;

/**
 * Allow the consumer to specify the max gas amount.
 * Ideally we specify the minimum of the coin balance and the cap for `maxGasAmount`.
 * TODO: just fetch it internally in the hook, reusing the query hook for the balance
 */
export interface UseTransactionSimulationOptions {
  maxGasAmount?: number,
}

export function useTransactionSimulation(
  key: QueryKey,
  payloadFactory: PayloadFactory,
  options?: UseQueryOptions<UserTransaction, Error> & UseTransactionSimulationOptions,
) {
  const { invalidate: invalidateSeqNumber } = useSequenceNumber();
  const {
    buildRawTransaction,
    simulateTransaction,
  } = useTransactions();

  return useQuery(
    key,
    async () => {
      const payload = payloadFactory();
      // TODO: Should cap by maximum maxGasAmount
      const txnOptions = options?.maxGasAmount
        ? { maxGasAmount: options.maxGasAmount }
        : {};
      const rawTxn = await buildRawTransaction(payload, txnOptions);
      try {
        return await simulateTransaction(rawTxn);
      } catch (err) {
        if (isSequenceNumberTooOldError(err)) {
          await invalidateSeqNumber();
        }
        throw err;
      }
    },
    {
      retry: (count, err) => count === 0 && isSequenceNumberTooOldError(err),
      ...options,
    },
  );
}

/**
 * Allow the user to specify an externally estimated gas fee that will be used
 * to compute the maxGasAmount
 */
export interface UseTransactionSubmitOptions {
  estimatedGasFee?: number,
}

export function useTransactionSubmit<TParams>(
  payloadFactory: PayloadFactory<TParams>,
  options?: UseMutationOptions<UserTransaction, Error, TParams> & UseTransactionSubmitOptions,
) {
  const {
    increment: incrementSeqNumber,
    invalidate: invalidateSeqNumber,
  } = useSequenceNumber();
  const {
    buildRawTransaction,
    submitTransaction,
  } = useTransactions();

  return useMutation(
    async (params: TParams) => {
      const payload = payloadFactory(params);
      const txnOptions = options?.estimatedGasFee
        ? { maxGasAmount: options.estimatedGasFee * 2 }
        : {};
      const rawTxn = await buildRawTransaction(payload, txnOptions);
      try {
        return await submitTransaction(rawTxn);
      } catch (err) {
        if (isSequenceNumberTooOldError(err)) {
          await invalidateSeqNumber();
        }

        throw err;
      }
    },
    {
      retry: (count, err) => count === 0 && isSequenceNumberTooOldError(err),
      ...options,
      async onSuccess(...params) {
        await incrementSeqNumber();
        if (options?.onSuccess) {
          options.onSuccess(...params);
        }
      },
    },
  );
}
