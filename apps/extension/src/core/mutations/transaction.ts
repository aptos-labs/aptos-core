// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  AptosClient,
  MaybeHexString,
  RequestError,
} from 'aptos';
import { toast } from 'core/components/Toast';
import { useWalletState } from 'core/hooks/useWalletState';
import { useSequenceNumber } from 'core/queries/account';
import queryKeys from 'core/queries/queryKeys';
import Analytics from 'core/utils/analytics/analytics';
import { coinEvents } from 'core/utils/analytics/events';
import { useMutation, useQuery, useQueryClient } from 'react-query';
import { AptosError, ScriptFunctionPayload, UserTransaction } from 'aptos/dist/api/data-contracts';
import { useChainId } from 'core/queries/network';
import { MoveExecutionStatus, parseMoveVmStatus } from 'core/utils/move';
import {
  buildCoinTransferPayload,
  buildAccountTransferPayload,
  createRawTransaction,
} from 'core/utils/transaction';

export interface SubmitCoinTransferParams {
  amount: number,
  create: boolean,
  recipient: MaybeHexString,
}

/**
 * Get a raw coin transfer transaction factory for the current account
 */
function useCreateCoinTransferTransaction() {
  const { aptosAccount } = useWalletState();
  const { data: chainId } = useChainId();
  const { get: getSequenceNumber } = useSequenceNumber();

  const sender = aptosAccount?.address();
  const isReady = sender && chainId !== undefined;

  return isReady
    ? async ({ amount, create, recipient }: SubmitCoinTransferParams) => {
      const payload = create
        ? buildAccountTransferPayload(recipient, BigInt(amount))
        : buildCoinTransferPayload(recipient, BigInt(amount));
      return createRawTransaction(payload, {
        chainId,
        sender,
        sequenceNumber: await getSequenceNumber(),
      });
    }
    : undefined;
}

export interface UseCoinTransferParams {
  amount?: number,
  create?: boolean,
  enabled?: boolean,
  recipient?: string,
}

/**
 * Query a coin transfer simulation for the specified recipient and amount
 */
export function useCoinTransferSimulation({
  amount,
  create,
  enabled,
  recipient,
} : UseCoinTransferParams) {
  const { aptosAccount, nodeUrl } = useWalletState();
  const { refetch: refetchSeqNumber } = useSequenceNumber();
  const createTxn = useCreateCoinTransferTransaction();

  const isReady = Boolean(aptosAccount && createTxn);
  const isInputValid = Boolean(amount && create !== undefined && recipient);

  return useQuery(
    [queryKeys.getCoinTransferSimulation, recipient, amount],
    async () => {
      const rawTxn = await createTxn!({
        amount: amount!,
        create: create!,
        recipient: recipient!,
      });

      const aptosClient = new AptosClient(nodeUrl);
      const simulatedTxn = AptosClient.generateBCSSimulation(aptosAccount!, rawTxn);
      const userTxn = (await aptosClient.submitBCSSimulation(simulatedTxn)) as UserTransaction;
      if (!userTxn.success) {
        // Miscellaneous error is probably associated with invalid sequence number
        if (parseMoveVmStatus(userTxn.vm_status) === MoveExecutionStatus.MiscellaneousError) {
          await refetchSeqNumber();
          throw new Error(userTxn.vm_status);
        }
      }
      return userTxn;
    },
    {
      cacheTime: 0,
      enabled: isReady && enabled && isInputValid,
      keepPreviousData: true,
      refetchInterval: 5000,
      retry: 1,
    },
  );
}

/**
 * Mutation for submitting a coin transfer transaction
 */
export function useCoinTransferTransaction() {
  const { aptosAccount, nodeUrl } = useWalletState();
  const {
    increment: incrementSeqNumber,
    refetch: refetchSeqNumber,
  } = useSequenceNumber();
  const createTxn = useCreateCoinTransferTransaction();
  const queryClient = useQueryClient();

  const isReady = Boolean(aptosAccount && createTxn);

  const submitCoinTransferTransaction = async ({
    amount,
    create,
    recipient,
  }: SubmitCoinTransferParams) => {
    const rawTxn = await createTxn!({ amount, create, recipient });
    const aptosClient = new AptosClient(nodeUrl);
    const signedTxn = AptosClient.generateBCSTransaction(aptosAccount!, rawTxn);

    try {
      const { hash } = await aptosClient.submitSignedBCSTransaction(signedTxn);
      await aptosClient.waitForTransaction(hash);
      return (await aptosClient.getTransaction(hash)) as UserTransaction;
    } catch (err) {
      if (err instanceof RequestError) {
        const errorMsg = (err.response?.data as AptosError)?.message;
        if (errorMsg && errorMsg.indexOf('SEQUENCE_NUMBER_TOO_OLD') >= 0) {
          await refetchSeqNumber();
        }
      }
      throw err;
    }
  };

  const mutation = useMutation(submitCoinTransferTransaction, {
    onSuccess: async (txn: UserTransaction, { amount }: SubmitCoinTransferParams) => {
      // Optimistic update of sequence number
      incrementSeqNumber();
      queryClient.invalidateQueries(queryKeys.getAccountCoinBalance);

      const eventType = txn.success
        ? coinEvents.TRANSFER_APTOS_COIN
        : coinEvents.ERROR_TRANSFER_APTOS_COIN;

      const payload = txn.payload as ScriptFunctionPayload;
      const coinType = payload.type_arguments[0];

      const params = {
        amount,
        coinType,
        fromAddress: txn.sender,
        network: nodeUrl,
        ...txn,
      };

      Analytics.event({ eventType, params });

      toast({
        description: (txn.success)
          ? `Amount transferred: ${amount}, gas consumed: ${txn.gas_used}`
          : `Transfer failed, gas consumed: ${txn.gas_used}`,
        status: txn.success ? 'success' : 'error',
        title: `Transaction ${txn.success ? 'success' : 'error'}`,
      });
    },
    retry: 1,
  });

  return { isReady, ...mutation };
}

export const TransferResult = Object.freeze({
  AmountOverLimit: 'Amount is over limit',
  AmountWithGasOverLimit: 'Amount with gas is over limit',
  IncorrectPayload: 'Incorrect transaction payload',
  Success: 'Transaction executed successfully',
  UndefinedAccount: 'Account does not exist',
} as const);
