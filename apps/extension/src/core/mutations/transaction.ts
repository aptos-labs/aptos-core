// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { UserTransaction } from 'aptos/dist/generated';
import { UseMutationOptions, useQueryClient, UseQueryOptions } from 'react-query';
import {
  useTransactionSimulation,
  UseTransactionSimulationOptions,
  useTransactionSubmit, UseTransactionSubmitOptions,
} from 'core/hooks/useTransactions';
import queryKeys from 'core/queries/queryKeys';
import { buildAccountTransferPayload, buildCoinTransferPayload } from 'shared/transactions';

export interface UseCoinTransferParams {
  amount: number | undefined,
  doesRecipientExist: boolean | undefined,
  recipient: string | undefined,
}

/**
 * Query a coin transfer simulation for the specified recipient and amount
 */
export function useCoinTransferSimulation(
  {
    amount,
    doesRecipientExist,
    recipient,
  }: UseCoinTransferParams,
  options?: UseQueryOptions<UserTransaction, Error> & UseTransactionSimulationOptions,
) {
  const isReady = recipient !== undefined
    && amount !== undefined
    && amount >= 0;

  return useTransactionSimulation(
    [queryKeys.getCoinTransferSimulation, recipient, amount],
    () => (doesRecipientExist
      ? buildCoinTransferPayload(recipient!, amount!)
      : buildAccountTransferPayload(recipient!, amount!)),
    {
      ...options,
      enabled: isReady && options?.enabled,
    },
  );
}

export interface SubmitCoinTransferParams {
  amount: number,
  doesRecipientExist: boolean,
  recipient: string,
}

/**
 * Mutation for submitting a coin transfer transaction
 */
export function useCoinTransferTransaction(
  options?: UseMutationOptions<UserTransaction, Error, SubmitCoinTransferParams>
  & UseTransactionSubmitOptions,
) {
  const queryClient = useQueryClient();

  return useTransactionSubmit(
    ({
      amount,
      doesRecipientExist,
      recipient,
    }: SubmitCoinTransferParams) => (doesRecipientExist
      ? buildCoinTransferPayload(recipient, amount)
      : buildAccountTransferPayload(recipient, amount)),
    {
      ...options,
      onSuccess(txn, data, ...rest) {
        queryClient.invalidateQueries(queryKeys.getAccountCoinBalance);

        // TODO: re-enable when fixing analytics
        // const { amount } = data;
        //
        // const eventType = txn.success
        //   ? coinEvents.TRANSFER_APTOS_COIN
        //   : coinEvents.ERROR_TRANSFER_APTOS_COIN;
        //
        // const payload = txn.payload as EntryFunctionPayload;
        // const coinType = payload.type_arguments[0];
        //
        // const params = {
        //   amount,
        //   coinType,
        //   fromAddress: txn.sender,
        //   network: activeNetwork.nodeUrl,
        //   ...txn,
        // };
        //
        // Analytics.event({ eventType, params });

        if (options?.onSuccess) {
          options.onSuccess(txn, data, ...rest);
        }
      },
    },
  );
}
