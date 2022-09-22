// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { UseMutationOptions, useQueryClient, UseQueryOptions } from 'react-query';
import {
  useTransactionSimulation,
  UseTransactionSimulationOptions,
  useTransactionSubmit, UseTransactionSubmitOptions,
} from 'core/hooks/useTransactions';
import queryKeys from 'core/queries/queryKeys';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { coinEvents } from 'core/utils/analytics/events';
import { useNetworks } from 'core/hooks/useNetworks';
import { useAnalytics } from 'core/hooks/useAnalytics';
import { buildAccountTransferPayload, buildCoinTransferPayload } from 'shared/transactions';
import { CoinTransferTransaction, Transaction } from 'shared/types';

export interface UseCoinTransferParams {
  doesRecipientExist: boolean | undefined,
  octaAmount: bigint | undefined,
  recipient: string | undefined,
}

/**
 * Query a coin transfer simulation for the specified recipient and amount
 */
export function useCoinTransferSimulation(
  {
    doesRecipientExist,
    octaAmount,
    recipient,
  }: UseCoinTransferParams,
  options?: UseQueryOptions<Transaction, Error> & UseTransactionSimulationOptions,
) {
  const isReady = recipient !== undefined
    && octaAmount !== undefined
    && octaAmount >= 0n;

  return useTransactionSimulation(
    [queryKeys.getCoinTransferSimulation, recipient, octaAmount?.toString()],
    () => (doesRecipientExist
      ? buildCoinTransferPayload(recipient!, octaAmount!)
      : buildAccountTransferPayload(recipient!, octaAmount!)),
    {
      ...options,
      enabled: isReady && options?.enabled,
    },
  );
}

export interface SubmitCoinTransferParams {
  amount: bigint,
  doesRecipientExist: boolean,
  recipient: string,
}

/**
 * Mutation for submitting a coin transfer transaction
 */
export function useCoinTransferTransaction(
  options?: UseMutationOptions<Transaction, Error, SubmitCoinTransferParams>
  & UseTransactionSubmitOptions,
) {
  const queryClient = useQueryClient();
  const { activeAccountAddress } = useActiveAccount();
  const { trackEvent } = useAnalytics();
  const { activeNetwork } = useNetworks();

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
      async onMutate() {
        await Promise.all([
          queryClient.invalidateQueries([
            queryKeys.getAccountOctaCoinBalance,
            activeAccountAddress,
          ]),
          queryClient.invalidateQueries([
            queryKeys.getUserTransactions,
            activeAccountAddress,
          ]),
        ]);
      },
      async onSettled(txn, error, ...rest) {
        const coinTransfer = txn as CoinTransferTransaction | undefined;

        const eventType = txn?.success
          ? coinEvents.TRANSFER_APTOS_COIN
          : coinEvents.ERROR_TRANSFER_APTOS_COIN;

        const params = {
          amount: coinTransfer?.amount,
          coinType: coinTransfer?.coinType,
          fromAddress: coinTransfer?.sender,
          network: activeNetwork.nodeUrl,
          txnHash: txn?.hash,
        };

        trackEvent({ eventType, params });

        if (options?.onSettled && txn) {
          options.onSettled(txn, error, ...rest);
        }
      },
    },
  );
}
