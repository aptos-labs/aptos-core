// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types, TxnBuilderTypes } from 'aptos';
import axios from 'axios';
import { useCallback } from 'react';
import {
  buildRawTransactionFromBCSPayload,
  buildRawTransactionFromJsonPayload, maxGasFeeFromEstimated,
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

type UserTransaction = Types.UserTransaction;
type RawTransaction = TxnBuilderTypes.RawTransaction;
type TransactionPayload = TxnBuilderTypes.TransactionPayload | Types.EntryFunctionPayload;

// Taken from https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/aptos-gas/src/transaction.rs
export const maxPricePerGasUnit = 10_000;
export const maxNumberOfGasUnits = 4_000_000;

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

export function useGasUnitPrice() {
  const { activeNetwork } = useNetworks();
  const queryClient = useQueryClient();

  const getGasUnitPriceEstimate = useCallback(
    () => queryClient.fetchQuery(
      ['getGasPriceEstimate', activeNetwork.nodeUrl],
      async () => {
        const gasPriceEstimationEndpoint = `${activeNetwork.nodeUrl}/v1/estimate_gas_price`;
        const result = await axios.get<{ gas_estimate: number }>(gasPriceEstimationEndpoint);
        return result.data.gas_estimate;
      },
      {
        staleTime: 10000,
      },
    ),
    [activeNetwork.nodeUrl, queryClient],
  );

  return { getGasUnitPriceEstimate };
}

export function useTransactions() {
  const { aptosClient } = useNetworks();
  const { aptosAccount } = useActiveAccount();

  const { get: getSequenceNumber } = useSequenceNumber();

  async function buildRawTransaction(
    payload: TransactionPayload,
    options?: TransactionOptions,
  ) {
    const [chainId, sequenceNumber] = await Promise.all([
      aptosClient.getChainId(),
      getSequenceNumber(),
    ]);

    return payload instanceof TxnBuilderTypes.TransactionPayload
      ? buildRawTransactionFromBCSPayload(
        aptosAccount.address(),
        sequenceNumber,
        chainId,
        payload,
        options,
      )
      : buildRawTransactionFromJsonPayload(
        aptosClient,
        aptosAccount.address(),
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
type PayloadOrFactory<TParams = void> = TransactionPayload | PayloadFactory<TParams>;

/**
 * Allow the consumer to specify the max gas amount.
 * Ideally we specify the minimum of the coin balance and the cap for `maxGasAmount`.
 * TODO: just fetch it internally in the hook, reusing the query hook for the balance
 */
export interface UseTransactionSimulationOptions {
  maxGasOctaAmount?: number,
}

export function useTransactionSimulation(
  key: QueryKey,
  payloadOrFactory: PayloadOrFactory,
  options?: UseQueryOptions<UserTransaction, Error> & UseTransactionSimulationOptions,
) {
  const { invalidate: invalidateSeqNumber } = useSequenceNumber();
  const {
    buildRawTransaction,
    simulateTransaction,
  } = useTransactions();
  const { getGasUnitPriceEstimate } = useGasUnitPrice();

  return useQuery(
    key,
    async () => {
      const payload = payloadOrFactory instanceof Function ? payloadOrFactory() : payloadOrFactory;
      const gasUnitPriceEstimate = await getGasUnitPriceEstimate();
      const txnOptions: TransactionOptions = {
        gasUnitPrice: gasUnitPriceEstimate,
      };

      if (options?.maxGasOctaAmount !== undefined) {
        const maxGasAmountEstimate = options.maxGasOctaAmount / gasUnitPriceEstimate;
        txnOptions.maxGasAmount = Math.min(maxGasAmountEstimate, maxNumberOfGasUnits);
      }

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
  const { getGasUnitPriceEstimate } = useGasUnitPrice();
  const {
    buildRawTransaction,
    submitTransaction,
  } = useTransactions();

  return useMutation(
    async (params: TParams) => {
      const payload = payloadFactory(params);
      const gasUnitPriceEstimate = await getGasUnitPriceEstimate();
      const txnOptions: TransactionOptions = {
        gasUnitPrice: gasUnitPriceEstimate,
      };

      if (options?.estimatedGasFee !== undefined) {
        txnOptions.maxGasAmount = maxGasFeeFromEstimated(options.estimatedGasFee);
      }

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
