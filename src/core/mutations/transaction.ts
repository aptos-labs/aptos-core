// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useToast } from '@chakra-ui/react';
import {
  AptosAccount, AptosClient, MaybeHexString, Types,
} from 'aptos';
import useWalletState from 'core/hooks/useWalletState';
import {
  type GetTestCoinTokenBalanceFromAccountResourcesProps,
} from 'core/queries/account';
import queryKeys from 'core/queries/queryKeys';
import { getUserTransaction } from 'core/queries/transaction';
import { useMutation, useQueryClient } from 'react-query';

export interface SubmitTransactionProps {
  fromAccount: AptosAccount;
  nodeUrl: string;
  payload: Types.TransactionPayload,
}

export const submitTransaction = async ({
  fromAccount,
  nodeUrl,
  payload,
}: SubmitTransactionProps) => {
  const client = new AptosClient(nodeUrl);
  const txnRequest = await client.generateTransaction(fromAccount.address(), payload);
  const signedTxn = await client.signTransaction(fromAccount, txnRequest);
  const transactionRes = await client.submitTransaction(signedTxn);
  await client.waitForTransaction(transactionRes.hash);
  return transactionRes.hash;
};

export interface TestCoinTransferTransactionPayload {
  amount: string | number;
  toAddress: MaybeHexString;
}

export type SendTestCoinTransactionProps = Omit<SubmitTransactionProps & TestCoinTransferTransactionPayload, 'payload'>;

export const sendTestCoinTransaction = async ({
  amount,
  fromAccount,
  nodeUrl,
  toAddress,
}: SendTestCoinTransactionProps) => {
  const payload: Types.TransactionPayload = {
    arguments: [toAddress, `${amount}`],
    function: '0x1::Coin::transfer',
    type: 'script_function_payload',
    type_arguments: ['0x1::TestCoin::TestCoin'],
  };
  const txnHash = await submitTransaction({
    fromAccount,
    nodeUrl,
    payload,
  });
  return txnHash;
};

export const TransferResult = Object.freeze({
  AmountOverLimit: 'Amount is over limit',
  AmountWithGasOverLimit: 'Amount with gas is over limit',
  IncorrectPayload: 'Incorrect transaction payload',
  Success: 'Transaction executed successfully',
  UndefinedAccount: 'Account does not exist',
} as const);

export type SubmitTestCoinTransferTransactionProps = Omit<
TestCoinTransferTransactionPayload &
SendTestCoinTransactionProps &
GetTestCoinTokenBalanceFromAccountResourcesProps & {
  onClose: () => void
},
'accountResources'
>;

export const submitTestCoinTransferTransaction = async ({
  amount,
  fromAccount,
  nodeUrl,
  onClose,
  toAddress,
}: SubmitTestCoinTransferTransactionProps) => {
  const txnHash = await sendTestCoinTransaction({
    amount,
    fromAccount,
    nodeUrl,
    toAddress,
  });
  onClose();
  return txnHash;
};

export const useSubmitTestCoinTransfer = () => {
  const { aptosNetwork } = useWalletState();
  const queryClient = useQueryClient();
  const toast = useToast();

  return useMutation(submitTestCoinTransferTransaction, {
    onSettled: async (txnHash) => {
      if (!txnHash) {
        return;
      }
      queryClient.invalidateQueries(queryKeys.getAccountResources);
      const txn = await getUserTransaction({ nodeUrl: aptosNetwork, txnHashOrVersion: txnHash });
      const amount = (txn?.payload)
        ? (txn.payload as { arguments: string[] }).arguments[1]
        : undefined;
      toast({
        description: (txn?.success) ? `Amount transferred: ${amount}, gas consumed: ${txn?.gas_used}` : `Transfer failed, gas consumed: ${txn?.gas_used}`,
        duration: 5000,
        isClosable: true,
        status: (txn?.success) ? 'success' : 'error',
        title: `Transaction ${txn?.success ? 'success' : 'error'}`,
        variant: 'solid',
      });
    },
  });
};
